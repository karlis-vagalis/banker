//! Run with `cargo run --example hello`: seed synthetic data and show real CLI output.
#[path = "../tests/support/seed.rs"]
#[allow(dead_code)] // The shared fixture also exposes persistence APIs for other consumers.
mod seed;

use anyhow::Result;
use banker::{cli::Cli, commands};
use clap::Parser;
use seed::{SeedSpec, seeded_database};
use std::{ffi::OsStr, path::Path};

async fn run(config: &Path, args: &[&str]) -> Result<()> {
    let cli = Cli::try_parse_from(
        [
            OsStr::new("banker"),
            OsStr::new("--config"),
            config.as_os_str(),
        ]
        .into_iter()
        .chain(args.iter().map(OsStr::new)),
    )?;
    commands::dispatch(cli).await
}

#[tokio::main]
async fn main() -> Result<()> {
    let fixture = seeded_database(SeedSpec::default()).await?;
    eprintln!(
        "Seeded {} synthetic transactions across 8 accounts and 4 banks.",
        fixture.transaction_count()
    );
    let config = fixture.config_path();
    let fields = "id,date,amount,currency,metadata.category,metadata.nested.weight";

    eprintln!("\nBirch: all 10 transactions, showing mixed currencies and annotations:");
    run(
        &config,
        &["transaction", "list", "--bank", "birch", "--fields", fields],
    )
    .await?;

    eprintln!("\nPine: the newest 5 of 10,000 transactions:");
    run(
        &config,
        &[
            "transaction",
            "list",
            "--bank",
            "pine",
            "--limit",
            "5",
            "--fields",
            fields,
        ],
    )
    .await?;

    // The fixture is dropped here, deleting the synthetic database and config.
    Ok(())
}
