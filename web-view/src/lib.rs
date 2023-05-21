use clumsy_rt::Scene;
use paddle::println;
use paddle::*;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn start() {
    // Build configuration object to define all setting
    let config = PaddleConfig::default()
        .with_canvas_id("paddle-canvas-id")
        .with_resolution((480, 360))
        .with_texture_config(TextureConfig::default().without_filter());

    // Initialize framework state and connect to browser window
    paddle::init(config).expect("Paddle initialization failed.");
    let state = Main {
        scene: clumsy_rt::sample_scenes::build_cool_scene(),
        rendered: None,
        tracker: None,
    };
    paddle::register_frame(state, (), (0, 0));
}

struct Main {
    scene: Scene,
    rendered: Option<ImageDesc>,
    tracker: Option<AssetLoadingTracker>,
}

impl paddle::Frame for Main {
    type State = ();
    const WIDTH: u32 = 480;
    const HEIGHT: u32 = 360;

    fn draw(&mut self, _state: &mut Self::State, canvas: &mut DisplayArea, _timestamp: f64) {
        canvas.fit_display(5.0);
        if let Some(img) = &self.rendered {
            if self.tracker.as_ref().unwrap().loaded() > 0 {
                canvas.draw(&Rectangle::new_sized((480, 360)), img);
            } else if self.tracker.as_ref().unwrap().had_error() {
                println!("there was an error")
            }
        }
    }

    fn update(&mut self, _state: &mut Self::State) {
        if self.rendered.is_none() {
            let n_samples = 4;
            let n_recursion = 4;
            let n_threads = 1;
            let w = 480;
            let h = 360;
            let mut img = clumsy_rt::PixelPlane::new(w, h);

            let camera = clumsy_rt::Camera::new(n_samples, n_recursion);
            println!("[{:#}] Rendering starts", paddle::utc_now());
            camera.render(self.scene.clone(), &mut img, n_threads);
            println!("[{:#}] Rendering done", paddle::utc_now());
            let mut buf = Vec::new();
            img.write_png(&mut buf).unwrap();
            let img_desc = ImageDesc::from_png_binary(&buf).unwrap();
            let mut bundle = AssetBundle::new();
            bundle.add_images(&[img_desc]);
            self.tracker = Some(bundle.load());
            self.rendered = Some(img_desc);
        }
    }

    fn pointer(&mut self, _state: &mut Self::State, _event: PointerEvent) {}
}
