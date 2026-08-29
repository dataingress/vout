use std::sync::OnceLock;

use sea_orm::{ActiveValue::Set, EntityTrait, SqlErr};

use crate::{crypto, db, outputln};

pub const DB_KEY: &str = "db-key";

pub struct Settings {
    pub db_key: &'static [u8; 32],
}

static SETTINGS: OnceLock<Settings> = OnceLock::new();

pub fn get<'a>() -> &'a Settings {
    assert!(SETTINGS.get().is_some(), "settings not loaded but used");

    SETTINGS.get().unwrap()
}

async fn create_db_key() -> anyhow::Result<()> {
    outputln!("creating database key");

    let conn = db::open().await?;
    let key = crypto::dbkey::create()?;

    migration::models::tb_setting::Entity::insert(migration::models::tb_setting::ActiveModel {
        key: Set(DB_KEY.to_owned()),
        value: Set(key),
    })
    .exec(&conn)
    .await?;

    Ok(())
}

macro_rules! populate_wrapper {
    ($func:expr, $id:expr) => {{
        let result = $func().await;

        if let Some(err) = db::erro::handle(&result) {
            if matches!(err, SqlErr::UniqueConstraintViolation(_)) {
                outputln!(concat!("skipping ", $id, " creation due to unique constraint violation"), error: "access key already exists");
            } else {
                anyhow::bail!(err);
            }
        } else {
            result?;

            outputln!(concat!($id, " created successfully"));
        }
    }};
}

pub async fn populate() -> anyhow::Result<()> {
    outputln!("populating settings");

    populate_wrapper!(create_db_key, "db key");

    Ok(())
}

async fn load_db_key() -> anyhow::Result<&'static [u8; 32]> {
    Ok(Box::leak(Box::new(load_db_key_owned().await?)))
}

pub async fn load_db_key_owned() -> anyhow::Result<[u8; 32]> {
    outputln!("loading database key");

    let conn = db::open().await?;

    if let Some(setting) = migration::models::tb_setting::Entity::find_by_id(DB_KEY.to_owned())
        .one(&conn)
        .await?
    {
        let key = crypto::dbkey::load(&setting.value)?;

        Ok(key)
    } else {
        anyhow::bail!("database key not found");
    }
}

pub async fn load() -> anyhow::Result<()> {
    outputln!("loading settings");

    let db_key = load_db_key().await?;

    assert!(
        SETTINGS.set(Settings { db_key }).is_ok(),
        "failed to set settings"
    );

    Ok(())
}

pub async fn db_key_exists() -> anyhow::Result<bool> {
    let conn = db::open().await?;

    Ok(
        migration::models::tb_setting::Entity::find_by_id(DB_KEY.to_owned())
            .one(&conn)
            .await?
            .is_some(),
    )
}
