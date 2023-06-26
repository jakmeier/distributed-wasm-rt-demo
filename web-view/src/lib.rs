use paddle::*;
use progress::{ProgressMade, ProgressReset, RenderProgress};
use render::{RenderSettings, RenderTask};
use wasm_bindgen::prelude::wasm_bindgen;
use worker::PngRenderWorker;

mod progress;
mod render;
mod worker;

const SCREEN_W: u32 = 1080;
const SCREEN_H: u32 = 1080;

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
    let main_handle = paddle::register_frame(state, (), (40, 0));
    main_handle.register_receiver(&Main::new_png_part);
    main_handle.register_receiver(&Main::worker_ready);
    main_handle.register_receiver(&Main::ping_next_job);
    let progress_handle = paddle::register_frame_no_state(RenderProgress::new(), (40, 740));
    progress_handle.register_receiver(&RenderProgress::progress_reset);
    progress_handle.register_receiver(&RenderProgress::progress_update);
}

struct Main {
    workers: Vec<PngRenderWorker>,
    /// target number of workers
    worker_num: usize,
    job_pool: Vec<RenderTask>,
    next_quality: u32,

    /// Stack of all images rendered, drawn in the order they were added and
    /// potentially covering older images.
    imgs: Vec<(ImageDesc, Rectangle)>,

    /// number of jobs currently waiting to be done
    outstanding_jobs: usize,
    /// number of images left from the previous job
    old_images: usize,
}

struct RequestNewRender;

impl paddle::Frame for Main {
    type State = ();
    const WIDTH: u32 = 960;
    const HEIGHT: u32 = 720;

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
            self.workers
                .push(PngRenderWorker::new(self.workers.len(), None));
        }

        for worker in &mut self.workers {
            if worker.ready() && worker.current_task().is_none() {
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
                    self.ping_next_job(_state, RequestNewRender);
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
        Main {
            workers: vec![],
            worker_num: 10,
            job_pool: vec![],
            next_quality: 0,
            imgs: vec![],
            old_images: 0,
            outstanding_jobs: 0,
        }
    }

    /// Starts new job if the old is no longer running
    ///
    /// paddle event listener
    fn ping_next_job(&mut self, _: &mut (), _msg: RequestNewRender) {
        if self.outstanding_jobs == 0 {
            if self.old_images > 0 {
                self.imgs.drain(..self.old_images);
            }
            let num_new_jobs = match self.next_quality {
                0..=2 => 64,
                3..=4 => 256,
                _ => 512,
            };

            self.old_images = self.imgs.len();
            self.outstanding_jobs = num_new_jobs;
            if let Some(job) = Self::new_full_screen_job(self.next_quality) {
                self.job_pool = job.divide(num_new_jobs as u32);
                paddle::send::<_, RenderProgress>(ProgressReset {
                    work_items: num_new_jobs,
                });
                self.next_quality += 1;
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
            .clear_task()
            .take()
            .expect("response without job?");
        self.imgs.push((img_desc, job.screen_area));
        self.outstanding_jobs -= 1;
        paddle::send::<_, RenderProgress>(ProgressMade {});
    }

    /// paddle event listener
    fn worker_ready(
        &mut self,
        _state: &mut (),
        worker::WorkerReady(worker_id): worker::WorkerReady,
    ) {
        self.workers[worker_id].set_ready(true);
    }

    fn new_full_screen_job(quality: u32) -> Option<RenderTask> {
        let screen = Rectangle::new_sized((Main::WIDTH, Main::HEIGHT));
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
