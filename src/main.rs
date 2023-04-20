use clap::Parser;

mod config;
mod cli;
mod modifiers;
mod error;

use cli::CliArgs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    Ok(())
}
