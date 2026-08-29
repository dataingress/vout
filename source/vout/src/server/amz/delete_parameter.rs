use macros::ApiUnsupportedFields;
use sea_orm::TransactionTrait;
use serde::Deserialize;

use crate::{deserialize_request_body, server::res::GwResponse};

#[derive(Deserialize, ApiUnsupportedFields)]
#[serde(rename_all = "PascalCase")]
struct Request {
    name: String,
}

pub async fn handler<'a>(req: super::AmzRequest) -> GwResponse<'a> {
    let request = deserialize_request_body!(req, Request);

    if let Some(err) = super::validate_parameter_name(&request.name) {
        return Ok(err);
    }

    let conn = crate::db::open().await?;
    let txn = conn.begin().await?;

    if !super::delete_parameter_named(&txn, &request.name).await? {
        return Ok(crate::server::err::builder::parameter_not_found());
    }

    txn.commit().await?;

    Ok(super::empty_json_response())
}
