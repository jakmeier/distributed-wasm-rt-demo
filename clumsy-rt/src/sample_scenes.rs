use nalgebra::geometry::*;
use nalgebra::Vector3;

use ncollide3d::query::Ray;
use ncollide3d::shape::*;
use std::f32::consts::*;

use crate::*;

pub fn build_cool_scene() -> Scene {
    const MAX_DISTANCE: f32 = 1_000_000.0;
    let mut scene = SceneBuilder::new(MAX_DISTANCE, sky);

    let rot = UnitQuaternion::from_scaled_axis(Vector3::y() * PI);

    // big sphere
    let tran = Translation3::new(0.0, VIEWPORT_WIDTH * -100.5 - 2.5, -5.0 * VIEWPORT_WIDTH);
    let big_sphere_radius = VIEWPORT_WIDTH * 100.0;
    let floor_col = Vector3::new(0.05, 0.8, 0.075);
    scene.add(
        Ball::new(big_sphere_radius),
        Isometry3::from_parts(tran, rot),
        Texture::diffuse(floor_col, 0.5).with_fuzz(0.125),
    );

    // center sphere
    let center_sphere_radius = VIEWPORT_WIDTH;
    let center_h = 0.5;
    let y = center_h + 1.75 * center_sphere_radius;
    let tran = Translation3::new(0.0, y, -5.0 * VIEWPORT_WIDTH);
    let blue = Vector3::new(0.0, 0.0, 0.5);
    scene.add(
        Ball::new(center_sphere_radius),
        Isometry3::from_parts(tran, rot),
        Texture::metal(blue, 0.6, 0.2).with_fuzz(0.125),
    );

    // moon (sun)
    let moon_d = VIEWPORT_WIDTH * 500.0;
    let tran = Translation3::new(moon_d, moon_d, -2.0 * moon_d);
    let moon_radius = VIEWPORT_WIDTH * 100.0;
    let moon_col = Vector3::new(10.0, 10.0, 0.5);
    scene.add(
        Ball::new(moon_radius),
        Isometry3::from_parts(tran, rot),
        Texture::perfect_diffuse(moon_col),
    );

    // die
    let tran = Translation3::new(0.0, center_h - 3.0, -5.0 * VIEWPORT_WIDTH);
    let die_rot = UnitQuaternion::from_scaled_axis(Vector3::x() * FRAC_PI_4)
        * UnitQuaternion::from_scaled_axis(Vector3::z() * FRAC_PI_4)
        * UnitQuaternion::from_scaled_axis(Vector3::y() * FRAC_PI_2);
    let red = Vector3::new(0.85, 0.0, 0.0);
    let side_len = center_sphere_radius * 0.3819;
    let die = Cuboid::new(Vector3::new(side_len, side_len, side_len));
    scene.add(
        die,
        Isometry3::from_parts(tran, die_rot),
        Texture::metal(red, 0.65, 0.25),
    );

    let smaller = VIEWPORT_WIDTH / 4.0;
    for ring_level in 0..4 {
        let r = center_sphere_radius + 1.0 + 1.25 * ring_level as f32;
        let y = center_h + - ring_level as f32 * 1.25;
        for alpha in 0..8 {
            let alpha = std::f32::consts::FRAC_PI_4 * (alpha as f32 + 0.5);
            let x = r * alpha.cos();
            let z = r * alpha.sin() - 5.0 * VIEWPORT_WIDTH;
            let tran = Translation3::new(x, y, z);
            scene.add(
                Ball::new(smaller),
                Isometry3::from_parts(tran, rot),
                Texture::dark_mirror(0.5),
            );
        }
    }

    scene.build()
}

pub fn build_simple_scene() -> Scene {
    const MAX_DISTANCE: f32 = 100.0;
    let mut scene = SceneBuilder::new(MAX_DISTANCE, simple_background);

    let rot = UnitQuaternion::from_scaled_axis(Vector3::y() * PI);

    // big sphere
    let tran = Translation3::new(0.0, VIEWPORT_WIDTH * -100.5, -5.0 * VIEWPORT_WIDTH);
    let sphere_radius = VIEWPORT_WIDTH * 100.0;
    let col = Vector3::new(0.2, 0.8, 0.2);
    scene.add(
        Ball::new(sphere_radius),
        Isometry3::from_parts(tran, rot),
        Texture::diffuse(col, 0.5),
    );

    // small sphere
    let tran = Translation3::new(0.0, 0.0, -5.0 * VIEWPORT_WIDTH);
    let sphere_radius = VIEWPORT_WIDTH;
    let col = Vector3::new(1.0, 1.0, 1.0);
    scene.add(
        Ball::new(sphere_radius),
        Isometry3::from_parts(tran, rot),
        Texture::metal(col, 0.05, 0.9).with_fuzz(0.5),
    );

    scene.build()
}

fn sky(ray: &Ray<f32>) -> Vector3<f32> {
    // todo: use sun as main source of light
    let direction: Vector3<f32> = ray.dir.into();
    let unit_direction = direction.normalize();
    let dark = Vector3::new(-0.3, -0.3, 0.3);
    let light = Vector3::new(0.5, 0.5, 1.55);
    let t = 0.35 - unit_direction.y;
    let t = t.max(0.0).min(1.0);
    let col = 0.5 * (1.0 - t) * light + t * dark;
    col
}

fn simple_background(ray: &Ray<f32>) -> Vector3<f32> {
    let direction: Vector3<f32> = ray.dir.into();
    let t = direction.normalize().y;
    let white = Vector3::new(1.0, 1.0, 1.0);
    t * white
}
