use super::*;
use nalgebra::Vector3;
use rand::Rng;

pub struct Texture {
    pub reflection_type: ReflectionType,
    pub fuzz: Option<f32>,
    color: Vector3<f32>,
    color_strength: f32,
}

impl Texture {
    pub fn new(color: Vector3<f32>, reflection_type: ReflectionType, color_strength: f32) -> Self {
        Self {
            reflection_type,
            color,
            color_strength,
            fuzz: None,
        }
    }
    pub fn diffuse(color: Vector3<f32>) -> Self {
        Self::new(color, ReflectionType::Lambert, 0.75)
    }
    pub fn perfect_mirror() -> Self {
        Self::dark_mirror(0.0)
    }
    pub fn dark_mirror(absorb: f32) -> Self {
        Self::metal(Vector3::new(0.0, 0.0, 0.0), absorb)
    }
    pub fn metal(color: Vector3<f32>, absorb: f32) -> Self {
        Self::new(color, ReflectionType::Metal, absorb)
    }
    pub fn color(&self) -> Vector3<f32> {
        self.color
    }
    pub fn color_strength(&self) -> f32 {
        self.color_strength
    }
    pub fn reflective_strength(&self) -> f32 {
        1.0 - self.color_strength
    }
    pub fn with_fuzz(mut self, f: f32) -> Self{
        self.fuzz = Some(f);
        self
    }
    pub fn fuzz(&self) -> Option<Vector3<f32>> {
        self.fuzz.map(random_fuzz)
    }
}

fn random_fuzz(max: f32) -> Vector3<f32> {
    let mut rng = rand::thread_rng();
    let x: f32 = rng.gen_range(-max, max);
    let y: f32 = rng.gen_range(-max, max);
    let z: f32 = rng.gen_range(-max, max);
    Vector3::new(x, y, z)
}
