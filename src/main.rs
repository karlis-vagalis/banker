mod api;
mod auth;
mod cli;
mod commands;
mod config;
mod db;
mod error;
mod models;
mod output;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::WARN)
        .init();

    let cli = Cli::parse();
    if let Err(err) = commands::dispatch(cli).await {
        tracing::error!("{err:#}");
        std::process::exit(1);
    }
}
