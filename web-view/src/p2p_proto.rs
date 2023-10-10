//! Ad-hoc protocol for communication between peers.
//!
//! This is the "application layer" protocol, building on top of a WebRTC data
//! channel.
//!
//! The receiving side makes use of the Blob API for reconstructing images from
//! incoming data, instead of reconstructing everything from an array buffer.
//! Thus, it is not a pure "enum + serde" implementation as in other places.

use std::io::Write;

use js_sys::{ArrayBuffer, Uint8Array};
use paddle::Rectangle;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Blob;

use crate::{ImageData, PngPart};

/// Parsed message sent between peers over WebRTC data channels.
///
/// This is the fully parsed representation used outside this module.
/// The wire format is `MessageHeader` + [fields] + [blob].
pub(crate) enum Message {
    // RequestJobs(N)
    // Jobs(N)
    RenderedPart(PngPart),
}
/// Message header sent between peers over WebRTC data channels.
///
/// This is only the header, which may be followed by fields and maybe a Blob.
/// There is no unified body type, depending on the header there will be a
/// different body.
#[repr(u8)]
enum MessageHeader {
    RenderedPart = 1,
    // RequestJobs(N)
    // Jobs(N)
}

struct RenderedPartBody {
    x: u32,
    y: u32,
    pixel_width: u32,
    pixel_height: u32,
    bytes: u32,
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
                if (blob.size() as usize) < (1 + fields_len) {
                    return Err("not enough data".into());
                }
                let fields_blob = blob.slice_with_i32_and_i32(1, 1 + fields_len as i32)?;
                let fields_bytes = blob_to_array(&fields_blob).await?;
                let data = fields_bytes.to_vec();
                let body = RenderedPartBody::deserialize(&data);
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
        }
    }
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
        match value {
            1 => Ok(Self::RenderedPart),
            _ => Err(()),
        }
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
