use anyhow::{Context, Result};
use clumsy_rt::RenderJobExt;
use spin_sdk::{
    http::{Request, Response},
    http_component,
};
use std::str::FromStr;

/// A simple Spin HTTP component.
#[http_component]
fn handle_spin_component(req: Request) -> Result<Response> {
    let bytes = req.body().as_ref().context("requires body")?;
    let string = std::str::from_utf8(bytes)?;
    let job = api::RenderJob::from_str(string)?;

    println!("{job:?}");
    let dt = std::time::Instant::now();
    let reponse_bytes = job.render();
    println!("done after {:<#.1?}", dt.elapsed());

    Ok(http::Response::builder()
        .status(200)
        .header("Content-Type", "image/png")
        .body(Some(reponse_bytes.into()))?)
}
