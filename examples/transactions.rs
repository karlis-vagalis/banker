//! Explore real CLI output against synthetic data: `cargo run --example transactions -- list`.
#[path = "../tests/support/seed.rs"]
#[allow(dead_code)] // Other fixture consumers use its manifest and persistence APIs.
mod seed;

use anyhow::Result;
use banker::{
    cli::{
        AccountAction, BankArg, Cli, Commands, OutputArg, TransactionAction, TransactionListArgs,
    },
    commands,
    models::TransactionId,
};
use clap::{Parser, Subcommand};
use seed::{SeedSpec, seeded_database};
use std::num::NonZeroU8;

#[derive(Parser)]
#[command(about = "Explore transaction output using an isolated synthetic database")]
struct Args {
    /// Transactions in Pine (other banks have 10, 100, and 1,000)
    #[arg(long, global = true, default_value_t = 10_000)]
    pine_transactions: u32,
    /// Accounts per bank; transaction totals are split across them
    #[arg(long, global = true, default_value = "2")]
    accounts_per_bank: NonZeroU8,
    #[command(subcommand)]
    command: ExampleCommand,
}

#[derive(Subcommand)]
enum ExampleCommand {
    /// List transactions; all normal local filters, fields, and formats are available
    List {
        #[command(flatten)]
        bank: BankArg,
        #[command(flatten)]
        output: OutputArg,
        #[command(flatten)]
        query: TransactionListArgs,
    },
    /// Inspect an ID from list output (use the same dataset-size options)
    Show {
        id: TransactionId,
        #[command(flatten)]
        output: OutputArg,
    },
    /// List the seeded accounts and their IDs
    Accounts {
        #[command(flatten)]
        bank: BankArg,
        #[command(flatten)]
        output: OutputArg,
    },
}

impl From<ExampleCommand> for Commands {
    fn from(command: ExampleCommand) -> Self {
        match command {
            ExampleCommand::List {
                bank,
                output,
                query,
            } => Self::Transaction {
                action: TransactionAction::List {
                    bank,
                    output,
                    query,
                },
            },
            ExampleCommand::Show { id, output } => Self::Transaction {
                action: TransactionAction::Show { id, output },
            },
            ExampleCommand::Accounts { bank, output } => Self::Account {
                action: AccountAction::List { bank, output },
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let fixture = seeded_database(SeedSpec::tiers(
        args.accounts_per_bank,
        args.pine_transactions,
    ))
    .await?;
    eprintln!(
        "Using {} synthetic transactions across 4 banks; generated files are deleted on exit.",
        fixture.transaction_count()
    );
    commands::dispatch(Cli {
        config: Some(fixture.config_path()),
        command: args.command.into(),
    })
    .await
}
