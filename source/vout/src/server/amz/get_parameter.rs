use macros::ApiUnsupportedFields;
use serde::Deserialize;

use crate::{deserialize_request_body, server::res::GwResponse};

fn default_request_with_decryption() -> bool {
    false
}

#[derive(Deserialize, ApiUnsupportedFields)]
#[serde(rename_all = "PascalCase")]
struct Request {
    name: String,
    #[serde(default = "default_request_with_decryption")]
    with_decryption: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct GetParameterResponse {
    parameter: super::Parameter,
}

pub async fn handler<'a>(req: super::AmzRequest) -> GwResponse<'a> {
    let request = deserialize_request_body!(req, Request);
    let selector = super::parse_selector(&request.name);

    if let Some(err) = super::validate_parameter_name(selector.name()) {
        return Ok(err);
    }

    let conn = crate::db::open().await?;
    let Some(version) = super::resolve_version(&conn, &selector).await? else {
        return Ok(crate::server::err::builder::parameter_not_found());
    };
    let parameter =
        super::parameter_from_version(version, selector.selector(), request.with_decryption)?;

    super::json_response(&GetParameterResponse { parameter })
}
