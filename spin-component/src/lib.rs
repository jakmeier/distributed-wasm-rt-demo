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
    if req.uri().path() == "/ping" {
        let body = Some("pong".as_bytes().to_vec().into());
        return Ok(http::Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .header("Content-Type", "text/plain")
            .body(body)?);
    }
    let job = api::RenderJob::from_str(req.uri().path())?;

    let dt = std::time::Instant::now();
    let response_bytes = job.render();
    println!("{job:?} done after {:<#.1?}", dt.elapsed());

    Ok(http::Response::builder()
        .status(200)
        .header("Content-Type", "image/png")
        .header("Access-Control-Allow-Origin", "*")
        .body(Some(response_bytes.into()))?)
}
