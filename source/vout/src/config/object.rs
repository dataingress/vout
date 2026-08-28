use serde::Deserialize;

use crate::config::defaults::*;

#[derive(Deserialize, Clone)]
pub struct Root {
    pub first_run: Option<FirstRun>,
    pub server: Option<Server>,
    #[serde(default = "default_db_dsn")]
    pub db_dsn: String,
}

#[derive(Deserialize, Clone)]
pub struct FirstRunInit {
    pub access_key: String,
    pub secret_key: String,
    pub expiration: tokio::time::Duration,
}

#[derive(Deserialize, Clone)]
pub struct FirstRunUser {
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Deserialize, Clone)]
pub struct FirstRun {
    pub init: Option<FirstRunInit>,
    pub user: Option<FirstRunUser>,
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
}
