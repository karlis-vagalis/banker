//! Run the actual CLI against synthetic data: `cargo run --example demo -- transaction list`.
#[path = "../tests/support/seed.rs"]
#[allow(dead_code)] // Integration tests also use the fixture's manifest and directory APIs.
mod seed;

use anyhow::{Result, ensure};
use banker::{
    cli::{AccountAction, BalanceAction, Cli, Commands, TransactionAction},
    commands,
};
use clap::Parser;
use seed::{SeedSpec, seeded_database};

#[tokio::main]
async fn main() -> Result<()> {
    // Use the actual parser: no demo-specific flags, subcommands, or argument wrappers.
    let mut cli = Cli::parse();
    ensure!(
        cli.config.is_none(),
        "demo supplies an isolated config; omit --config"
    );
    ensure!(
        matches!(
            &cli.command,
            Commands::Transaction {
                action: TransactionAction::List { .. } | TransactionAction::Show { .. }
            } | Commands::Account {
                action: AccountAction::List { .. }
            } | Commands::Balance {
                action: BalanceAction::List { .. } | BalanceAction::Current { .. }
            }
        ),
        "demo supports local read commands: transaction list/show, account list, and balance list/current; use the normal binary for other operations"
    );
    let fixture = seeded_database(SeedSpec::default()).await?;
    eprintln!(
        "Demo: {} synthetic transactions across 4 banks; generated files are deleted on exit.",
        fixture.transaction_count()
    );
    cli.config = Some(fixture.config_path());
    commands::dispatch(cli).await
}
