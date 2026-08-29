use sea_orm::{Database, DatabaseConnection};
use tokio::sync::OnceCell;

static CONNECTION: OnceCell<DatabaseConnection> = OnceCell::const_new();

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
    let connection = CONNECTION
        .get_or_try_init(|| async {
            let config = crate::config::get();
            Ok::<DatabaseConnection, anyhow::Error>(Database::connect(&config.db_dsn).await?)
        })
        .await?;

    Ok(connection.clone())
}
