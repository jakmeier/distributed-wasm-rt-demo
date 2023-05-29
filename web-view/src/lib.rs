use paddle::*;
use render::{RenderSettings, RenderTask};
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
    current_quality: u32,

    /// Stack of all images rendered, drawn in the order they were added and
    /// potentially covering older images.
    imgs: Vec<(ImageDesc, Rectangle)>,

    /// number of jobs currently waiting to be done
    outstanding_jobs: usize,
    /// number of images left from the previous job
    old_images: usize,
}

impl paddle::Frame for Main {
    type State = ();
    const WIDTH: u32 = SCREEN_W;
    const HEIGHT: u32 = SCREEN_H;

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

    fn key(&mut self, _state: &mut Self::State, key: KeyEvent) {
        if key.event_type() == KeyEventType::KeyPress {
            match key.key() {
                Key::Space => {
                    self.ping_next_job();
                }
                _ => {}
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
        let quality = 0;
        let job = Self::new_full_screen_job(quality).unwrap();
        let num_starter_jobs = 64;

        Main {
            workers: vec![],
            worker_num: 10,
            job_pool: job.divide(num_starter_jobs),
            current_quality: quality,
            imgs: vec![],
            old_images: 0,
            outstanding_jobs: num_starter_jobs as usize,
        }
    }

    /// Starts new job if the old isn no longer running
    fn ping_next_job(&mut self) {
        if self.outstanding_jobs == 0 {
            self.current_quality += 1;
            if self.old_images > 0 {
                self.imgs.drain(..self.old_images);
            }
            let num_new_jobs = match self.current_quality {
                0..=2 => 64,
                3..=4 => 256,
                _ => 512,
            };

            self.old_images = self.imgs.len();
            self.outstanding_jobs = num_new_jobs;
            if let Some(job) = Self::new_full_screen_job(self.current_quality) {
                self.job_pool = job.divide(num_new_jobs as u32);
            }
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
        self.imgs.push((img_desc, job.screen_area));
        self.outstanding_jobs -= 1;
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

    fn new_full_screen_job(quality: u32) -> Option<RenderTask> {
        let screen = Rectangle::new_sized((SCREEN_W, SCREEN_H));
        let mut settings = RenderSettings {
            resolution: (screen.width() as u32, screen.height() as u32),
            samples: 512,
            recursion: 200,
        };
        match quality {
            0 => {
                settings.resolution.0 = settings.resolution.0 / 4;
                settings.resolution.1 = settings.resolution.1 / 4;
                settings.samples = 1;
                settings.recursion = 2;
            }
            1 => {
                settings.resolution.0 = settings.resolution.0 / 2;
                settings.resolution.1 = settings.resolution.1 / 2;
                settings.samples = 1;
                settings.recursion = 4;
            }
            2 => {
                settings.samples = 4;
                settings.recursion = 4;
            }
            3 => {
                settings.samples = 4;
                settings.recursion = 100;
            }
            4 => {
                settings.samples = 64;
                settings.recursion = 100;
            }
            5 => {
                // return max settings
            }
            _more => return None,
        }
        Some(RenderTask::new(screen, settings))
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
