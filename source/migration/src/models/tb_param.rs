use sea_orm::entity::prelude::*;
use sea_orm_migration::sea_orm::{self, entity::prelude::ChronoDateTimeUtc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tb_param")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub key: String,
    pub r#type: String,
    pub description: Option<String>,
    pub data_type: Option<String>,
    pub allowed_pattern: Option<String>,
    #[sea_orm(indexed)]
    pub version: i64,
    #[sea_orm(indexed)]
    pub last_modified_date: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::tb_param_tag::Entity")]
    ParamTag,
    #[sea_orm(has_many = "super::tb_param_version::Entity")]
    ParamVersion,
    #[sea_orm(has_many = "super::tb_param_label::Entity")]
    ParamLabel,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelatedEntity)]
pub enum RelatedEntity {
    #[sea_orm(entity = "super::tb_param_tag::Entity")]
    ParamTag,
    #[sea_orm(entity = "super::tb_param_version::Entity")]
    ParamVersion,
    #[sea_orm(entity = "super::tb_param_label::Entity")]
    ParamLabel,
}

impl ActiveModelBehavior for ActiveModel {}
