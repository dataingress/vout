use http_body_util::Full;
use hyper::Response;
use hyper::body::Bytes;

use crate::server::res::{GwPassed, GwResponse};

#[derive(serde::Serialize)]
struct Object {
    version: &'static str,
    ready: bool,
}

pub async fn handler<'a>() -> GwResponse<'a> {
    Ok(GwPassed::Success(Response::new(Full::new(Bytes::from(
        serde_json::to_string(&Object {
            version: env!("CARGO_PKG_VERSION"),
            ready: true,
        })
        .unwrap(),
    )))))
}
