use chrono::{DateTime, TimeDelta, Utc};
use std::str::FromStr;

/// A strictly positive relative time window, parsed once at the CLI boundary.
#[derive(Debug, Clone)]
pub struct Lookback(TimeDelta);

impl Lookback {
    pub fn start_at(&self, now: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
        now.checked_sub_signed(self.0)
            .ok_or_else(|| "duration is out of range".into())
    }
}

impl FromStr for Lookback {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        let duration = duration_str::parse_chrono(s)?;
        if duration <= TimeDelta::zero() {
            return Err("--last must be positive".into());
        }
        Ok(Self(duration))
    }
}
