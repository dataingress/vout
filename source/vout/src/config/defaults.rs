pub fn default_listen_address() -> String {
    "127.0.0.1:4566".to_string()
}

pub fn default_concurrent() -> u32 {
    50
}

pub fn default_timeout() -> tokio::time::Duration {
    tokio::time::Duration::from_secs(30)
}

pub fn default_max_body_bytes() -> u64 {
    1024 * 1024
}

pub fn default_db_dsn() -> String {
    "sqlite::memory:".to_string()
}

pub fn default_amz_error_on_unsupported() -> bool {
    true
}
