use crate::{
    app::{account, settings},
    config, db, outputln,
};
use sea_orm::SqlErr;

pub async fn stage1() -> anyhow::Result<()> {
    outputln!("starting", stage: 1);

    db::migration::run().await?;

    if settings::db_key_exists().await? {
        return Ok(());
    }

    let config = config::get();

    settings::populate().await?;

    if let Some(ref user) = config.first_user {
        let result = account::create_account(Some(account::CreateAccountParam {
            access_key: user.access_key.to_owned(),
            secret_key: user.secret_key.to_owned(),
            lifetime: user.lifetime.map(|v| tokio::time::Duration::from_mins(v)),
        }))
        .await;

        if let Some(err) = db::erro::handle(&result) {
            if matches!(err, SqlErr::UniqueConstraintViolation(_)) {
                outputln!("skipping user creation due to unique constraint violation", error: "access key already exists");
            } else {
                anyhow::bail!(err);
            }
        } else {
            result?;

            outputln!("first user created", access_key: user.access_key);
        }
    } else {
        outputln!("no existing valid user found, creating a new one");

        let result = account::create_account(None).await;

        if let Some(err) = db::erro::handle(&result) {
            if matches!(err, SqlErr::UniqueConstraintViolation(_)) {
                outputln!("skipping user creation due to unique constraint violation", error: "access key already exists");
            } else {
                anyhow::bail!(err);
            }
        } else {
            let result = result?;

            outputln!("first user created", access_key: result.access_key, secret_key: result.secret_key);
        }
    }

    Ok(())
}
