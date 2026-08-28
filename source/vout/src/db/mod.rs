use sea_orm::{Database, DatabaseConnection};

pub mod schema;

pub async fn open() -> anyhow::Result<DatabaseConnection> {
    let config = crate::config::get();

    Ok(Database::connect(&config.db_dsn).await?)
}
