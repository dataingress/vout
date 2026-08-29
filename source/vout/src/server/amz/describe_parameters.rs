use macros::ApiUnsupportedFields;
use sea_orm::{EntityTrait, QueryOrder};
use serde::Deserialize;

use crate::{deserialize_request_body, server::res::GwResponse};

#[derive(Deserialize, ApiUnsupportedFields)]
#[serde(rename_all = "PascalCase")]
struct Request {
    max_results: Option<u64>,
    next_token: Option<String>,
    parameter_filters: Option<Vec<super::ParameterStringFilter>>,

    #[api_notsupported]
    filters: Option<serde_json::Value>,
    #[api_notsupported]
    shared: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct DescribeParametersResponse {
    parameters: Vec<super::ParameterMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_token: Option<String>,
}

fn string_matches(value: &str, option: &str, values: &[String]) -> bool {
    if values.is_empty() {
        return true;
    }

    match option {
        "Equals" => values.iter().any(|expected| value == expected),
        "BeginsWith" => values.iter().any(|expected| value.starts_with(expected)),
        "Contains" => values.iter().any(|expected| value.contains(expected)),
        _ => false,
    }
}

async fn matches_filter(
    conn: &sea_orm::DatabaseConnection,
    param: &migration::models::tb_param::Model,
    filter: &super::ParameterStringFilter,
) -> anyhow::Result<Result<bool, crate::server::res::GwPassed<'static>>> {
    let option = filter.option.as_deref().unwrap_or("Equals");
    let values = filter.values.as_deref().unwrap_or(&[]);

    match filter.key.as_str() {
        "Name" => {
            if !matches!(option, "Equals" | "BeginsWith" | "Contains") {
                return Ok(Err(crate::server::err::builder::invalid_filter_option()));
            }

            Ok(Ok(string_matches(&param.key, option, values)))
        }
        "Path" => {
            if !matches!(option, "Recursive" | "OneLevel") {
                return Ok(Err(crate::server::err::builder::invalid_filter_option()));
            }

            if values.len() != 1 {
                return Ok(Err(crate::server::err::builder::invalid_filter_value()));
            }

            Ok(Ok(super::path_matches(
                &param.key,
                &values[0],
                option == "Recursive",
            )))
        }
        "Type" => {
            if !matches!(option, "Equals" | "BeginsWith") {
                return Ok(Err(crate::server::err::builder::invalid_filter_option()));
            }

            Ok(Ok(string_matches(&param.r#type, option, values)))
        }
        "DataType" => {
            if !matches!(option, "Equals" | "BeginsWith") {
                return Ok(Err(crate::server::err::builder::invalid_filter_option()));
            }

            let data_type = param.data_type.as_deref().unwrap_or("text");
            Ok(Ok(string_matches(data_type, option, values)))
        }
        "Tier" => {
            if !matches!(option, "Equals" | "BeginsWith") {
                return Ok(Err(crate::server::err::builder::invalid_filter_option()));
            }

            Ok(Ok(string_matches("Standard", option, values)))
        }
        key if key.starts_with("tag:") => {
            if option != "Equals" {
                return Ok(Err(crate::server::err::builder::invalid_filter_option()));
            }

            Ok(Ok(super::parameter_has_tag(
                conn,
                &param.key,
                key.trim_start_matches("tag:"),
                values,
            )
            .await?))
        }
        _ => Ok(Err(crate::server::err::builder::invalid_filter_key())),
    }
}

pub async fn handler<'a>(req: super::AmzRequest) -> GwResponse<'a> {
    let request = deserialize_request_body!(req, Request);

    if let Some(err) = super::validate_filters(request.parameter_filters.as_deref()) {
        return Ok(err);
    }

    let conn = crate::db::open().await?;
    let params = migration::models::tb_param::Entity::find()
        .order_by_asc(migration::models::tb_param::Column::Key)
        .all(&conn)
        .await?;
    let mut parameters = Vec::new();

    'params: for param in params {
        if let Some(filters) = &request.parameter_filters {
            for filter in filters {
                match matches_filter(&conn, &param, filter).await? {
                    Ok(true) => {}
                    Ok(false) => continue 'params,
                    Err(err) => return Ok(err),
                }
            }
        }

        parameters.push(super::metadata_from_param(param));
    }

    let (offset, limit) =
        match super::page_bounds(request.next_token.as_deref(), request.max_results, 50, 50) {
            Some(bounds) => bounds,
            None => return Ok(crate::server::err::builder::invalid_next_token()),
        };
    let (parameters, next_token) = match super::take_page(parameters, offset, limit) {
        Some(page) => page,
        None => return Ok(crate::server::err::builder::invalid_next_token()),
    };

    super::json_response(&DescribeParametersResponse {
        parameters,
        next_token,
    })
}
