use super::*;
use nalgebra::Vector3;

pub struct Texture {
    pub reflection_type: ReflectionType,
    pub color: Vector3<f32>,
    pub reflective_strength: f32,
    pub color_strength: f32,
    pub absorb: f32,
}

impl Texture {
    pub fn new(
        color: Vector3<f32>,
        reflection_type: ReflectionType,
        reflective_strength: f32,
        color_strength: f32,
    ) -> Self {
        Self {
            reflection_type,
            color,
            reflective_strength,
            color_strength,
            absorb: 1.0 - reflective_strength - color_strength,
        }
    }
    pub fn new_diffuse(color: Vector3<f32>) -> Self {
        Self::new(color, ReflectionType::Lambert, 0.25, 0.5)
    }
    pub fn new_mirror(absorb: f32) -> Self {
        Self::new(
            Vector3::new(1.0, 1.0, 1.0),
            ReflectionType::Mirror,
            0.0,
            1.0 - absorb,
        )
    }
}
