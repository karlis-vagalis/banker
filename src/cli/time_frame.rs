use crate::models::{Lookback, TimeFrame};
use chrono::{DateTime, NaiveDate, Utc};
use clap::Args;

/// Parsing and CLI policy for the date precision required by each consumer.
pub trait DateFilterValue: Clone + Send + Sync + 'static {
    const FROM_HELP: &'static str;
    const TO_HELP: &'static str;
    fn parse(s: &str) -> Result<Self, String>;
    fn required_peer(peer: &'static str) -> Option<&'static str>;
}

impl DateFilterValue for DateTime<Utc> {
    const FROM_HELP: &'static str = "From date (YYYY-MM-DD) or RFC 3339 timestamp";
    const TO_HELP: &'static str = "To date (YYYY-MM-DD) or RFC 3339 timestamp";

    fn parse(s: &str) -> Result<Self, String> {
        if let Ok(datetime) = s.parse::<DateTime<Utc>>() {
            return Ok(datetime);
        }
        if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            && let Some(naive_dt) = date.and_hms_opt(0, 0, 0)
        {
            return Ok(naive_dt.and_utc());
        }
        Err(format!(
            "Invalid timestamp '{s}'. Expected RFC 3339 or YYYY-MM-DD."
        ))
    }

    fn required_peer(peer: &'static str) -> Option<&'static str> {
        Some(peer)
    }
}

impl DateFilterValue for NaiveDate {
    const FROM_HELP: &'static str = "Inclusive booking date (YYYY-MM-DD)";
    const TO_HELP: &'static str = "Exclusive booking date (YYYY-MM-DD)";

    fn parse(s: &str) -> Result<Self, String> {
        s.parse::<NaiveDate>().map_err(|error| error.to_string())
    }

    fn required_peer(_: &'static str) -> Option<&'static str> {
        None
    }
}

/// Shared, flattenable date arguments. `--all` deliberately belongs to the consumer.
#[derive(Args, Clone)]
#[group(multiple = true)]
#[command(next_help_heading = "Time frame")]
pub struct DateFilterArgs<D: DateFilterValue> {
    #[arg(long, help = D::FROM_HELP, requires_all = D::required_peer("to"), value_parser = D::parse)]
    pub(super) from: Option<D>,
    #[arg(long, help = D::TO_HELP, requires_all = D::required_peer("from"), value_parser = D::parse)]
    pub(super) to: Option<D>,
    /// Relative window (e.g. 1d, 1month); local booking-date queries include today
    #[arg(long, conflicts_with_all = ["from", "to"])]
    pub(super) last: Option<Lookback>,
}

/// Live API time-frame selection; `--all` means fetch the entire history.
#[derive(Args, Clone)]
#[group(required = false, multiple = true)]
pub struct TimeFrameArgs {
    /// Fetch the entire history
    #[arg(long, conflicts_with_all = ["from", "to", "last"])]
    all: bool,
    #[command(flatten)]
    dates: DateFilterArgs<DateTime<Utc>>,
}

pub fn time_frame_from_args(args: TimeFrameArgs) -> Result<Option<TimeFrame>, String> {
    if args.all {
        return Ok(None);
    }
    let now = Utc::now();
    match args.dates {
        DateFilterArgs {
            last: Some(last), ..
        } => Ok(Some(TimeFrame {
            from: last.start_at(now)?,
            to: now,
        })),
        DateFilterArgs {
            from: Some(from),
            to: Some(to),
            ..
        } => {
            if from > to {
                return Err("--from must be on or before --to".into());
            }
            Ok(Some(TimeFrame { from, to }))
        }
        _ => Ok(None),
    }
}
