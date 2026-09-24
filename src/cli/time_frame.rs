use crate::models::TimeFrame;
use chrono::{DateTime, Duration, NaiveDate, TimeDelta, Utc};
use clap::Args;

fn parse_clap_duration(s: &str) -> Result<TimeDelta, String> {
    let duration = duration_str::parse_chrono(s)?;
    if duration <= TimeDelta::zero() {
        return Err("duration must be positive".into());
    }
    Ok(duration)
}

fn parse_utc_date(s: &str) -> Result<DateTime<Utc>, String> {
    if let Ok(datetime) = s.parse::<DateTime<Utc>>() {
        return Ok(datetime);
    }
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        && let Some(naive_dt) = date.and_hms_opt(0, 0, 0)
    {
        return Ok(naive_dt.and_utc());
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

pub fn time_frame_from_args(args: Option<TimeFrameArgs>) -> Result<Option<TimeFrame>, String> {
    let now = Utc::now();
    match args {
        Some(args) if args.all => Ok(None),
        Some(TimeFrameArgs {
            last: Some(last), ..
        }) => Ok(Some(TimeFrame {
            from: now
                .checked_sub_signed(last)
                .ok_or("duration is out of range")?,
            to: now,
        })),
        Some(TimeFrameArgs {
            from: Some(from),
            to: Some(to),
            ..
        }) => {
            if from > to {
                return Err("--from must be on or before --to".into());
            }
            Ok(Some(TimeFrame { from, to }))
        }
        _ => Ok(None),
    }
}
