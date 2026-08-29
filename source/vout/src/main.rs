pub mod app;
pub mod config;
pub mod crypto;
pub mod db;
pub mod logger;
pub mod server;

#[tokio::main]
async fn main() {
    if let Err(err) = app::run().await {
        criticalln!("Unexpected error",
            error: err
        );
    }
}
