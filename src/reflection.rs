use nalgebra::{Point3, Vector3};
use ncollide3d::query::Ray;
use rand::Rng;
use std::f32::consts::*;

pub enum ReflectionType {
    Lambert,
    Mirror,
    Absorb,
}

pub fn mirror_reflection(
    direction: &Vector3<f32>,
    point: &Point3<f32>,
    normal: &Vector3<f32>,
) -> Ray<f32> {
    let out_direction = direction - 2.0 * direction.dot(normal) * normal;
    Ray::new(*point, out_direction)
}

/// Reflects randomly from a surface using a lambertian distribution. ( cos(alpha) )
/// Note that the incoming angle is ignored, only the surface normal matters.
pub fn lambertian_reflection(point: &Point3<f32>, normal: &Vector3<f32>) -> Ray<f32> {
    let mut rng = rand::thread_rng();
    let a: f32 = rng.gen_range(0.0, PI * 2.0);
    let z: f32 = rng.gen_range(-1.0, 1.0);
    let r = (1.0 - z * z).sqrt();
    let out_direction = normal + Vector3::new(r * a.cos(), r * a.cos(), z);
    Ray::new(*point, out_direction)
}
