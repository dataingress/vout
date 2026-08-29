use serde::Deserialize;

use crate::config::defaults::*;

#[derive(Deserialize, Clone)]
pub struct Root {
    pub first_user: Option<FirstUser>,
    #[serde(default)]
    pub server: Server,
    #[serde(default = "default_db_dsn")]
    pub db_dsn: String,
    #[serde(default = "default_amz_error_on_unsupported")]
    pub amz_error_on_unsupported: bool,
}

#[derive(Deserialize, Clone)]
pub struct FirstUser {
    pub access_key: String,
    pub secret_key: String,
    pub lifetime: Option<u64>,
}

#[derive(Deserialize, Clone)]
pub struct ServerTls {
    pub cert_filename: String,
    pub key_filename: String,
}

#[derive(Deserialize, Clone)]
pub struct Server {
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    pub tls: Option<ServerTls>,
    #[serde(default = "default_concurrent")]
    pub concurrent: u32,
    #[serde(default = "default_timeout")]
    pub timeout: tokio::time::Duration,
    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: u64,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            listen_address: default_listen_address(),
            tls: None,
            concurrent: default_concurrent(),
            timeout: default_timeout(),
            max_body_bytes: default_max_body_bytes(),
        }
    }
}
