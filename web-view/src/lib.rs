use paddle::*;
use render::RenderTask;
use wasm_bindgen::prelude::{wasm_bindgen, Closure};
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, Worker};

mod render;

const SCREEN_W: u32 = 960;
const SCREEN_H: u32 = 720;

#[wasm_bindgen]
pub fn start() {
    // Build configuration object to define all setting
    let config = PaddleConfig::default()
        .with_canvas_id("paddle-canvas-id")
        .with_resolution((SCREEN_W, SCREEN_H))
        .with_texture_config(TextureConfig::default().without_filter());

    // Initialize framework state and connect to browser window
    paddle::init(config).expect("Paddle initialization failed.");
    let state = Main::init();
    let handle = paddle::register_frame(state, (), (0, 0));
    handle.register_receiver(&Main::new_png_part);
    handle.register_receiver(&Main::worker_ready);
}

struct Main {
    workers: Vec<PngRenderWorker>,
    /// target number of workers
    worker_num: usize,
    job_pool: Vec<RenderTask>,

    imgs: Vec<(ImageDesc, Rectangle)>,
}

impl paddle::Frame for Main {
    type State = ();
    const WIDTH: u32 = 480;
    const HEIGHT: u32 = 360;

    fn draw(&mut self, _state: &mut Self::State, canvas: &mut DisplayArea, _timestamp: f64) {
        canvas.fit_display(5.0);
        for (img, area) in &self.imgs {
            canvas.draw(area, img);
        }
    }

    fn update(&mut self, _state: &mut Self::State) {
        // while self.workers.len() > self.worker_num {
        //     let worker = self.workers.pop().unwrap();
        //     TODO: proper cleanup
        //     worker.terminate();
        // }
        while self.workers.len() < self.worker_num {
            self.workers.push(self.new_worker(self.workers.len()));
        }

        for worker in &mut self.workers {
            if worker.ready && worker.current_job.is_none() {
                if let Some(job) = self.job_pool.pop() {
                    worker.accept_task(job);
                }
            }
        }
    }
}

struct RawPngPart {
    data: Vec<u8>,
    worker_id: usize,
}

impl Main {
    fn init() -> Self {
        let screen = Rectangle::new_sized((SCREEN_W, SCREEN_H));
        let job = RenderTask::new(screen);

        Main {
            // scene: IncrementalSceneRenderer::new(clumsy_rt::sample_scenes::build_cool_scene()),
            workers: vec![],
            worker_num: 10,
            job_pool: job.divide(64),
            imgs: vec![],
        }
    }

    /// paddle event listener
    fn new_png_part(&mut self, _state: &mut (), msg: RawPngPart) {
        let img_desc = ImageDesc::from_png_binary(&msg.data).unwrap();
        let mut bundle = AssetBundle::new();
        bundle.add_images(&[img_desc]);
        let _tracker = bundle.load();

        let job = self.workers[msg.worker_id]
            .current_job
            .take()
            .expect("response without job?");
        self.imgs.push((img_desc, job.area));
    }

    /// paddle event listener
    fn worker_ready(&mut self, _state: &mut (), WorkerReady(worker_id): WorkerReady) {
        self.workers[worker_id].ready = true;
    }

    fn new_worker(&self, worker_id: usize) -> PngRenderWorker {
        let worker = web_sys::Worker::new("./worker.js").expect("Failed to create worker");

        let rx = move |evt: MessageEvent| {
            if let Ok(array) = evt.data().dyn_into::<js_sys::Uint8Array>() {
                let array = js_sys::Array::from(&array);
                let raw: Vec<u8> = (0..array.length())
                    .map(|i| array.get(i).as_f64().unwrap_or(0.0) as u8)
                    .collect();

                paddle::send::<_, Main>(RawPngPart {
                    data: raw,
                    worker_id,
                })
            } else if let Some(s) = evt.data().as_string() {
                match s.as_str() {
                    "ready" => paddle::send::<_, Main>(WorkerReady(worker_id)),
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

struct WorkerReady(usize);

struct PngRenderWorker {
    worker: Worker,
    _worker_rx: Closure<dyn FnMut(MessageEvent)>,
    current_job: Option<RenderTask>,
    ready: bool,
}

impl PngRenderWorker {
    fn accept_task(&mut self, task: RenderTask) {
        assert!(self.current_job.is_none());
        task.submit_to_worker(&self.worker);
        self.current_job = Some(task);
    }
}
