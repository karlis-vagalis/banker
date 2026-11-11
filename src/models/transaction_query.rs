//! Parsed transaction query inputs. Constructors carry invariants into the query layer.
use super::{AccountId, Lookback};
use chrono::{NaiveDate, Utc};
use std::{num::NonZeroU32, str::FromStr};

#[derive(Debug, Clone)]
pub struct BookingDateRange {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

impl BookingDateRange {
    /// Half-open range: from is inclusive, to is exclusive.
    pub fn new(from: Option<NaiveDate>, to: Option<NaiveDate>) -> Result<Self, String> {
        if let (Some(from), Some(to)) = (from, to)
            && from >= to
        {
            return Err("--from must be before --to (exclusive)".into());
        }
        Ok(Self { from, to })
    }

    pub fn since(last: Lookback) -> Result<Self, String> {
        let now = Utc::now();
        let from = last.start_at(now)?;
        // Booking dates have day precision: include today for relative queries.
        Self::new(Some(from.date_naive()), now.date_naive().succ_opt())
    }

    pub fn from(&self) -> Option<NaiveDate> {
        self.from
    }
    pub fn to(&self) -> Option<NaiveDate> {
        self.to
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RowLimit {
    Bounded(NonZeroU32),
    All,
}

impl Default for RowLimit {
    fn default() -> Self {
        Self::Bounded(NonZeroU32::new(50).unwrap())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataPath(Vec<String>);

impl MetadataPath {
    pub fn segments(&self) -> &[String] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionField {
    Id,
    AccountId,
    Date,
    Amount,
    Currency,
    Counterparty,
    Description,
    UpdatedAt,
    Metadata,
    MetadataValue(MetadataPath),
}

impl TransactionField {
    pub fn name(&self) -> String {
        match self {
            Self::Id => "id",
            Self::AccountId => "account_id",
            Self::Date => "date",
            Self::Amount => "amount",
            Self::Currency => "currency",
            Self::Counterparty => "counterparty",
            Self::Description => "description",
            Self::UpdatedAt => "updated_at",
            Self::Metadata => "metadata",
            Self::MetadataValue(path) => return format!("metadata.{}", path.0.join(".")),
        }
        .into()
    }
}

impl FromStr for TransactionField {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Ok(match s {
            "id" => Self::Id,
            "account_id" => Self::AccountId,
            "date" => Self::Date,
            "amount" => Self::Amount,
            "currency" => Self::Currency,
            "counterparty" => Self::Counterparty,
            "description" => Self::Description,
            "updated_at" => Self::UpdatedAt,
            "metadata" => Self::Metadata,
            _ => {
                let path = s
                    .strip_prefix("metadata.")
                    .ok_or_else(|| format!("unknown transaction field: {s}"))?;
                let segments: Vec<String> = path.split('.').map(str::to_owned).collect();
                if segments.iter().any(|s| {
                    s.is_empty()
                        || !s
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                }) {
                    return Err("metadata paths require nonempty dot-separated keys (letters, digits, _ or -)".into());
                }
                Self::MetadataValue(MetadataPath(segments))
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct FieldSelection(Vec<TransactionField>);

impl FieldSelection {
    pub fn fields(&self) -> &[TransactionField] {
        &self.0
    }
}

impl Default for FieldSelection {
    fn default() -> Self {
        Self(vec![
            TransactionField::Id,
            TransactionField::Date,
            TransactionField::Amount,
            TransactionField::Currency,
            TransactionField::Counterparty,
            TransactionField::Description,
        ])
    }
}

impl FromStr for FieldSelection {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let fields = s
            .split(',')
            .map(str::parse)
            .collect::<Result<Vec<TransactionField>, _>>()?;
        for (index, field) in fields.iter().enumerate() {
            if fields[..index].contains(field) {
                return Err(format!("duplicate field: {}", field.name()));
            }
        }
        Ok(Self(fields))
    }
}

#[derive(Debug, Clone)]
pub struct TransactionQuery {
    pub accounts: Vec<AccountId>,
    pub dates: BookingDateRange,
    pub search: Option<String>,
    pub limit: RowLimit,
}
