use clumsy_rt::Scene;
use paddle::println;
use paddle::quicksilver_compat::about_equal;
use paddle::*;
use wasm_bindgen::prelude::wasm_bindgen;

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
    let state = Main {
        scene: IncrementalSceneRenderer::new(clumsy_rt::sample_scenes::build_cool_scene()),
    };
    paddle::register_frame(state, (), (0, 0));
}

struct Main {
    scene: IncrementalSceneRenderer,
}

impl paddle::Frame for Main {
    type State = ();
    const WIDTH: u32 = 480;
    const HEIGHT: u32 = 360;

    fn draw(&mut self, _state: &mut Self::State, canvas: &mut DisplayArea, _timestamp: f64) {
        canvas.fit_display(5.0);
        if let Some(img) = self.scene.img() {
            canvas.draw(&Rectangle::new_sized((SCREEN_W, SCREEN_H)), img);
        }
    }

    fn update(&mut self, _state: &mut Self::State) {
        self.scene.render();
    }

    fn pointer(&mut self, _state: &mut Self::State, _event: PointerEvent) {}
}

struct IncrementalSceneRenderer {
    scene: Scene,
    rendered: Vec<ImageDesc>,
    tracker: Option<AssetLoadingTracker>,
    cooldown: i64,
}

impl IncrementalSceneRenderer {
    pub fn new(scene: Scene) -> Self {
        Self {
            scene,
            rendered: vec![],
            tracker: None,
            cooldown: 0,
        }
    }

    pub fn render(&mut self) {
        if self.cooldown > paddle::utc_now().timestamp() {
            return;
        }
        if let Some(tracker) = &self.tracker {
            if tracker.had_error() {
                println!("there was an error")
            } else if !about_equal(tracker.progress(), 1.0) {
                self.ping_cooldown();
                return;
            }
        }
        self.render_new_quality();
    }

    pub fn img(&self) -> Option<&ImageDesc> {
        self.rendered.last()
    }

    fn render_new_quality(&mut self) {
        let quality = self.rendered.len();

        let n_samples = 4 + 10 * quality;
        let n_recursion = 4 + 10 * quality;
        let n_threads = 1;
        let w = 480;
        let h = 360;

        let mut img = clumsy_rt::PixelPlane::new(w, h);
        let camera = clumsy_rt::Camera::new(n_samples, n_recursion);
        println!(
            "[{:#}] Rendering quality {quality} starts",
            paddle::utc_now()
        );
        camera.render(self.scene.clone(), &mut img, n_threads);
        println!("[{:#}] Rendering quality {quality} done", paddle::utc_now());

        let mut buf = Vec::new();
        img.write_png(&mut buf).unwrap();
        let img_desc = ImageDesc::from_png_binary(&buf).unwrap();
        let mut bundle = AssetBundle::new();
        bundle.add_images(&[img_desc]);
        self.tracker = Some(bundle.load());
        self.rendered.push(img_desc);
        self.ping_cooldown();
    }

    fn ping_cooldown(&mut self) {
        self.cooldown = paddle::utc_now().timestamp() + 1;
    }
}
