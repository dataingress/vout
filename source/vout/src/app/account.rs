use sea_orm::{ActiveValue::Set, EntityTrait};

mod generator {
    use crate::crypto;

    const ASCII: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    pub fn access_key() -> String {
        crypto::rand::random_bytes::<u8>(32)
            .iter()
            .map(|v| ASCII[*v as usize % ASCII.len()] as char)
            .collect()
    }

    pub fn secret_key() -> String {
        crypto::rand::random_bytes::<u8>(32)
            .iter()
            .map(|v| ASCII[*v as usize % ASCII.len()] as char)
            .collect()
    }
}

pub struct CreateAccountParam {
    pub access_key: String,
    pub secret_key: String,
    pub lifetime: Option<tokio::time::Duration>,
}

pub struct CreateAccountResult {
    pub access_key: String,
    pub secret_key: String,
    /// None if the account does not expire
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn create_account(
    param: Option<CreateAccountParam>,
) -> anyhow::Result<CreateAccountResult> {
    let created_at = chrono::Utc::now();
    let (access_key, secret_key, expires_at) = if let Some(param) = param {
        let expires = param
            .lifetime
            .map(|duration| chrono::Utc::now() + chrono::Duration::from_std(duration).unwrap());

        (param.access_key, param.secret_key, expires)
    } else {
        (generator::access_key(), generator::secret_key(), None)
    };

    let conn = crate::db::open().await?;

    migration::models::tb_account::Entity::insert(migration::models::tb_account::ActiveModel {
        access_key: Set(access_key.clone()),
        secret_key: Set(secret_key.clone()),
        created_at: Set(created_at),
        expires_at: Set(expires_at),
    })
    .exec(&conn)
    .await?;

    Ok(CreateAccountResult {
        access_key,
        secret_key,
        expires_at,
    })
}
