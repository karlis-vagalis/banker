//! Create persistent synthetic data for exploring the real CLI by hand.
//! The test fixture generator is intentionally reused, not copied into production code.
#[path = "../tests/support/seed.rs"]
mod seed;

use anyhow::Result;
use clap::Parser;
use seed::{SeedSpec, seeded_database};
use std::io::Write;

#[derive(Parser)]
#[command(about = "Create a synthetic Banker database; stdout is its config path")]
struct Args {}

#[tokio::main]
async fn main() -> Result<()> {
    Args::parse();
    let fixture = seeded_database(SeedSpec::default()).await?;
    eprintln!(
        "Created {} synthetic transactions across {} fictional banks.",
        fixture.transaction_count(),
        fixture.banks.len()
    );
    eprintln!("Database: {}", fixture.db_path().display());
    for bank in &fixture.banks {
        for account in &bank.accounts {
            eprintln!(
                "  {:<6} {}  {}  {} transactions",
                bank.bank.key(),
                account.id,
                account.currency,
                account.transaction_count
            );
        }
    }
    eprintln!("\nCapture stdout in CONFIG, then try:");
    eprintln!("  cargo run -- --config \"$CONFIG\" account list");
    eprintln!("  cargo run -- --config \"$CONFIG\" transaction list --bank pine --limit 10");
    eprintln!(
        "  cargo run -- --config \"$CONFIG\" transaction list --bank pine --fields id,date,amount,currency,metadata.category,metadata.nested.weight"
    );
    eprintln!(
        "  cargo run -- --config \"$CONFIG\" transaction list --from 2025-02-01 --to 2025-03-01 --search household --jsonl"
    );
    eprintln!(
        "  cargo run -- --config \"$CONFIG\" transaction list --bank pine --all --jsonl > transactions.jsonl"
    );
    eprintln!("\nDates are fixed from 2025-01-01; recent --last windows may return no records.");
    eprintln!(
        "Local commands only: dummy credentials and a loopback API URL, never real bank data."
    );
    eprintln!(
        "Generated files persist. Remove this directory when finished: {}",
        fixture.directory().display()
    );
    let config = fixture.config_path();
    // Write the path successfully before relinquishing automatic cleanup.
    writeln!(std::io::stdout().lock(), "{}", config.display())?;
    fixture.persist();
    Ok(())
}
