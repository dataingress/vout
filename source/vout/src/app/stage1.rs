use migration::{Migrator, MigratorTrait};

use crate::{db, outputln};

async fn migration() -> anyhow::Result<()> {
    outputln!("running migration");

    let conn = db::open().await?;
    Migrator::up(&conn, None).await?;

    Ok(())
}

pub async fn stage1() -> anyhow::Result<()> {
    outputln!("starting", stage: 1);

    migration().await?;

    Ok(())
}
