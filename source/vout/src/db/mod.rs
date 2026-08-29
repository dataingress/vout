use sea_orm::{Database, DatabaseConnection};

pub mod erro;

pub mod migration {
    use migration::{Migrator, MigratorTrait};

    use crate::outputln;

    pub async fn run() -> anyhow::Result<()> {
        outputln!("running dbms migration");

        let conn = super::open().await?;

        Migrator::up(&conn, None).await?;

        Ok(())
    }
}

pub async fn open() -> anyhow::Result<DatabaseConnection> {
    let config = crate::config::get();

    Ok(Database::connect(&config.db_dsn).await?)
}
