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
pub use scene::*;
pub use texture::*;

use api::RenderJob;
use js_sys::Uint32Array;
use render_job::RenderJobExt;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn render(array: Uint32Array) -> Vec<u8> {
    let vec: Vec<u32> = Uint32Array::from(array).to_vec();
    let job = RenderJob::try_from_slice(&vec).unwrap();
    job.render()
}
