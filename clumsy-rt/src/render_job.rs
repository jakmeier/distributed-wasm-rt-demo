use api::RenderJob;

use crate::{sample_scenes, Camera, PixelPlane};

pub trait RenderJobExt {
    fn render(&self) -> Vec<u8>;
}

impl RenderJobExt for RenderJob {
    fn render(&self) -> Vec<u8> {
        let mut pixels = PixelPlane::new(self.w as usize, self.h as usize);
        let camera = Camera::new(
            self.n_samples as usize,
            self.n_recursion as usize,
            self.camera_w as usize,
            self.camera_h as usize,
        );
        let scene = sample_scenes::build_cool_scene();
        camera.render_tile(&scene, self.x as usize, self.y as usize, &mut pixels);

        let mut buf = Vec::new();
        pixels
            .write_png(&mut buf)
            .expect("failed writing png to buffer");
        buf
    }
}

#[test]
fn smoke_test() {
    RenderJob::new(0, 0, 128, 128, 128, 128, 1, 1).render();
    RenderJob::new(0, 0, 96, 54, 960, 540, 2, 2).render();
    RenderJob::new(0, 0, 96, 54, 96, 54, 2, 2).render();
    RenderJob::new(48, 0, 48, 27, 96, 54, 2, 2).render();
    RenderJob::new(150, 157, 30, 22, 240, 180, 1, 2).render();
}
