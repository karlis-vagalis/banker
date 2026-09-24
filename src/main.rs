use banker::cli::Cli;
use banker::commands;
use clap::Parser;

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
