use crate::render_job::RenderJobExt;
use api::RenderJob;
use js_sys::Uint32Array;
use wasm_bindgen::prelude::wasm_bindgen;

macro_rules! console_log {
    ($($t:tt)*) => (web_sys::console::log_1(&format_args!($($t)*).to_string().into()))
}

#[wasm_bindgen]
pub fn render(array: Uint32Array) -> Vec<u8> {
    console_error_panic_hook::set_once();
    let vec: Vec<u32> = Uint32Array::from(array).to_vec();
    let job = RenderJob::try_from_slice(&vec).unwrap();

    console_log!("{job:?}");
    let result = job.render();

    result
}
