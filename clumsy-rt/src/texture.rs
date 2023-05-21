use super::*;
use nalgebra::Vector3;
use rand::Rng;

pub struct Texture {
    /// Determines direction and weight of recursive rays
    pub reflection_type: ReflectionType,
    /// Randomized softness
    pub fuzz: Option<f32>,
    color: Vector3<f32>,
    /// How much of incoming light is reflected
    reflective_strength: f32,
    /// How much of incoming light is absorbed
    // absorption_strength: f32,
    /// How much of incoming light is converted to the object's own color
    color_strength: f32,
}

impl Texture {
    pub fn new(
        color: Vector3<f32>,
        reflection_type: ReflectionType,
        color_strength: f32,
        reflective_strength: f32,
    ) -> Self {
        Self {
            reflection_type,
            color,
            color_strength,
            reflective_strength,
            // absorption_strength: 1.0 - color_strength - reflective_strength,
            fuzz: None,
        }
    }
    pub fn perfect_diffuse(color: Vector3<f32>) -> Self {
        Self::new(color, ReflectionType::Lambert, 1.0, 0.0)
    }
    pub fn diffuse(color: Vector3<f32>, absorb: f32) -> Self {
        Self::new(color, ReflectionType::Lambert, 1.0 - absorb, 0.0)
    }
    pub fn perfect_mirror() -> Self {
        Self::new(Vector3::new(0.0, 0.0, 0.0), ReflectionType::Metal, 0.0, 1.0)
    }
    pub fn dark_mirror(absorb: f32) -> Self {
        Self::metal(Vector3::new(0.0, 0.0, 0.0), 0.0, 1.0 - absorb)
    }
    pub fn metal(color: Vector3<f32>, color_strength: f32, reflect: f32) -> Self {
        Self::new(color, ReflectionType::Metal, color_strength, reflect)
    }
    pub fn color(&self) -> Vector3<f32> {
        self.color
    }
    pub fn color_strength(&self) -> f32 {
        self.color_strength
    }
    pub fn reflective_strength(&self) -> f32 {
        self.reflective_strength
    }
    pub fn with_fuzz(mut self, f: f32) -> Self {
        self.fuzz = Some(f);
        self
    }
    pub fn fuzz(&self) -> Option<Vector3<f32>> {
        self.fuzz.map(random_fuzz)
    }
}

fn random_fuzz(max: f32) -> Vector3<f32> {
    let mut rng = rand::thread_rng();
    let x = rng.gen::<f32>() * 2.0 * max - max;
    let y = rng.gen::<f32>() * 2.0 * max - max;
    let z = rng.gen::<f32>() * 2.0 * max - max;
    Vector3::new(x, y, z)
}
