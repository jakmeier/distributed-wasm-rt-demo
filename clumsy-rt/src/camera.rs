use super::*;
use nalgebra::Vector3;
use ncollide3d::query::Ray;
use std::sync::Arc;
use std::thread;

pub const VIEWPORT_S: f32 = 0.5;
pub const VIEWPORT_WIDTH: f32 = 4.0 * VIEWPORT_S;
pub const VIEWPORT_HEIGHT: f32 = 3.0 * VIEWPORT_S;
pub const FOCAL_LENGTH: f32 = 1.0;

#[derive(Clone)]
pub struct Camera {
    origin: Vector3<f32>,
    lower_left_corner: Vector3<f32>,
    vertical: Vector3<f32>,
    horizontal: Vector3<f32>,
    w_samples: usize,
    h_samples: usize,
    n_recursion: usize,
}
impl Camera {
    pub fn new(n_samples: usize, n_recursion: usize) -> Self {
        let origin = Vector3::new(0.0, 0.0, 0.0);
        let horizontal = Vector3::x() * VIEWPORT_WIDTH;
        let vertical = Vector3::y() * VIEWPORT_HEIGHT;
        let lower_left_corner =
            origin - horizontal / 2.0 - vertical / 2.0 - Vector3::z() * FOCAL_LENGTH;
        let w_samples = (n_samples as f64).sqrt() as usize;
        let h_samples = n_samples / w_samples;
        Self {
            origin,
            horizontal,
            vertical,
            lower_left_corner,
            w_samples,
            h_samples,
            n_recursion,
        }
    }

    pub fn render(&self, scene: Scene, buffer: &mut PixelPlane, n_threads: usize) {
        let (w, h) = (buffer.w, buffer.h);
        if n_threads == 1 {
            let mut shard = buffer.into();
            self.render_shard(&scene, w, h, &mut shard);
            *buffer = shard.into();
            return;
        }
        let shards = buffer.split_into_shards(n_threads);
        let mut handles = vec![];
        let scene = Arc::new(scene);
        for mut shard in shards {
            let camera = self.clone();
            let scene = scene.clone();
            let handle = thread::spawn(move || {
                camera.render_shard(&scene, w, h, &mut shard);
                shard
            });
            handles.push(handle);
        }
        let finished_shards = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>();
        buffer.collect_shards(&finished_shards);
    }
    fn render_shard(&self, scene: &Scene, w: usize, h: usize, shard: &mut PixelPlaneShard) {
        for y in 0..shard.h {
            for x in 0..shard.w {
                let mut col = Vector3::new(0.0, 0.0, 0.0);
                for xs in 0..self.w_samples {
                    for ys in 0..self.h_samples {
                        let xi = (shard.x + x) as f32 + xs as f32 / self.w_samples as f32;
                        let yi = (shard.y + y) as f32 + ys as f32 / self.h_samples as f32;
                        let u = xi / (w - 1) as f32;
                        let v = yi / (h - 1) as f32;
                        let ray = self.get_ray(u, v);
                        let ray_col = scene.cast_ray(&ray, self.n_recursion);
                        col += ray_col;
                    }
                }
                col /= (self.w_samples * self.h_samples) as f32;
                shard.set_pixel(x, shard.h - 1 - y, Pixel::rgb_vec(col));
            }
        }
    }

    /// Computes a ray through the viewport with the given real pixel coordiantes (ranging from 0.0 to 1.0).
    fn get_ray(&self, u: f32, v: f32) -> Ray<f32> {
        Ray::new(
            self.origin.into(),
            self.lower_left_corner + u * self.horizontal + v * self.vertical - self.origin,
        )
    }
}
