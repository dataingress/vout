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
    server::start().await?;

    Ok(())
}
