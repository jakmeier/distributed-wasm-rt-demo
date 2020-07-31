#![allow(dead_code)]

mod camera;
mod output;
mod pixel;
mod reflection;
mod scene;
mod texture;

pub use camera::*;
pub use pixel::*;
pub use reflection::*;
pub use scene::*;
pub use texture::*;

use nalgebra::geometry::*;
use nalgebra::Vector3;
use std::path::Path;

use ncollide3d::shape::*;
use std::f32::consts::*;

fn main() -> std::io::Result<()> {
    let n_threads: usize = std::env::var("N_THREADS")
        .map(|s| s.parse::<usize>().expect("invalid value"))
        .unwrap_or(12);

    let n_samples: usize = std::env::var("N_SAMPLES")
        .map(|s| s.parse::<usize>().expect("invalid value"))
        .unwrap_or(9);
    let size_scalar: usize = std::env::var("SIZE_SCALAR")
        .map(|s| s.parse::<usize>().expect("invalid value"))
        .unwrap_or(120);
    let n_recursion: usize = std::env::var("N_RECURSION")
        .map(|s| s.parse::<usize>().expect("invalid value"))
        .unwrap_or(50);

    // let scene = build_simple_scene();
    let scene = build_cool_scene();
    let camera = Camera::new(n_samples, n_recursion);

    let w = 4 * size_scalar;
    let h = 3 * size_scalar;
    let mut img = PixelPlane::new(w, h);

    println!("{}x{}", w, h);
    println!("{}x multi-sampling", n_samples);
    println!("{}x ray-bouncing", n_recursion);

    camera.render(scene, &mut img, n_threads);

    img.export_png(Path::new("out.png"))?;
    // img.export_ppm(Path::new("out.ppm"))?;

    Ok(())
}

fn build_cool_scene() -> Scene {
    const MAX_DISTANCE: f32 = 100.0;
    let mut scene = Scene::new(MAX_DISTANCE);

    let rot = UnitQuaternion::from_scaled_axis(Vector3::y() * PI);

    // big sphere
    let tran = Translation3::new(0.0, VIEWPORT_WIDTH * -100.5, -5.0 * VIEWPORT_WIDTH);
    let big_sphere_radius = VIEWPORT_WIDTH * 100.0;
    let col = Vector3::new(0.2, 0.8, 0.2);
    scene.add(
        Ball::new(big_sphere_radius),
        Isometry3::from_parts(tran, rot),
        Texture::new_diffuse(col),
    );

    // small sphere
    let tran = Translation3::new(0.0, 0.0, -5.0 * VIEWPORT_WIDTH);
    let sphere_radius = VIEWPORT_WIDTH;
    let col = Vector3::new(0.0, 0.0, 0.0);
    scene.add(
        Ball::new(sphere_radius),
        Isometry3::from_parts(tran, rot),
        Texture::new_diffuse(col),
    );

    let smaller = VIEWPORT_WIDTH / 4.0;
    for ring_level in 0..4 {
        let r = sphere_radius + 1.0 + 1.25 * ring_level as f32;
        let y = 0.0 - ring_level as f32 * 0.5;
        for alpha in 0..8 {
            let alpha = std::f32::consts::FRAC_PI_4 * alpha as f32;
            let x = r * alpha.cos();
            let z = r * alpha.sin() - 5.0 * VIEWPORT_WIDTH;
            // println!("alpha: {}, r: {}, x: {}, y: {}, z: {}", alpha, r, x, y, z);
            let absorb = 0.5;
            let tran = Translation3::new(x, y, z);
            scene.add(
                Ball::new(smaller),
                Isometry3::from_parts(tran, rot),
                Texture::new_mirror(absorb),
            );
        }
    }

    scene
}

fn build_simple_scene() -> Scene {
    const MAX_DISTANCE: f32 = 100.0;
    let mut scene = Scene::new(MAX_DISTANCE);

    let rot = UnitQuaternion::from_scaled_axis(Vector3::y() * PI);

    // big sphere
    let tran = Translation3::new(0.0, VIEWPORT_WIDTH * -100.5, -5.0 * VIEWPORT_WIDTH);
    let sphere_radius = VIEWPORT_WIDTH * 100.0;
    let col = Vector3::new(0.2, 0.8, 0.2);
    scene.add(
        Ball::new(sphere_radius),
        Isometry3::from_parts(tran, rot),
        Texture::new_diffuse(col),
    );

    // small sphere
    let tran = Translation3::new(0.0, 0.0, -5.0 * VIEWPORT_WIDTH);
    let sphere_radius = VIEWPORT_WIDTH;
    let col = Vector3::new(0.0, 0.0, 0.0);
    scene.add(
        Ball::new(sphere_radius),
        Isometry3::from_parts(tran, rot),
        Texture::new_diffuse(col),
    );

    scene
}

#[test]
fn produce_test_img() -> std::io::Result<()> {
    const W: usize = 256;
    const H: usize = 256;

    let mut img = PixelPlane::new(W, H);
    for y in (0..H).rev() {
        for x in 0..W {
            let r = x as f32 / (W - 1) as f32;
            let g = (H - y - 1) as f32 / (H - 1) as f32;
            let b = 0.25;
            img.set_pixel(x, y, Pixel::rgb(r, g, b));
        }
    }
    img.export_ppm(Path::new("test.ppm"))?;
    img.export_png(Path::new("test.png"))?;
    Ok(())
}
