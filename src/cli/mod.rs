mod metadata;
mod time_frame;
pub use metadata::{MetadataAction, SchemaAction};
pub use time_frame::{TimeFrameArgs, time_frame_from_args};

use crate::models::{AccountId, BalanceId, Resource, ResourceType, TransactionId};
use crate::output::OutputFormat;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct BankArg {
    /// Bank config name (required only when multiple banks are configured)
    #[arg(long)]
    pub bank: Option<String>,
}

/// Shared flag for table vs streaming output.
#[derive(Args, Clone)]
pub struct OutputArg {
    /// Output as line-delimited JSON (streaming) instead of a table
    #[arg(long, conflicts_with = "lines")]
    pub jsonl: bool,
    /// Output as pipe-separated plain text (one record per line, no headers, no truncation — grep-friendly)
    #[arg(long, conflicts_with = "jsonl")]
    pub lines: bool,
}

impl OutputArg {
    pub fn format(&self) -> OutputFormat {
        if self.jsonl {
            OutputFormat::Jsonl
        } else if self.lines {
            OutputFormat::Lines
        } else {
            OutputFormat::Table
        }
    }
}

/// Shared across commands that create an API client.
#[derive(Args, Clone)]
pub struct AppArg {
    /// EnableBanking application name (required only when multiple applications are configured)
    #[arg(long)]
    pub app: Option<String>,
}

#[derive(Parser)]
#[command(
    name = "banker",
    version,
    about = "Personal CLI for EnableBanking - authenticate with your linked banks and collect account data into a local SQLite database"
)]
pub struct Cli {
    /// Path to config file (default: ~/.config/banker/config.toml)
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage authentication with EnableBanking (login/logout/status)
    Auth {
        #[command(flatten)]
        app: AppArg,
        #[command(subcommand)]
        action: AuthAction,
    },
    /// List banks (ASPSPs) available through your EnableBanking application
    Bank {
        /// Filter by ISO country code, e.g. DE
        #[arg(long)]
        country: Option<crate::api::openapi::types::Country>,
        /// Filter by (partial, case-insensitive) bank name
        #[arg(long)]
        search: Option<String>,
    },
    /// Fetch account data from the API for all sessions and store in the local database
    Sync {
        /// Only sync the named banks (repeatable). Sync all banks when omitted.
        #[arg(long)]
        bank: Option<Vec<String>>,
        /// How many days of transaction history to request
        #[command(flatten)]
        time_frame_args: Option<TimeFrameArgs>,
    },
    /// Work with bank accounts
    Account {
        #[command(subcommand)]
        action: AccountAction,
    },
    /// Work with account balances
    Balance {
        #[command(subcommand)]
        action: BalanceAction,
    },
    /// Work with account transactions
    Transaction {
        #[command(subcommand)]
        action: TransactionAction,
    },
    /// ID utilities for database
    Id {
        #[command(subcommand)]
        action: IdAction,
    },
    /// Print the CLI command tree or list
    #[command(name = "self")]
    CliSelf {
        #[command(subcommand)]
        action: SelfFormat,
    },
}

#[derive(Subcommand)]
pub enum AuthAction {
    /// Start a new bank authorization flow and store the resulting session
    Login {
        #[command(flatten)]
        target: BankArg,
        /// Number of days the access/session should remain valid
        #[arg(long, default_value_t = 90)]
        valid_days: i64,
        /// Don't try to auto-open the authorization URL in a browser
        #[arg(long)]
        no_browser: bool,
    },
    /// Revoke a remote session and remove it locally
    Logout {
        #[command(flatten)]
        target: BankArg,
    },
    /// Show details about all stored sessions
    Status,
}

#[derive(Subcommand)]
pub enum IdAction {
    Encode { kind: ResourceType, id: i64 },
    Decode { resource: Resource },
}

#[derive(Subcommand)]
pub enum AccountAction {
    /// Fetch a single account's details from the API (live)
    Fetch {
        #[command(flatten)]
        app: AppArg,
        #[command(flatten)]
        output: OutputArg,
        /// Account UID as returned by `account list`
        account_id: String,
    },
    /// List stored accounts from the database (local)
    #[command(visible_alias = "ls")]
    List {
        #[command(flatten)]
        bank: BankArg,
        #[command(flatten)]
        output: OutputArg,
    },
    /// Manage account metadata
    Metadata {
        #[command(subcommand)]
        action: MetadataAction<AccountId>,
    },
}

#[derive(Subcommand)]
pub enum BalanceAction {
    /// Fetch a single account's balances from the API (live)
    Fetch {
        #[command(flatten)]
        app: AppArg,
        #[command(flatten)]
        output: OutputArg,
        /// Account UID as returned by `account list`
        account_id: String,
    },
    /// List stored balances from the database (local)
    #[command(visible_alias = "ls")]
    List {
        #[command(flatten)]
        bank: BankArg,
        #[command(flatten)]
        output: OutputArg,
    },
    /// Show the latest balance for each account and type (local)
    Current {
        #[command(flatten)]
        bank: BankArg,
        #[command(flatten)]
        output: OutputArg,
    },
    /// Manage balance metadata
    Metadata {
        #[command(subcommand)]
        action: MetadataAction<BalanceId>,
    },
}

#[derive(Subcommand)]
pub enum TransactionAction {
    /// Fetch a single account's transactions from the API (live)
    Fetch {
        #[command(flatten)]
        app: AppArg,
        #[command(flatten)]
        output: OutputArg,
        /// Account UID as returned by `account list`
        account_id: String,
        #[command(flatten)]
        time_frame_args: TimeFrameArgs,
    },
    /// List stored transactions from the database (local)
    #[command(visible_alias = "ls")]
    List {
        #[command(flatten)]
        bank: BankArg,
        #[command(flatten)]
        output: OutputArg,
    },
    /// Manage transaction metadata
    Metadata {
        #[command(subcommand)]
        action: MetadataAction<TransactionId>,
    },
}

#[derive(Subcommand)]
pub enum SelfFormat {
    /// Display the full command tree with tree-drawing characters
    Tree,
    /// Display commands as a flat list with full paths
    #[command(visible_alias = "ls")]
    List,
    /// Manage systemd user timer for periodic background sync
    Systemd {
        #[command(subcommand)]
        action: SystemdAction,
    },
}

#[derive(Subcommand)]
pub enum SystemdAction {
    /// Install the systemd service and timer for background sync
    Install,
    /// Remove the systemd service and timer
    Uninstall,
}
