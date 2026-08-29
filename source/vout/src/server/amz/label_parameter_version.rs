use macros::ApiUnsupportedFields;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, TransactionTrait};
use serde::Deserialize;

use crate::{deserialize_request_body, server::res::GwResponse};

#[derive(Deserialize, ApiUnsupportedFields)]
#[serde(rename_all = "PascalCase")]
struct Request {
    name: String,
    labels: Vec<String>,
    parameter_version: Option<i64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct LabelParameterVersionResponse {
    invalid_labels: Vec<String>,
    parameter_version: i64,
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
    let Some(param) = migration::models::tb_param::Entity::find_by_id(request.name.clone())
        .one(&txn)
        .await?
    else {
        return Ok(crate::server::err::builder::parameter_not_found());
    };
    let parameter_version = request.parameter_version.unwrap_or(param.version);

    if migration::models::tb_param_version::Entity::find_by_id((
        request.name.clone(),
        parameter_version,
    ))
    .one(&txn)
    .await?
    .is_none()
    {
        return Ok(crate::server::err::builder::parameter_version_not_found());
    }

    let mut invalid_labels = Vec::new();
    let mut valid_labels = Vec::new();

    for label in request.labels {
        if super::valid_label(&label) {
            valid_labels.push(label);
        } else {
            invalid_labels.push(label);
        }
    }

    valid_labels.sort();
    valid_labels.dedup();
    invalid_labels.sort();
    invalid_labels.dedup();

    let mut labels_on_version = super::labels_for_version(&txn, &request.name, parameter_version)
        .await?
        .into_iter()
        .filter(|label| !valid_labels.iter().any(|new_label| new_label == label))
        .collect::<Vec<_>>();
    labels_on_version.extend(valid_labels.iter().cloned());
    labels_on_version.sort();
    labels_on_version.dedup();

    if labels_on_version.len() > super::label_limit() {
        return Ok(crate::server::err::builder::parameter_version_label_limit_exceeded());
    }

    for label in &valid_labels {
        if let Some(existing) = migration::models::tb_param_label::Entity::find_by_id((
            request.name.clone(),
            label.clone(),
        ))
        .one(&txn)
        .await?
        {
            let mut active: migration::models::tb_param_label::ActiveModel = existing.into();
            active.version = Set(parameter_version);
            active.update(&txn).await?;
        } else {
            migration::models::tb_param_label::Entity::insert(
                migration::models::tb_param_label::ActiveModel {
                    param_key: Set(request.name.clone()),
                    label: Set(label.clone()),
                    version: Set(parameter_version),
                },
            )
            .exec(&txn)
            .await?;
        }
    }

    txn.commit().await?;

    super::json_response(&LabelParameterVersionResponse {
        invalid_labels,
        parameter_version,
    })
}
