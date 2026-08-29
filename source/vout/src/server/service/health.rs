use http_body_util::Full;
use hyper::Response;
use hyper::body::Bytes;

use crate::server::res::{GwPassed, GwResponse};

pub async fn handler<'a>() -> GwResponse<'a> {
    Ok(GwPassed::Success(Response::new(Full::new(Bytes::from(
        "OK",
    )))))
}
