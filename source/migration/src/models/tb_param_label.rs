use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tb_param_label")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub param_key: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub label: String,
    #[sea_orm(indexed)]
    pub version: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tb_param::Entity",
        from = "Column::ParamKey",
        to = "super::tb_param::Column::Key"
    )]
    Param,
}

impl Related<super::tb_param::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Param.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
