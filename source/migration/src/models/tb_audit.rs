use sea_orm::entity::prelude::*;
use sea_orm_migration::sea_orm::{self, entity::prelude::ChronoDateTimeUtc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tb_audit")]
pub struct Model {
    #[sea_orm(primary_key, unique)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub timestamp: ChronoDateTimeUtc,
    pub message: String,
    #[sea_orm(indexed)]
    pub actor: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelatedEntity)]
pub enum RelatedEntity {}

impl ActiveModelBehavior for ActiveModel {}
