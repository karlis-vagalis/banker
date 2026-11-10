use super::time_frame::DateFilterArgs;
use crate::models::{
    AccountId,
    transaction_query::{BookingDateRange, FieldSelection, RowLimit, TransactionQuery},
};
use crate::output::OutputFormat;
use chrono::NaiveDate;
use clap::Args;
use std::num::NonZeroU32;

#[derive(Args)]
pub struct TransactionListArgs {
    /// Restrict to an account (repeat to include multiple accounts)
    #[arg(long = "account")]
    accounts: Vec<AccountId>,
    #[command(flatten)]
    dates: DateFilterArgs<NaiveDate>,
    /// Literal, ASCII-case-insensitive substring in description or counterparty
    #[arg(long)]
    search: Option<String>,
    /// Ordered comma-separated fields, including metadata.key paths
    #[arg(long)]
    fields: Option<FieldSelection>,
    /// Maximum records (default: 50, including for JSONL and lines)
    #[arg(long, conflicts_with = "all")]
    limit: Option<NonZeroU32>,
    /// Stream all matches; requires --jsonl or --lines
    #[arg(long, conflicts_with = "limit")]
    all: bool,
}

/// Constructed only by parsing CLI input: tables always have a finite row limit.
pub struct TransactionListRequest {
    query: TransactionQuery,
    fields: Option<FieldSelection>,
    format: OutputFormat,
}

impl TransactionListRequest {
    pub fn into_parts(self) -> (TransactionQuery, Option<FieldSelection>, OutputFormat) {
        (self.query, self.fields, self.format)
    }
}

impl TransactionListArgs {
    pub fn parse_request(self, format: OutputFormat) -> Result<TransactionListRequest, String> {
        if self.all && matches!(format, OutputFormat::Table) {
            return Err("--all requires --jsonl or --lines; tables must be bounded".into());
        }
        let dates = match self.dates.last {
            Some(last) => BookingDateRange::since(last)?,
            None => BookingDateRange::new(self.dates.from, self.dates.to)?,
        };
        let limit = if self.all {
            RowLimit::All
        } else {
            self.limit.map(RowLimit::Bounded).unwrap_or_default()
        };
        Ok(TransactionListRequest {
            query: TransactionQuery {
                accounts: self.accounts,
                dates,
                search: self.search,
                limit,
            },
            fields: self.fields,
            format,
        })
    }
}
