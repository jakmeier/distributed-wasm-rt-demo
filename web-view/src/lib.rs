use std::cell::RefCell;
use std::rc::Rc;

use bottom_tabs::Tabs;
use js_sys::Uint8Array;
use network::NetworkView;
use paddle::quicksilver_compat::Color;
use paddle::*;
use progress::{ProgressReset, RenderProgress};
use render::{RenderSettings, RenderTask};
use wasm_bindgen::prelude::wasm_bindgen;
use workers_view::WorkerView;

mod bottom_tabs;
mod network;
mod p2p_proto;
mod progress;
mod render;
mod webrtc_signaling;
mod worker;
mod worker_node;
mod workers_view;
mod ws;

const SCREEN_W: u32 = 1620;
const SCREEN_H: u32 = 1620;

#[wasm_bindgen]
pub fn start() {
    // Build configuration object to define all setting
    let config = PaddleConfig::default()
        .with_canvas_id("paddle-canvas-id")
        .with_resolution((SCREEN_W, SCREEN_H + 150))
        .with_text_board(Rectangle::new((100, 100), (500, 500)))
        .with_texture_config(TextureConfig::default().without_filter());

    // Initialize framework state and connect to browser window
    paddle::init(config).expect("Paddle initialization failed.");

    let mut loader = AssetBundle::new();
    let fermyon_img = ImageDesc::from_path("assets/fermyon.png");
    loader.add_images(&[fermyon_img]);
    loader.load();

    let state = Main::init();
    let main_handle = paddle::register_frame(state, (), (40, 0));
    main_handle.register_receiver(&Main::ping_next_job);
    main_handle.listen(&Main::new_png_part);
    main_handle.listen(&Main::peer_message);
    main_handle.listen(&Main::stop);

    let lower_y = Main::HEIGHT + 5;
    let progress_handle = paddle::register_frame_no_state(RenderProgress::new(), (40, lower_y));
    progress_handle.register_receiver(&RenderProgress::progress_reset);
    progress_handle.register_receiver(&RenderProgress::progress_update);
    progress_handle.listen(&RenderProgress::stop);

    let worker_handle = paddle::register_frame_no_state(
        WorkerView::new(fermyon_img),
        (40 + RenderProgress::WIDTH + 5, lower_y),
    );
    worker_handle.register_receiver(&WorkerView::worker_ready);
    worker_handle.register_receiver(&WorkerView::new_jobs);
    worker_handle.register_receiver(&WorkerView::job_done);
    worker_handle.listen(&WorkerView::add_worker);
    worker_handle.listen(&WorkerView::stop);

    let network_handle = NetworkView::init();
    network_handle.listen(&NetworkView::new_png_part);
    let _tabs_handle = Tabs::init(main_handle, progress_handle, worker_handle, network_handle);

    paddle::share(workers_view::AddWorker::InBrowser);
    paddle::share(workers_view::AddWorker::InBrowser);
    paddle::share(workers_view::AddWorker::InBrowser);
    paddle::share(workers_view::AddWorker::InBrowser);
}

struct Main {
    /// Image is rendered in increasing quality each time triggered.
    next_quality: u32,

    /// Stack of all images rendered, drawn in the order they were added and
    /// potentially covering older images.
    imgs: Vec<PngPart>,

    /// number of jobs currently waiting to be done
    outstanding_jobs: usize,
    /// number of images left from the previous job
    old_images: usize,
}

struct RequestNewRender;
#[derive(Clone, Copy)]
struct Stop;

impl paddle::Frame for Main {
    type State = ();
    const WIDTH: u32 = SCREEN_W;
    const HEIGHT: u32 = SCREEN_H * 2 / 3;

    fn draw(&mut self, _state: &mut Self::State, canvas: &mut DisplayArea, _timestamp: f64) {
        canvas.fit_display(5.0);
        for part in &self.imgs {
            canvas.draw(&part.screen_area, &part.img.img);
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

#[derive(Clone)]
struct PngPart {
    screen_area: Rectangle,
    img: ImageData,
}

#[derive(Clone)]
struct ImageData {
    img: ImageDesc,
    data: Rc<RefCell<Option<Vec<u8>>>>,
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
                0..=2 => 32,
                3 => 128,
                4 => 256,
                5 => 512,
                _ => 1024,
            };

            self.old_images = self.imgs.len();
            if let Some(job) = Self::new_full_screen_job(self.next_quality) {
                let jobs = job.divide(num_new_jobs as u32);
                self.outstanding_jobs = jobs.len();
                paddle::send::<_, WorkerView>(jobs);
                paddle::send::<_, RenderProgress>(ProgressReset {
                    work_items: self.outstanding_jobs,
                });
                self.next_quality += 1;
            }
        }
    }

    /// paddle event listener
    fn new_png_part(&mut self, _state: &mut (), png: &PngPart) {
        let mut bundle = AssetBundle::new();
        bundle.add_images(&[png.img.img]);
        bundle.load();

        self.imgs.push(png.clone());
        self.outstanding_jobs = self.outstanding_jobs.saturating_sub(1);
    }

    /// paddle event listener
    pub fn stop(&mut self, _state: &mut (), _msg: &crate::Stop) {
        self.imgs.drain(self.old_images..);
        self.old_images = 0;
        self.next_quality = self.next_quality.saturating_sub(1);
        self.outstanding_jobs = 0;
    }

    fn new_full_screen_job(quality: u32) -> Option<RenderTask> {
        let mut resolution = 1.0;
        let samples;
        let recursion;
        match quality {
            0 => {
                resolution = 0.25;
                samples = 1;
                recursion = 2;
            }
            1 => {
                resolution = 0.5;
                samples = 1;
                recursion = 4;
            }
            2 => {
                samples = 4;
                recursion = 4;
            }
            3 => {
                samples = 4;
                recursion = 100;
            }
            4 => {
                samples = 64;
                recursion = 100;
            }
            5 => {
                samples = 256;
                recursion = 200;
            }
            6 => {
                // very slightly better bright spot reflection
                samples = 1024;
                recursion = 512;
            }
            _more => return None,
        }
        let screen = Rectangle::new_sized((Main::WIDTH, Main::HEIGHT));
        let settings = RenderSettings {
            resolution: (
                (resolution * screen.width()) as u32,
                (resolution * screen.height()) as u32,
            ),
            samples,
            recursion,
        };
        Some(RenderTask::new(screen, settings))
    }

    /// paddle event listener
    pub fn peer_message(&mut self, state: &mut (), msg: &p2p_proto::Message) {
        match msg {
            p2p_proto::Message::RenderedPart(part) => self.new_png_part(state, part),
        }
    }
}

impl ImageData {
    fn new_from_array(data: Uint8Array) -> Self {
        let vec = data.to_vec();
        Self {
            // TODO: Avoid memory leak (in paddle itself!)
            img: ImageDesc::from_png_binary(&vec).unwrap(),
            data: Rc::new(Some(vec).into()),
        }
    }

    // TODO: avoid memory leaks
    fn new_leaky(path: String) -> Self {
        Self {
            img: ImageDesc::from_path(Box::leak(path.into_boxed_str())),
            data: Rc::new(None.into()),
        }
    }
}

fn button<T: 'static + Clone>(area: Rectangle, color: Color, msg: T, text: String) -> UiElement {
    UiElement::new(area, color)
        .with_rounded_corners(25.0)
        .with_text(text)
        .unwrap()
        .with_text_alignment(FitStrategy::Center)
        .unwrap()
        .with_pointer_interaction(PointerEventType::PrimaryClick, msg)
}
