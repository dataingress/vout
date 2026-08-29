use sea_orm::{EntityTrait, Schema};
use sea_orm_migration::prelude::*;

use crate::models::{tb_account, tb_audit, tb_setting};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_entity(manager, tb_setting::Entity).await?;
        create_entity(manager, tb_account::Entity).await?;
        create_entity(manager, tb_audit::Entity).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_entity(manager, tb_audit::Entity).await?;
        drop_entity(manager, tb_account::Entity).await?;
        drop_entity(manager, tb_setting::Entity).await?;

        Ok(())
    }
}

async fn create_entity<E>(manager: &SchemaManager<'_>, entity: E) -> Result<(), DbErr>
where
    E: EntityTrait,
{
    let schema = Schema::new(manager.get_database_backend());
    let mut table = schema.create_table_from_entity(entity);

    table.if_not_exists();

    manager.create_table(table).await?;

    for mut index in schema.create_index_from_entity(entity) {
        index.if_not_exists();
        manager.create_index(index).await?;
    }

    Ok(())
}

async fn drop_entity<E>(manager: &SchemaManager<'_>, entity: E) -> Result<(), DbErr>
where
    E: EntityTrait,
{
    manager
        .drop_table(
            Table::drop()
                .table(entity.table_ref())
                .if_exists()
                .to_owned(),
        )
        .await
}
