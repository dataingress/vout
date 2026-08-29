use std::borrow::Cow;

use super::AWS_HEADER_REQUEST_ID;
use http_body_util::Full;
use hyper::{Response, StatusCode, body::Bytes};
use uuid::Uuid;

use crate::server::err::ErrorResponse;

pub type GwResponseInner = Response<Full<Bytes>>;

pub enum GwPassed<'a> {
    Success(GwResponseInner),
    Failure((&'static str, Cow<'a, str>)),
}

impl<'a> GwPassed<'a> {
    pub fn status(&self) -> hyper::StatusCode {
        match self {
            GwPassed::Success(_) => hyper::StatusCode::OK,
            GwPassed::Failure(_) => hyper::StatusCode::BAD_REQUEST,
        }
    }

    pub fn into_response(self, request_id: Uuid) -> anyhow::Result<GwResponseInner> {
        let status = self.status();

        match self {
            GwPassed::Success(mut resp) => {
                resp.headers_mut().insert(
                    AWS_HEADER_REQUEST_ID,
                    request_id.to_string().parse().unwrap(),
                );
                Ok(resp)
            }
            GwPassed::Failure(msg) => build_http_response(
                status,
                request_id,
                ErrorResponse::render(msg.0, msg.1)?.to_string(),
            ),
        }
    }
}

pub type GwResponse<'a> = anyhow::Result<GwPassed<'a>>;

fn build_http_response(
    status: StatusCode,
    request_id: Uuid,
    body: String,
) -> anyhow::Result<GwResponseInner> {
    Ok(Response::builder()
        .status(status)
        .header(AWS_HEADER_REQUEST_ID, request_id.to_string())
        .body(Full::new(Bytes::from(body)))?)
}
