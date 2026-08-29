use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tb_param_tag")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub param_key: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub tag_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tb_param::Entity",
        from = "Column::ParamKey",
        to = "super::tb_param::Column::Key"
    )]
    Param,
    #[sea_orm(
        belongs_to = "super::tb_tag::Entity",
        from = "Column::TagId",
        to = "super::tb_tag::Column::Id"
    )]
    Tag,
}

impl Related<super::tb_param::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Param.def()
    }
}

impl Related<super::tb_tag::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tag.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
