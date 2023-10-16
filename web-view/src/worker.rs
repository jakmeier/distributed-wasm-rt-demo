use js_sys::Uint32Array;
use paddle::quicksilver_compat::{Color, Shape};
use paddle::{FloatingText, ImageDesc, Rectangle, TextBoard, Transform};
use std::cell::RefCell;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::MessageEvent;

use crate::render::RenderTask;
use crate::workers_view::WorkerView;
use crate::ImageData;

/// Worker has completed initialization.
pub(crate) struct WorkerReady(pub usize);

/// Worker has completed as task
pub(crate) struct WorkerResult {
    pub worker_id: usize,
    pub img: ImageData,
}

pub(crate) struct PngRenderWorker {
    current_job: Option<RenderTask>,
    ready: bool,
    /// Marks an in-flight job while rendering was stopped. Reset when job finishes.
    interrupted: bool,
    ctx: Box<dyn TaskRenderer>,
    displayable: Box<dyn paddle::DisplayPaint>,
    start: chrono::NaiveDateTime,
    prev_time: RefCell<FloatingText>,
    ready_img: ImageDesc,
    loading_img: ImageDesc,
}
pub(crate) trait TaskRenderer {
    /// Enqueues a new task that will be executed eventually.
    ///
    /// When the task finishes, it will send a `WorkerResult`.
    fn submit(&self, task: &RenderTask);
}

pub(crate) struct LocalWorkerContext {
    worker: web_sys::Worker,
    _worker_rx: Closure<dyn FnMut(MessageEvent)>,
}

pub(crate) struct RemoteWorkerContext {
    url: String,
    worker_id: usize,
}

impl TaskRenderer for LocalWorkerContext {
    fn submit(&self, task: &RenderTask) {
        let vec = task.marshal().to_vec();
        let array = Uint32Array::new_with_length(vec.len() as u32);
        array.copy_from(&vec);

        self.worker
            .post_message(&array)
            .expect("Failed posting job to worker");
    }
}

impl TaskRenderer for RemoteWorkerContext {
    fn submit(&self, task: &RenderTask) {
        // Reading from this URL causes work on the remote, so let's not use it
        // directly. Instead, download the data and build a local url object.
        let full_url = format!("{}/{}", self.url, task.marshal());
        let worker_id = self.worker_id;
        let future = async move {
            let binary = paddle::load_file(&full_url).await.unwrap();
            paddle::send::<_, WorkerView>(WorkerResult {
                img: ImageData::new_from_vec(binary),
                worker_id,
            });
        };
        wasm_bindgen_futures::spawn_local(future);
    }
}

impl RemoteWorkerContext {
    pub(crate) fn new(url: String, worker_id: usize) -> Self {
        // construct this outside to make future independent of `url` lifetime
        let ping_url = format!("{url}/ping");
        let future = async move {
            match paddle::fetch::load_file(&ping_url).await {
                Ok(s) if s == b"pong" => paddle::send::<_, WorkerView>(WorkerReady(worker_id)),
                Ok(s) => {
                    TextBoard::display_error_message("Unexpected remote worker response.".into())
                        .unwrap();
                    if let Ok(msg) = std::str::from_utf8(&s) {
                        let error_msg = format!("Message was: {msg}.");
                        paddle::println!("{}", error_msg);
                        TextBoard::display_error_message(error_msg).unwrap();
                    }
                }
                Err(_) => {
                    TextBoard::display_error_message(format!(
                        "Connecting worker {worker_id} failed."
                    ))
                    .unwrap();
                }
            }
        };
        wasm_bindgen_futures::spawn_local(future);
        Self { url, worker_id }
    }
}

impl PngRenderWorker {
    pub fn new(
        worker_id: usize,
        remote_url: Option<String>,
        displayable: Box<dyn paddle::DisplayPaint>,
        loading_img: ImageDesc,
        ready_img: ImageDesc,
    ) -> Self {
        let ctx: Box<dyn TaskRenderer> = if let Some(url) = remote_url {
            Box::new(RemoteWorkerContext::new(url, worker_id))
        } else {
            Box::new(LocalWorkerContext::new(worker_id))
        };
        let mut text = FloatingText::new(&Rectangle::default(), String::default()).unwrap();
        text.update_fit_strategy(paddle::FitStrategy::Center)
            .unwrap();
        PngRenderWorker {
            current_job: None,
            ready: false,
            ctx,
            start: paddle::utc_now(),
            displayable,
            prev_time: RefCell::new(text),
            interrupted: false,
            loading_img,
            ready_img,
        }
    }

    pub fn accept_task(&mut self, task: RenderTask) {
        assert!(self.current_job.is_none());
        self.start = paddle::utc_now();
        self.ctx.submit(&task);
        self.current_job = Some(task);
    }

    pub fn current_task(&self) -> &Option<RenderTask> {
        &self.current_job
    }

    pub fn clear_task(&mut self) -> Option<(RenderTask, std::time::Duration)> {
        self.current_job.take().map(|job| {
            (
                job,
                paddle::utc_now()
                    .signed_duration_since(self.start)
                    .to_std()
                    .unwrap(),
            )
        })
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn set_ready(&mut self, yes: bool) {
        self.ready = yes;
    }

    pub fn record_time(&mut self, duration: std::time::Duration) {
        self.prev_time
            .get_mut()
            .update_text(&format!("{:#.1?}", duration));
    }

    pub fn clear(&mut self) {
        if !self.interrupted {
            self.clear_task();
            self.prev_time.get_mut().update_text("...");
        }
    }

    pub fn interrupt(&mut self) {
        if self.current_job.is_some() {
            self.interrupted = true;
        }
    }

    pub fn clear_interrupt(&mut self) -> bool {
        std::mem::take(&mut self.interrupted)
    }

    /// Display self in the specified area.
    pub fn draw(&self, canvas: &mut paddle::DisplayArea, area: Rectangle, timestamp: f64) {
        if self.current_job.is_some() {
            canvas.draw(&area, &Color::WHITE);
            let trans = Transform::translate(area.center())
                * Transform::rotate(timestamp / 10.0)
                * Transform::translate(-area.center());
            canvas.draw_ex(&area.shrink_to_center(0.8), &self.loading_img, trans, 1);
        } else {
            canvas.draw_ex(
                &area.shrink_to_center(0.8),
                &self.ready_img,
                Transform::default(),
                1,
            );
        }
        canvas.draw_ex(&area.padded(2.0), &self.displayable, Transform::IDENTITY, 0);
        // self.prev_time
        //     .borrow_mut()
        //     .update_position(&canvas.frame_to_display_area(area), 0)
        //     .unwrap();
        // self.prev_time.borrow_mut().draw();
    }

    pub fn active(&self) {
        self.prev_time.borrow().show().unwrap();
    }

    pub fn inactive(&self) {
        self.prev_time.borrow().hide().unwrap();
    }
}

impl LocalWorkerContext {
    fn new(worker_id: usize) -> Self {
        let worker = web_sys::Worker::new("./worker.js").expect("Failed to create worker");

        let rx = move |evt: MessageEvent| {
            if let Ok(array) = evt.data().dyn_into::<js_sys::Uint8Array>() {
                paddle::send::<_, WorkerView>(WorkerResult {
                    img: ImageData::new_from_array(array),
                    worker_id,
                });
            } else if let Some(s) = evt.data().as_string() {
                match s.as_str() {
                    "ready" => paddle::send::<_, WorkerView>(WorkerReady(worker_id)),
                    _ => {}
                }
            } else {
                paddle::println!("Unexpected message type!");
            }
        };
        let _worker_rx = Closure::wrap(Box::new(rx) as Box<dyn FnMut(MessageEvent)>);
        worker.set_onmessage(Some(_worker_rx.as_ref().dyn_ref().unwrap()));
        LocalWorkerContext { worker, _worker_rx }
    }
}
