use anyhow::Result;
use clumsy_rt::RenderJobExt;
use spin_sdk::{
    http::{Request, Response},
    http_component,
};
use std::str::FromStr;

/// A simple Spin HTTP component.
#[http_component]
fn handle_spin_component(req: Request) -> Result<Response> {
    let job = api::RenderJob::from_str(req.uri().path())?;

    println!("{job:?}");
    let dt = std::time::Instant::now();
    let reponse_bytes = job.render();
    println!("done after {:<#.1?}", dt.elapsed());

    Ok(http::Response::builder()
        .status(200)
        .header("Content-Type", "image/png")
        .header("Access-Control-Allow-Origin", "*")
        .body(Some(reponse_bytes.into()))?)
}
