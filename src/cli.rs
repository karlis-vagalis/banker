use crate::models::{AccountId, BalanceId, Resource, ResourceType, TimeFrame, TransactionId};
use crate::output::OutputFormat;
use chrono::{DateTime, Duration, NaiveDate, TimeDelta, Utc};
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
    #[arg(long)]
    pub jsonl: bool,
    /// Output as pipe-separated plain text (one record per line, no headers, no truncation — grep-friendly)
    #[arg(long)]
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

fn parse_clap_duration(s: &str) -> Result<chrono::TimeDelta, String> {
    duration_str::parse_chrono(s)
}

fn parse_utc_date(s: &str) -> Result<DateTime<Utc>, String> {
    if let Ok(datetime) = s.parse::<DateTime<Utc>>() {
        return Ok(datetime);
    }

    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        if let Some(naive_dt) = date.and_hms_opt(0, 0, 0) {
            return Ok(naive_dt.and_utc());
        }
    }

    Err(format!(
        "Invalid timestamp '{}'. Expected RFC 3339 / ISO timestamp (e.g., 2026-07-10T14:30:00Z) or a plain date (e.g., 2026-07-10).",
        s
    ))
}

/// Group of arguments used to specify time frame, from-to OR last x
#[derive(Args, Clone)]
#[group(required = false)]
#[command(next_help_heading = "Time frame")]
pub struct TimeFrameArgs {
    /// Fetch the entire history
    #[arg(long, conflicts_with_all = ["from", "to", "last"])]
    pub all: bool,

    /// From date in the format "YYYY-MM-DD" (e.g., 2026-07-10) or RFC 3339 / ISO 8601 (e.g., 2026-07-10T14:30:00Z)
    #[arg(long, requires = "to", value_parser = parse_utc_date)]
    pub from: Option<DateTime<Utc>>,
    /// To date in the format "YYYY-MM-DD" (e.g., 2026-07-10) or RFC 3339 / ISO 8601 (e.g., 2026-07-10T14:30:00Z)
    #[arg(long, requires = "from", value_parser = parse_utc_date)]
    pub to: Option<DateTime<Utc>>,

    /// Specify a relative duration from now (e.g., "1d", "1month")
    #[arg(long, conflicts_with_all = ["from", "to", "all"], value_parser = parse_clap_duration)]
    pub last: Option<Duration>,
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
        country: Option<String>,
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

use std::error::Error as StdError;
use std::str::FromStr;

#[derive(Subcommand)]
pub enum MetadataAction<T>
where
    T: FromStr + Clone + Send + Sync + 'static,
    <T as FromStr>::Err: Into<Box<dyn StdError + Send + Sync>>,
{
    /// Manage the global JSON Schema for metadata
    Schema {
        #[command(subcommand)]
        action: SchemaAction,
    },
    /// Attach metadata
    Set {
        /// Internal ID from the database
        id: T,
        /// Raw JSON metadata string, a file path, or "-" for stdin. Omit to pipe from stdin.
        metadata: Option<String>,
        /// Skip the confirmation prompt when overwriting existing metadata
        #[arg(short, long)]
        yes: bool,
    },
    /// Retrieve metadata
    Get {
        /// Internal ID from the database
        id: T,
    },
    /// Delete metadata
    Delete {
        /// Internal ID from the database
        id: T,
        /// Skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum SchemaAction {
    /// Save a JSON Schema to the database
    Set {
        /// JSON Schema string, a file path, or "-" for stdin. Omit to pipe from stdin.
        schema: Option<String>,
        /// Skip the confirmation prompt when overwriting an existing schema
        #[arg(short, long)]
        yes: bool,
    },
    /// Print the active JSON Schema
    Get,
    /// Delete the stored JSON Schema
    Delete {
        /// Skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,
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

pub fn time_frame_from_args(time_frame_args: Option<TimeFrameArgs>) -> Option<TimeFrame> {
    let now = Utc::now();
    match &time_frame_args {
        Some(args) if args.all => None,
        Some(args) => match args.last {
            Some(last) => Some(TimeFrame {
                from: now - last,
                to: now,
            }),
            None => {
                let (Some(from), Some(to)) = (args.from, args.to) else {
                    panic!()
                };
                Some(TimeFrame { from, to })
            }
        },
        None => Some(TimeFrame {
            from: now - TimeDelta::days(7),
            to: now,
        }),
    }
}
