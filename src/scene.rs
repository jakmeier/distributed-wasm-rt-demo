use crate::reflection::*;
use crate::texture::Texture;
use nalgebra::geometry::*;
use nalgebra::Vector3;
use ncollide3d::pipeline::*;
use ncollide3d::query::*;
use ncollide3d::shape::*;

const EPSILON: f32 = f32::EPSILON;

pub struct Scene {
    max_distance: f32,
    collision_group: CollisionGroups,
    query_type: GeometricQueryType<f32>,
    world: CollisionWorld<f32, Texture>,
}

// pub struct ObjectData {
//     color: Vector3<f32>,
//     reflection: ReflectionType,
// }

impl Scene {
    pub fn new(max_distance: f32) -> Self {
        let margin = 0.0002;
        // these values should only matter for collision between objects - not for mere ray casting
        //  Contact points will be generated as long as the two objects are penetrating or closer than the sum of both prediction values.
        let prediction = 0.2;
        let ang_prediction = 0.9;
        //  allow the generation of contacts between two features (vertices, edges, or faces) of solids that should be in contact if the solid were rotated by this amount.
        let query_type = GeometricQueryType::Contacts(prediction, ang_prediction);

        Self {
            world: CollisionWorld::new(margin),
            collision_group: CollisionGroups::new(),
            max_distance,
            query_type,
        }
    }
    pub fn add(&mut self, obj: impl Shape<f32>, position: Isometry3<f32>, texture: Texture) {
        let handle = ShapeHandle::new(obj);

        self.world.add(
            position,
            handle,
            self.collision_group,
            self.query_type,
            texture,
        );
        self.world.update();
    }
    pub fn cast_ray(&self, ray: &Ray<f32>, depth: usize) -> Vector3<f32> {
        if depth == 0 {
            return Vector3::new(0.0, 0.0, 0.0);
        }

        let mut intersections: Vec<_> = self
            .world
            .interferences_with_ray(ray, self.max_distance, &self.collision_group)
            .filter(|(_handle, _obj, collision)| collision.toi > EPSILON)
            .collect();

        intersections.sort_unstable_by(|a, b| a.2.toi.partial_cmp(&b.2.toi).unwrap());

        for (_handle, obj, collision) in intersections {
            let texture = obj.data();
            let point_of_impact = ray.origin + collision.toi * ray.dir;
            let normal = collision.normal;
            let light_in = match texture.reflection_type {
                ReflectionType::Lambert => {
                    let mut new_ray = lambertian_reflection(&point_of_impact, &normal);
                    if let Some(fuzz) = texture.fuzz() {
                        new_ray.dir = new_ray.dir.normalize() + fuzz;
                    }
                    self.cast_ray(&new_ray, depth - 1)
                }
                ReflectionType::Metal => {
                    let mut new_ray = mirror_reflection(&ray.dir, &point_of_impact, &normal);
                    if let Some(fuzz) = texture.fuzz() {
                        new_ray.dir = new_ray.dir.normalize() + fuzz;
                    }
                    self.cast_ray(&new_ray, depth - 1)
                }
                ReflectionType::Absorb => Vector3::new(0.0, 0.0, 0.0),
            };
            return texture.color_strength() * texture.color()
                + texture.reflective_strength() * light_in;
        }
        background_color(ray)
    }
}

fn background_color(ray: &Ray<f32>) -> Vector3<f32> {
    let direction: Vector3<f32> = ray.dir.into();
    let unit_direction = direction.normalize();
    let blue = Vector3::new(0.0, 0.0, 0.1);
    // let white = Vector3::new(1.0, 1.0, 1.0);
    let black = Vector3::new(0.0, 0.0, 0.0005);
    // let t = 0.25 * (3.0*unit_direction.y + 1.0);
    let t = 0.35 - unit_direction.y;
    let t = t.max(0.0).min(1.0);
    let col = (1.0 - t) * black + t * blue;
    let shadow = 1.0;
    col * shadow
}
