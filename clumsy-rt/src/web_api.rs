use crate::render_job::RenderJobExt;
use api::RenderJob;
use js_sys::Uint32Array;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn render(array: Uint32Array) -> Vec<u8> {
    console_error_panic_hook::set_once();
    let vec: Vec<u32> = Uint32Array::from(array).to_vec();
    let job = RenderJob::try_from_slice(&vec).unwrap();

    let result = job.render();

    result
}
