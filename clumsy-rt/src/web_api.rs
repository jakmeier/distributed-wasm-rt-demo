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
    let start = js_sys::Date::now();
    let result = job.render();
    let end = js_sys::Date::now();
    console_log!("done after {:<#.1?}", millis_to_duration(end - start));

    result
}

fn millis_to_duration(amt: f64) -> std::time::Duration {
    let secs = (amt as u64) / 1_000;
    let nanos = (((amt as u64) % 1_000) as u32) * 1_000_000;
    std::time::Duration::new(secs, nanos)
}
