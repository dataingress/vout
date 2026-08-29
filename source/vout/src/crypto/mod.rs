pub mod dbkey;
pub mod param;
pub mod rand;
pub mod tls;

pub fn init() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .ok();
}
