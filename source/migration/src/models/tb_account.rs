use sea_orm::entity::prelude::*;
use sea_orm_migration::sea_orm::{self, entity::prelude::ChronoDateTimeUtc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tb_account")]
pub struct Model {
    #[sea_orm(primary_key, unique)]
    pub access_key: String,
    pub secret_key: String,
    pub created_at: ChronoDateTimeUtc,
    pub expires_at: Option<ChronoDateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelatedEntity)]
pub enum RelatedEntity {}

impl ActiveModelBehavior for ActiveModel {}
