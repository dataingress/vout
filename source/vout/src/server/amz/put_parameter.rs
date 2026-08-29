use macros::ApiUnsupportedFields;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use serde::Deserialize;

use crate::{deserialize_request_body, server::res::GwResponse};

fn default_request_overwrite() -> bool {
    false
}

#[derive(Deserialize, ApiUnsupportedFields)]
#[serde(rename_all = "PascalCase")]
struct Request {
    name: String,
    value: String,
    allowed_pattern: Option<String>,
    data_type: Option<String>,
    description: Option<String>,
    #[serde(default = "default_request_overwrite")]
    overwrite: bool,
    tags: Option<Vec<super::Tag>>,
    r#type: super::ParameterType,

    #[api_notsupported]
    key_id: Option<serde_json::Value>,
    #[api_notsupported]
    policies: Option<serde_json::Value>,
    #[api_notsupported]
    tier: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct PutParameterResponse {
    version: i64,
}

async fn replace_tags<C>(conn: &C, parameter_name: &str, tags: &[super::Tag]) -> anyhow::Result<()>
where
    C: sea_orm::ConnectionTrait,
{
    let old_links = migration::models::tb_param_tag::Entity::find()
        .filter(migration::models::tb_param_tag::Column::ParamKey.eq(parameter_name))
        .all(conn)
        .await?;
    let old_tag_ids = old_links.iter().map(|link| link.tag_id).collect::<Vec<_>>();

    migration::models::tb_param_tag::Entity::delete_many()
        .filter(migration::models::tb_param_tag::Column::ParamKey.eq(parameter_name))
        .exec(conn)
        .await?;

    for tag_id in old_tag_ids {
        let still_used = migration::models::tb_param_tag::Entity::find()
            .filter(migration::models::tb_param_tag::Column::TagId.eq(tag_id))
            .one(conn)
            .await?
            .is_some();

        if !still_used {
            migration::models::tb_tag::Entity::delete_by_id(tag_id)
                .exec(conn)
                .await?;
        }
    }

    for tag in tags {
        migration::models::tb_tag::Entity::insert(migration::models::tb_tag::ActiveModel {
            key: Set(tag.key.clone()),
            value: Set(tag.value.clone()),
            ..Default::default()
        })
        .exec_without_returning(conn)
        .await?;

        let tag_model = migration::models::tb_tag::Entity::find()
            .filter(migration::models::tb_tag::Column::Key.eq(&tag.key))
            .filter(migration::models::tb_tag::Column::Value.eq(&tag.value))
            .order_by_desc(migration::models::tb_tag::Column::Id)
            .one(conn)
            .await?
            .ok_or_else(|| anyhow::anyhow!("inserted tag was not found"))?;

        migration::models::tb_param_tag::Entity::insert(
            migration::models::tb_param_tag::ActiveModel {
                param_key: Set(parameter_name.to_owned()),
                tag_id: Set(tag_model.id),
            },
        )
        .exec(conn)
        .await?;
    }

    Ok(())
}

async fn insert_version<C>(
    conn: &C,
    request: &Request,
    value: Vec<u8>,
    version: i64,
    last_modified_date: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<()>
where
    C: sea_orm::ConnectionTrait,
{
    migration::models::tb_param_version::Entity::insert(
        migration::models::tb_param_version::ActiveModel {
            param_key: Set(request.name.clone()),
            version: Set(version),
            value: Set(value),
            r#type: Set(request.r#type.as_str().to_owned()),
            description: Set(request.description.clone()),
            data_type: Set(request.data_type.clone()),
            allowed_pattern: Set(request.allowed_pattern.clone()),
            last_modified_date: Set(last_modified_date),
        },
    )
    .exec(conn)
    .await?;

    Ok(())
}

pub async fn handler<'a>(req: super::AmzRequest) -> GwResponse<'a> {
    let request = deserialize_request_body!(req, Request);

    if let Some(err) = super::validate_put_request(
        &request.name,
        &request.value,
        request.description.as_deref(),
        request.allowed_pattern.as_deref(),
        request.tags.as_deref(),
    ) {
        return Ok(err);
    }

    if let Some(pattern) = &request.allowed_pattern {
        let regex = match regex::Regex::new(pattern) {
            Ok(regex) => regex,
            Err(_) => return Ok(crate::server::err::builder::invalid_allowed_pattern()),
        };

        if !regex.is_match(&request.value) {
            return Ok(crate::server::err::builder::invalid_allowed_pattern());
        }
    }

    let conn = crate::db::open().await?;
    let txn = conn.begin().await?;
    let last_modified_date = chrono::Utc::now();
    let existing = migration::models::tb_param::Entity::find_by_id(request.name.clone())
        .one(&txn)
        .await?;
    let version = if let Some(existing) = existing {
        if !request.overwrite {
            return Ok(crate::server::err::builder::parameter_already_exists());
        }

        let version = existing.version + 1;
        let value = super::encode_value(&request.value, &request.name, version, &request.r#type)?;
        let mut active: migration::models::tb_param::ActiveModel = existing.into();

        active.r#type = Set(request.r#type.as_str().to_owned());
        active.description = Set(request.description.clone());
        active.data_type = Set(request.data_type.clone());
        active.allowed_pattern = Set(request.allowed_pattern.clone());
        active.version = Set(version);
        active.last_modified_date = Set(last_modified_date);

        active.update(&txn).await?;
        insert_version(&txn, &request, value, version, last_modified_date).await?;

        version
    } else {
        let version = 1;
        let value = super::encode_value(&request.value, &request.name, version, &request.r#type)?;

        migration::models::tb_param::Entity::insert(migration::models::tb_param::ActiveModel {
            key: Set(request.name.clone()),
            r#type: Set(request.r#type.as_str().to_owned()),
            description: Set(request.description.clone()),
            data_type: Set(request.data_type.clone()),
            allowed_pattern: Set(request.allowed_pattern.clone()),
            version: Set(version),
            last_modified_date: Set(last_modified_date),
        })
        .exec(&txn)
        .await?;
        insert_version(&txn, &request, value, version, last_modified_date).await?;

        version
    };

    if let Some(tags) = &request.tags {
        replace_tags(&txn, &request.name, tags).await?;
    }

    txn.commit().await?;

    super::json_response(&PutParameterResponse { version })
}
