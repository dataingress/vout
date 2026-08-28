use sea_orm_migration::{prelude::*, schema::*, sea_query::Keyword::CurrentTimestamp};

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn up_settings<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table("tb_app")
                .if_not_exists()
                .col(string("tb_app_key").primary_key())
                .col(string("tb_app_value"))
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_tb_app_tb_app_key")
                .table("tb_app")
                .col("tb_app_key")
                .to_owned(),
        )
        .await?;

    Ok(())
}

async fn down_settings<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .drop_table(Table::drop().table("tb_app").to_owned())
        .await?;

    manager
        .drop_index(
            Index::drop()
                .name("idx_tb_app_tb_app_key")
                .table("tb_app")
                .to_owned(),
        )
        .await?;

    Ok(())
}

async fn up_create_access<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table("tb_access")
                .if_not_exists()
                .col(pk_auto("tb_access_id"))
                .col(string("tb_access_access_key").not_null())
                .col(string("tb_access_secret_key").not_null())
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_tb_access_access_key")
                .table("tb_access")
                .col("tb_access_access_key")
                .to_owned(),
        )
        .await?;

    Ok(())
}

async fn down_create_access<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .drop_table(Table::drop().table("tb_access").to_owned())
        .await?;

    manager
        .drop_index(
            Index::drop()
                .name("idx_tb_access_access_key")
                .table("tb_access")
                .to_owned(),
        )
        .await?;

    Ok(())
}

async fn up_audit_logs<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table("tb_audit_logs")
                .if_not_exists()
                .col(pk_auto("tb_audit_logs_id"))
                .col(timestamp("tb_audit_logs_time").default(CurrentTimestamp))
                .col(string("tb_audit_logs_message").not_null())
                .col(string("tb_audit_logs_actor").not_null())
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_tb_audit_logs_time")
                .table("tb_audit_logs")
                .col("tb_audit_logs_time")
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_tb_audit_logs_actor")
                .table("tb_audit_logs")
                .col("tb_audit_logs_actor")
                .to_owned(),
        )
        .await?;

    Ok(())
}

async fn down_audit_logs<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .drop_table(Table::drop().table("tb_audit_logs").to_owned())
        .await?;

    manager
        .drop_index(
            Index::drop()
                .name("idx_tb_audit_logs_time")
                .table("tb_audit_logs")
                .to_owned(),
        )
        .await?;

    manager
        .drop_index(
            Index::drop()
                .name("idx_tb_audit_logs_actor")
                .table("tb_audit_logs")
                .to_owned(),
        )
        .await?;

    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        up_settings(manager).await?;
        up_create_access(manager).await?;
        up_audit_logs(manager).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        down_settings(manager).await?;
        down_create_access(manager).await?;
        down_audit_logs(manager).await?;

        Ok(())
    }
}
