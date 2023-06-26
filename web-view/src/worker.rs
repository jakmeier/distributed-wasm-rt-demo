use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::MessageEvent;

use crate::render::RenderTask;
use crate::RawPngPart;

pub(crate) struct WorkerReady(pub usize);

pub(crate) struct PngRenderWorker {
    current_job: Option<RenderTask>,
    ready: bool,
    ctx: Box<dyn TaskRenderer>,
}
pub(crate) trait TaskRenderer {
    /// Enqueues a new task that will be executed eventually.
    ///
    /// When the task finishes, it will send a `RawPngPart`.
    fn submit(&self, task: &RenderTask);
}

pub(crate) struct LocalWorkerContext {
    worker: web_sys::Worker,
    _worker_rx: Closure<dyn FnMut(MessageEvent)>,
}

// pub(crate) struct RemoteWorker {
//     current_job: Option<RenderTask>,
//     ready: bool,
// }

impl TaskRenderer for LocalWorkerContext {
    fn submit(&self, task: &RenderTask) {
        task.submit_to_local_worker(&self.worker);
    }
}

impl PngRenderWorker {
    pub fn new(worker_id: usize, remote_url: Option<String>) -> Self {
        let ctx = if let Some(url) = remote_url {
            todo!()
        } else {
            LocalWorkerContext::new(worker_id)
        };
        PngRenderWorker {
            current_job: None,
            ready: false,
            ctx: Box::new(ctx),
        }
    }

    pub fn accept_task(&mut self, task: RenderTask) {
        assert!(self.current_job.is_none());
        self.ctx.submit(&task);
        self.current_job = Some(task);
    }

    pub fn current_task(&self) -> &Option<RenderTask> {
        &self.current_job
    }

    pub fn clear_task(&mut self) -> Option<RenderTask> {
        self.current_job.take()
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn set_ready(&mut self, yes: bool) {
        self.ready = yes;
    }
}

impl LocalWorkerContext {
    fn new(worker_id: usize) -> Self {
        let worker = web_sys::Worker::new("./worker.js").expect("Failed to create worker");

        let rx = move |evt: MessageEvent| {
            if let Ok(array) = evt.data().dyn_into::<js_sys::Uint8Array>() {
                let array = js_sys::Array::from(&array);
                let raw: Vec<u8> = (0..array.length())
                    .map(|i| array.get(i).as_f64().unwrap_or(0.0) as u8)
                    .collect();

                paddle::send::<_, crate::Main>(RawPngPart {
                    data: raw,
                    worker_id,
                })
            } else if let Some(s) = evt.data().as_string() {
                match s.as_str() {
                    "ready" => paddle::send::<_, crate::Main>(WorkerReady(worker_id)),
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
