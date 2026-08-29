use macros::ApiUnsupportedFields;
use sea_orm::TransactionTrait;
use serde::Deserialize;

use crate::{deserialize_request_body, server::res::GwResponse};

#[derive(Deserialize, ApiUnsupportedFields)]
#[serde(rename_all = "PascalCase")]
struct Request {
    names: Vec<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct DeleteParametersResponse {
    deleted_parameters: Vec<String>,
    invalid_parameters: Vec<String>,
}

pub async fn handler<'a>(req: super::AmzRequest) -> GwResponse<'a> {
    let request = deserialize_request_body!(req, Request);

    if let Some(err) =
        super::validate_parameter_names(&request.names, super::max_delete_parameters_names())
    {
        return Ok(err);
    }

    let conn = crate::db::open().await?;
    let txn = conn.begin().await?;
    let mut deleted_parameters = Vec::new();
    let mut invalid_parameters = Vec::new();

    for name in request.names {
        if super::delete_parameter_named(&txn, &name).await? {
            deleted_parameters.push(name);
        } else {
            invalid_parameters.push(name);
        }
    }

    deleted_parameters.sort();
    invalid_parameters.sort();
    txn.commit().await?;

    super::json_response(&DeleteParametersResponse {
        deleted_parameters,
        invalid_parameters,
    })
}
