use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::MessageEvent;

use crate::render::RenderTask;
use crate::RawPngPart;

pub(crate) struct WorkerReady(pub usize);

pub(crate) trait Worker {
    fn accept_task(&mut self, task: RenderTask);
    fn current_task(&self) -> &Option<RenderTask>;
    fn clear_task(&mut self) -> Option<RenderTask>;
    fn ready(&self) -> bool;
    fn set_ready(&mut self, yes: bool);
}

pub(crate) struct PngRenderWorker {
    worker: web_sys::Worker,
    _worker_rx: Closure<dyn FnMut(MessageEvent)>,
    current_job: Option<RenderTask>,
    ready: bool,
}

// pub(crate) struct RemoteWorker {
//     current_job: Option<RenderTask>,
//     ready: bool,
// }

impl Worker for PngRenderWorker {
    fn accept_task(&mut self, task: RenderTask) {
        assert!(self.current_job.is_none());
        task.submit_to_worker(&self.worker);
        self.current_job = Some(task);
    }

    fn current_task(&self) -> &Option<RenderTask> {
        &self.current_job
    }

    fn clear_task(&mut self) -> Option<RenderTask> {
        self.current_job.take()
    }

    fn ready(&self) -> bool {
        self.ready
    }

    fn set_ready(&mut self, yes: bool) {
        self.ready = yes;
    }
}

impl PngRenderWorker {
    pub fn new(worker_id: usize) -> Self {
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
        PngRenderWorker {
            worker,
            _worker_rx,
            current_job: None,
            ready: false,
        }
    }
}
