use paddle::*;
use progress::{ProgressReset, RenderProgress};
use render::{RenderSettings, RenderTask};
use wasm_bindgen::prelude::wasm_bindgen;
use worker_view::WorkerView;

mod progress;
mod render;
mod worker;
mod worker_view;

const SCREEN_W: u32 = 1080;
const SCREEN_H: u32 = 1080;

#[wasm_bindgen]
pub fn start() {
    // Build configuration object to define all setting
    let config = PaddleConfig::default()
        .with_canvas_id("paddle-canvas-id")
        .with_resolution((SCREEN_W, SCREEN_H))
        .with_text_board(Rectangle::new((100, 100), (500, 500)))
        .with_texture_config(TextureConfig::default().without_filter());

    // Initialize framework state and connect to browser window
    paddle::init(config).expect("Paddle initialization failed.");
    let state = Main::init();

    let main_handle = paddle::register_frame(state, (), (40, 0));
    main_handle.register_receiver(&Main::new_png_part);
    main_handle.register_receiver(&Main::ping_next_job);

    let progress_handle = paddle::register_frame_no_state(RenderProgress::new(), (40, 740));
    progress_handle.register_receiver(&RenderProgress::progress_reset);
    progress_handle.register_receiver(&RenderProgress::progress_update);

    let worker_handle =
        paddle::register_frame_no_state(WorkerView::new(), (40 + RenderProgress::WIDTH + 5, 740));
    worker_handle.register_receiver(&WorkerView::worker_ready);
    worker_handle.register_receiver(&WorkerView::new_jobs);
    worker_handle.register_receiver(&WorkerView::job_done);
    worker_handle.listen(&WorkerView::add_worker);
}

struct Main {
    /// Image is rendered in increasing quality each time triggered.
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
    const WIDTH: u32 = SCREEN_W;
    const HEIGHT: u32 = 720;

    fn draw(&mut self, _state: &mut Self::State, canvas: &mut DisplayArea, _timestamp: f64) {
        canvas.fit_display(5.0);
        for (img, area) in &self.imgs {
            canvas.draw(area, img);
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

struct PngPart {
    img: ImageDesc,
    screen_area: Rectangle,
}

impl Main {
    fn init() -> Self {
        Main {
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
                let jobs = job.divide(num_new_jobs as u32);
                paddle::send::<_, WorkerView>(jobs);
                paddle::send::<_, RenderProgress>(ProgressReset {
                    work_items: num_new_jobs,
                });
                self.next_quality += 1;
            }
        }
    }

    /// paddle event listener
    fn new_png_part(&mut self, _state: &mut (), msg: PngPart) {
        let mut bundle = AssetBundle::new();
        bundle.add_images(&[msg.img]);
        bundle.load();

        self.imgs.push((msg.img, msg.screen_area));
        self.outstanding_jobs = self.outstanding_jobs.saturating_sub(1);
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
