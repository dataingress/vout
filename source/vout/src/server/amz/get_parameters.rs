use macros::ApiUnsupportedFields;
use serde::Deserialize;

use crate::{deserialize_request_body, server::res::GwResponse};

fn default_request_with_decryption() -> bool {
    false
}

#[derive(Deserialize, ApiUnsupportedFields)]
#[serde(rename_all = "PascalCase")]
struct Request {
    names: Vec<String>,
    #[serde(default = "default_request_with_decryption")]
    with_decryption: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct GetParametersResponse {
    parameters: Vec<super::Parameter>,
    invalid_parameters: Vec<String>,
}

pub async fn handler<'a>(req: super::AmzRequest) -> GwResponse<'a> {
    let request = deserialize_request_body!(req, Request);

    if request.names.is_empty() || request.names.len() > super::max_get_parameters_names() {
        return Ok(crate::server::err::builder::invalid_parameter(
            "Invalid number of parameter names.",
        ));
    }

    let conn = crate::db::open().await?;
    let mut parameters = Vec::new();
    let mut invalid_parameters = Vec::new();

    for name in request.names {
        let selector = super::parse_selector(&name);

        if super::validate_parameter_name(selector.name()).is_some() {
            invalid_parameters.push(name);
            continue;
        }

        if let Some(version) = super::resolve_version(&conn, &selector).await? {
            parameters.push(super::parameter_from_version(
                version,
                selector.selector(),
                request.with_decryption,
            )?);
        } else {
            invalid_parameters.push(name);
        }
    }

    parameters.sort_by(|left, right| left.name.cmp(&right.name));
    invalid_parameters.sort();

    super::json_response(&GetParametersResponse {
        parameters,
        invalid_parameters,
    })
}
