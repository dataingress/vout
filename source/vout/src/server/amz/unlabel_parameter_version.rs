use macros::ApiUnsupportedFields;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use serde::Deserialize;

use crate::{deserialize_request_body, server::res::GwResponse};

#[derive(Deserialize, ApiUnsupportedFields)]
#[serde(rename_all = "PascalCase")]
struct Request {
    name: String,
    labels: Vec<String>,
    parameter_version: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct UnlabelParameterVersionResponse {
    invalid_labels: Vec<String>,
    removed_labels: Vec<String>,
}

pub async fn handler<'a>(req: super::AmzRequest) -> GwResponse<'a> {
    let request = deserialize_request_body!(req, Request);

    if let Some(err) = super::validate_parameter_name(&request.name) {
        return Ok(err);
    }

    if let Some(err) = super::validate_labels(&request.labels) {
        return Ok(err);
    }

    let conn = crate::db::open().await?;
    let txn = conn.begin().await?;

    if migration::models::tb_param::Entity::find_by_id(request.name.clone())
        .one(&txn)
        .await?
        .is_none()
    {
        return Ok(crate::server::err::builder::parameter_not_found());
    }

    if migration::models::tb_param_version::Entity::find_by_id((
        request.name.clone(),
        request.parameter_version,
    ))
    .one(&txn)
    .await?
    .is_none()
    {
        return Ok(crate::server::err::builder::parameter_version_not_found());
    }

    let mut invalid_labels = Vec::new();
    let mut removed_labels = Vec::new();

    for label in request.labels {
        let existing = migration::models::tb_param_label::Entity::find()
            .filter(migration::models::tb_param_label::Column::ParamKey.eq(&request.name))
            .filter(migration::models::tb_param_label::Column::Label.eq(&label))
            .filter(
                migration::models::tb_param_label::Column::Version.eq(request.parameter_version),
            )
            .one(&txn)
            .await?;

        if existing.is_some() {
            migration::models::tb_param_label::Entity::delete_by_id((
                request.name.clone(),
                label.clone(),
            ))
            .exec(&txn)
            .await?;
            removed_labels.push(label);
        } else {
            invalid_labels.push(label);
        }
    }

    invalid_labels.sort();
    invalid_labels.dedup();
    removed_labels.sort();
    removed_labels.dedup();
    txn.commit().await?;

    super::json_response(&UnlabelParameterVersionResponse {
        invalid_labels,
        removed_labels,
    })
}
