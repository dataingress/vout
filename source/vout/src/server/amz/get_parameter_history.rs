use macros::ApiUnsupportedFields;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Deserialize;

use crate::{deserialize_request_body, server::res::GwResponse};

fn default_request_with_decryption() -> bool {
    false
}

#[derive(Deserialize, ApiUnsupportedFields)]
#[serde(rename_all = "PascalCase")]
struct Request {
    name: String,
    max_results: Option<u64>,
    next_token: Option<String>,
    #[serde(default = "default_request_with_decryption")]
    with_decryption: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct GetParameterHistoryResponse {
    parameters: Vec<super::ParameterHistory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_token: Option<String>,
}

pub async fn handler<'a>(req: super::AmzRequest) -> GwResponse<'a> {
    let request = deserialize_request_body!(req, Request);

    if let Some(err) = super::validate_parameter_name(&request.name) {
        return Ok(err);
    }

    let conn = crate::db::open().await?;

    if migration::models::tb_param::Entity::find_by_id(request.name.clone())
        .one(&conn)
        .await?
        .is_none()
    {
        return Ok(crate::server::err::builder::parameter_not_found());
    }

    let versions = migration::models::tb_param_version::Entity::find()
        .filter(migration::models::tb_param_version::Column::ParamKey.eq(&request.name))
        .order_by_asc(migration::models::tb_param_version::Column::Version)
        .all(&conn)
        .await?;
    let (offset, limit) =
        match super::page_bounds(request.next_token.as_deref(), request.max_results, 50, 50) {
            Some(bounds) => bounds,
            None => return Ok(crate::server::err::builder::invalid_next_token()),
        };
    let (versions, next_token) = match super::take_page(versions, offset, limit) {
        Some(page) => page,
        None => return Ok(crate::server::err::builder::invalid_next_token()),
    };
    let mut parameters = Vec::with_capacity(versions.len());

    for version in versions {
        parameters
            .push(super::history_from_version(&conn, version, request.with_decryption).await?);
    }

    super::json_response(&GetParameterHistoryResponse {
        parameters,
        next_token,
    })
}
