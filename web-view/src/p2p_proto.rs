//! Ad-hoc protocol for communication between peers.
//!
//! This is the "application layer" protocol, building on top of a WebRTC data
//! channel.
//!
//! The receiving side makes use of the Blob API for reconstructing images from
//! incoming data, instead of reconstructing everything from an array buffer.
//! Thus, it is not a pure "enum + serde" implementation as in other places.

use std::io::Write;

use api::RenderJob;
use js_sys::{ArrayBuffer, Uint8Array};
use paddle::Rectangle;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Blob;

use crate::render::RenderTask;
use crate::{ImageData, PngPart};

/// Parsed message sent between peers over WebRTC data channels.
///
/// This is the fully parsed representation used outside this module.
/// The wire format is `MessageHeader` + [fields] + [blob].
pub(crate) enum Message {
    // RequestJobs(N)
    // Jobs(N)
    RenderedPart(PngPart),
    StealWork(StealWorkBody),
    Job(JobBody),
    RenderControl(RenderControlBody),
}
/// Message header sent between peers over WebRTC data channels.
///
/// This is only the header, which may be followed by fields and maybe a Blob.
/// There is no unified body type, depending on the header there will be a
/// different body.
#[repr(u8)]
#[derive(Clone, Copy)]
enum MessageHeader {
    /// A rendered output.
    RenderedPart = 1,
    /// Request for work, as the workers managed on this instance are idle.
    StealWork = 2,
    /// Response to `StealWork`, a list of jobs that can be done by the work stealer.
    Job = 3,
    /// Start or stop rendering.
    RenderControl = 4,
}

struct RenderedPartBody {
    x: u32,
    y: u32,
    pixel_width: u32,
    pixel_height: u32,
    bytes: u32,
}

pub(crate) struct StealWorkBody {
    pub num_jobs: u32,
}

pub(crate) struct JobBody {
    pub jobs: Vec<RenderTask>,
}

pub(crate) struct RenderControlBody {
    pub num_new_jobs: u32,
}

impl Message {
    pub(crate) async fn serialize(&self, w: &mut impl Write) -> std::io::Result<()> {
        w.write(&[self.header() as u8])?;
        match self {
            Message::RenderedPart(part) => {
                if part.img.data.borrow().is_none() {
                    let result = paddle::load_file(part.img.img.url().unwrap()).await;

                    let bytes = result.map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("loading image data failed: {e:?}"),
                        )
                    })?;
                    *part.img.data.borrow_mut() = Some(bytes);
                }
                let borrow = &part.img.data.borrow();
                let blob_bytes = borrow.as_ref().expect("just filled blob");
                let body = RenderedPartBody {
                    x: part.screen_area.pos.x as u32,
                    y: part.screen_area.pos.y as u32,
                    pixel_width: part.screen_area.width() as u32,
                    pixel_height: part.screen_area.height() as u32,
                    bytes: blob_bytes.len() as u32,
                };
                body.serialize(w)?;
                w.write(&blob_bytes)?;
            }
            Message::StealWork(body) => body.serialize(w)?,
            Message::Job(body) => body.serialize(w)?,
            Message::RenderControl(body) => body.serialize(w)?,
        }
        Ok(())
    }

    pub(crate) async fn from_blob(blob: web_sys::Blob) -> Result<Self, wasm_bindgen::JsValue> {
        let first_byte_blob = blob.slice_with_i32_and_i32(0, 1)?;
        let first_byte = blob_to_array(&first_byte_blob).await?;
        let header: Option<MessageHeader> = first_byte.get_index(0).try_into().ok();
        match header {
            Some(MessageHeader::RenderedPart) => {
                let fields_len = 5 * std::mem::size_of::<u32>();
                let fields_bytes = blob_slice(&blob, 1, fields_len).await?;
                let body = RenderedPartBody::deserialize(&fields_bytes);
                let png_blob = blob.slice_with_i32(1 + fields_len as i32)?;
                // TODO: unregister object?
                let url = web_sys::Url::create_object_url_with_blob(&png_blob)?;
                let png = PngPart {
                    img: ImageData::new_leaky(url),
                    screen_area: Rectangle::new(
                        (body.x, body.y),
                        (body.pixel_width, body.pixel_height),
                    ),
                };
                Ok(Message::RenderedPart(png))
            }
            Some(MessageHeader::StealWork) => {
                let fields_len = std::mem::size_of::<u32>();
                let fields_bytes = blob_slice(&blob, 1, fields_len).await?;
                let body = StealWorkBody::deserialize(&fields_bytes);
                Ok(Message::StealWork(body))
            }
            Some(MessageHeader::Job) => {
                let body_blob = blob.slice_with_i32(1)?;
                let body_bytes = blob_to_array(&body_blob).await?;
                let body = JobBody::deserialize(&body_bytes.to_vec());
                Ok(Message::Job(body))
            }
            Some(MessageHeader::RenderControl) => {
                let body_blob = blob.slice_with_i32(1)?;
                let body_bytes = blob_to_array(&body_blob).await?;
                let body = RenderControlBody::deserialize(&body_bytes.to_vec());
                Ok(Message::RenderControl(body))
            }
            None => Err(format!(
                "Unexpected message, starting with byte {} and a total length of {}.",
                first_byte.get_index(0),
                blob.size()
            )
            .into()),
        }
    }

    fn header(&self) -> MessageHeader {
        match self {
            Message::RenderedPart(_) => MessageHeader::RenderedPart,
            Message::StealWork(_) => MessageHeader::StealWork,
            Message::Job(_) => MessageHeader::Job,
            Message::RenderControl(_) => MessageHeader::RenderControl,
        }
    }
}

async fn blob_slice(blob: &Blob, offset: usize, len: usize) -> Result<Vec<u8>, JsValue> {
    if (blob.size() as usize) < (offset + len) {
        return Err("not enough data".into());
    }
    let fields_blob = blob.slice_with_i32_and_i32(offset as i32, offset as i32 + len as i32)?;
    let fields_bytes = blob_to_array(&fields_blob).await?;
    Ok(fields_bytes.to_vec())
}

async fn blob_to_array(blob: &Blob) -> Result<Uint8Array, JsValue> {
    let promise = blob.array_buffer();
    let result = JsFuture::from(promise).await?;
    let array_buffer = result.dyn_into::<ArrayBuffer>()?;
    Ok(Uint8Array::new(&array_buffer))
}

impl TryFrom<u8> for MessageHeader {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let result = match value {
            1 => Self::RenderedPart,
            2 => Self::StealWork,
            3 => Self::Job,
            4 => Self::RenderControl,
            _ => return Err(()),
        };
        assert_eq!(result as u8, value);
        Ok(result)
    }
}

impl RenderedPartBody {
    fn serialize(&self, w: &mut impl Write) -> Result<(), std::io::Error> {
        w.write(&self.x.to_be_bytes())?;
        w.write(&self.y.to_be_bytes())?;
        w.write(&self.pixel_width.to_be_bytes())?;
        w.write(&self.pixel_height.to_be_bytes())?;
        w.write(&self.bytes.to_be_bytes())?;
        Ok(())
    }

    fn deserialize(data: &[u8]) -> RenderedPartBody {
        assert_eq!(data.len(), 20, "RenderedPartBody must be 20 bytes");
        RenderedPartBody {
            x: u32::from_be_bytes(data[0..4].try_into().unwrap()),
            y: u32::from_be_bytes(data[4..8].try_into().unwrap()),
            pixel_width: u32::from_be_bytes(data[8..12].try_into().unwrap()),
            pixel_height: u32::from_be_bytes(data[12..16].try_into().unwrap()),
            bytes: u32::from_be_bytes(data[16..20].try_into().unwrap()),
        }
    }
}

impl StealWorkBody {
    fn serialize(&self, w: &mut impl Write) -> Result<(), std::io::Error> {
        w.write(&self.num_jobs.to_be_bytes())?;
        Ok(())
    }

    fn deserialize(data: &[u8]) -> Self {
        assert_eq!(data.len(), 4, "StealWorkBody must be 4 bytes");
        Self {
            num_jobs: u32::from_be_bytes(data[0..4].try_into().unwrap()),
        }
    }
}

impl JobBody {
    fn serialize(&self, w: &mut impl Write) -> Result<(), std::io::Error> {
        let num_jobs = self.jobs.len() as u32;
        w.write(&num_jobs.to_be_bytes())?;
        for job in &self.jobs {
            let data: Vec<u8> = job
                .marshal()
                .to_vec()
                .iter()
                .flat_map(|num| num.to_be_bytes().into_iter())
                .collect();
            w.write(&data)?;
        }
        Ok(())
    }

    fn deserialize(data: &[u8]) -> Self {
        assert!(data.len() >= 4, "JobBody must be at least 4 bytes");
        let num_jobs = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;

        assert_eq!(
            num_jobs * 8 * 4,
            data.len() - 4,
            "JobBody must be 8 times a u32 per job"
        );
        let numbers = data[4..]
            .chunks_exact(4)
            .map(|slice| u32::from_be_bytes(slice.try_into().expect("window size must be exact")))
            .collect::<Vec<_>>();
        let jobs = numbers
            .chunks_exact(8)
            .map(|slice| RenderTask::from(RenderJob::try_from_slice(slice).unwrap()))
            .collect();
        Self { jobs }
    }
}

impl RenderControlBody {
    fn serialize(&self, w: &mut impl Write) -> Result<(), std::io::Error> {
        w.write(&self.num_new_jobs.to_be_bytes())?;
        Ok(())
    }

    fn deserialize(data: &[u8]) -> Self {
        assert!(
            data.len() >= 4,
            "RenderControlBody must be at least 4 bytes"
        );
        let num_new_jobs = u32::from_be_bytes(data[0..4].try_into().unwrap());
        Self { num_new_jobs }
    }
}
