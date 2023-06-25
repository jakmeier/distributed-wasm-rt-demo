//! Simple CPU ray-tracer, based on and inspired by https://github.com/RayTracing/raytracing.github.io

mod camera;
mod output;
mod pixel;
mod reflection;
mod render_job;
mod scene;
mod texture;

pub mod sample_scenes;

pub use camera::*;
pub use pixel::*;
pub use reflection::*;
pub use render_job::RenderJobExt;
pub use scene::*;
pub use texture::*;

#[cfg(feature = "web")]
mod web_api;
