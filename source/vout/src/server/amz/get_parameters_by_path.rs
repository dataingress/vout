use macros::ApiUnsupportedFields;
use sea_orm::{EntityTrait, QueryOrder};
use serde::Deserialize;

use crate::{deserialize_request_body, server::res::GwResponse};

fn default_request_recursive() -> bool {
    false
}

fn default_request_with_decryption() -> bool {
    false
}

#[derive(Deserialize, ApiUnsupportedFields)]
#[serde(rename_all = "PascalCase")]
struct Request {
    path: String,
    max_results: Option<u64>,
    next_token: Option<String>,
    parameter_filters: Option<Vec<super::ParameterStringFilter>>,
    #[serde(default = "default_request_recursive")]
    recursive: bool,
    #[serde(default = "default_request_with_decryption")]
    with_decryption: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct GetParametersByPathResponse {
    parameters: Vec<super::Parameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_token: Option<String>,
}

async fn matches_filter(
    conn: &sea_orm::DatabaseConnection,
    version: &migration::models::tb_param_version::Model,
    filter: &super::ParameterStringFilter,
) -> anyhow::Result<Result<bool, crate::server::res::GwPassed<'static>>> {
    let option = filter.option.as_deref().unwrap_or("Equals");
    let values = filter.values.as_deref().unwrap_or(&[]);

    match filter.key.as_str() {
        "Type" => {
            if !matches!(option, "Equals" | "BeginsWith") {
                return Ok(Err(crate::server::err::builder::invalid_filter_option()));
            }

            if values.is_empty() {
                return Ok(Ok(true));
            }

            Ok(Ok(match option {
                "Equals" => values.iter().any(|value| value == &version.r#type),
                "BeginsWith" => values.iter().any(|value| version.r#type.starts_with(value)),
                _ => false,
            }))
        }
        "Label" => {
            if option != "Equals" {
                return Ok(Err(crate::server::err::builder::invalid_filter_option()));
            }

            for value in values {
                if super::parameter_version_has_label(
                    conn,
                    &version.param_key,
                    version.version,
                    value,
                )
                .await?
                {
                    return Ok(Ok(true));
                }
            }

            Ok(Ok(values.is_empty()))
        }
        "KeyId" => Ok(Ok(false)),
        _ => Ok(Err(crate::server::err::builder::invalid_filter_key())),
    }
}

pub async fn handler<'a>(req: super::AmzRequest) -> GwResponse<'a> {
    let request = deserialize_request_body!(req, Request);

    if let Some(err) = super::validate_parameter_name(&request.path) {
        return Ok(err);
    }

    if let Some(err) = super::validate_filters(request.parameter_filters.as_deref()) {
        return Ok(err);
    }

    let conn = crate::db::open().await?;
    let params = migration::models::tb_param::Entity::find()
        .order_by_asc(migration::models::tb_param::Column::Key)
        .all(&conn)
        .await?;
    let mut parameters = Vec::new();

    for param in params {
        if !super::path_matches(&param.key, &request.path, request.recursive) {
            continue;
        }

        let Some(version) = migration::models::tb_param_version::Entity::find_by_id((
            param.key.clone(),
            param.version,
        ))
        .one(&conn)
        .await?
        else {
            continue;
        };

        if let Some(filters) = &request.parameter_filters {
            let mut matched = true;

            for filter in filters {
                match matches_filter(&conn, &version, filter).await? {
                    Ok(true) => {}
                    Ok(false) => {
                        matched = false;
                        break;
                    }
                    Err(err) => return Ok(err),
                }
            }

            if !matched {
                continue;
            }
        }

        parameters.push(super::parameter_from_version(
            version,
            None,
            request.with_decryption,
        )?);
    }

    let (offset, limit) =
        match super::page_bounds(request.next_token.as_deref(), request.max_results, 10, 10) {
            Some(bounds) => bounds,
            None => return Ok(crate::server::err::builder::invalid_next_token()),
        };
    let (parameters, next_token) = match super::take_page(parameters, offset, limit) {
        Some(page) => page,
        None => return Ok(crate::server::err::builder::invalid_next_token()),
    };

    super::json_response(&GetParametersByPathResponse {
        parameters,
        next_token,
    })
}
