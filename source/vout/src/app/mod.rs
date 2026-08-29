use crate::{config, outputln, server};
use clap::Parser;

mod stage1;
mod stage2;

pub mod account;
pub mod settings;

use stage1::stage1;
use stage2::stage2;

#[derive(clap::Parser)]
struct Arguments {
    #[clap(short = 'c', long = "config", value_parser)]
    config: Option<String>,
}

struct ParsedArguments {
    config: Option<String>,
}

fn arguments_parser() -> ParsedArguments {
    let args = Arguments::parse();

    ParsedArguments {
        config: args.config,
    }
}

pub async fn run() -> anyhow::Result<()> {
    outputln!("starting",
        version: env!("CARGO_PKG_VERSION")
    );

    let arguments = arguments_parser();

    config::load(arguments.config)?;

    stage1().await?;
    stage2().await?;
    tokio::select! {
        result = server::start() => result?,
        _ = shutdown_signal() => {
            crate::coverage::flush();
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler");

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
