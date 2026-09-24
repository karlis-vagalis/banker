use chrono::{DateTime, Utc};
use sqids::Sqids;
use sqlx::Sqlite;
use sqlx::sqlite::{SqliteArgumentsBuffer, SqliteTypeInfo, SqliteValueRef};
use sqlx::{Decode, Encode, Type};
use std::sync::LazyLock;
use xxhash_rust::xxh3::xxh3_64;
pub struct TimeFrame {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

pub static SQIDS: LazyLock<Sqids> = LazyLock::new(|| {
    Sqids::builder()
        .alphabet(
            "k3G7QAe51FCsPW92uEOyq4Bg6Sp8YzVTmnU0liwDdHXLajZrfxNhobJIRcMvKt"
                .chars()
                .collect(),
        )
        .min_length(6)
        .build()
        .expect("Failed to initialize global Sqids builder")
});

macro_rules! define_id_type {
    ($struct_name:ident, $prefix:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $struct_name(pub String);

        impl TryFrom<String> for $struct_name {
            type Error = &'static str;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                let parts: Vec<&str> = value.split('_').collect();
                if parts.len() != 2 {
                    return Err("Invalid format");
                }
                if parts[0] != $prefix {
                    return Err("Invalid ID prefix");
                }

                // Ensure the sqid part actually decodes to a valid ID
                let numbers = SQIDS.decode(parts[1]);
                if numbers.is_empty() {
                    return Err("Malformed or invalid Sqid payload");
                }

                Ok(Self(value))
            }
        }

        impl std::str::FromStr for $struct_name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::try_from(s.to_string())
                    .map_err(|e| format!("Failed to parse {}: {}", stringify!($struct_name), e))
            }
        }

        impl From<u64> for $struct_name {
            fn from(value: u64) -> Self {
                let sqid_str = SQIDS.encode(&[value]).expect("Failed to encode ID");
                Self(format!("{}_{}", $prefix, sqid_str))
            }
        }

        impl From<i64> for $struct_name {
            fn from(value: i64) -> Self {
                let sqid_str = SQIDS.encode(&[value as u64]).expect("Failed to encode ID");
                Self(format!("{}_{}", $prefix, sqid_str))
            }
        }

        // Reusable extraction path (e.g., used by SQLx Encode)
        impl TryFrom<&$struct_name> for i64 {
            type Error = &'static str;

            fn try_from(id: &$struct_name) -> Result<Self, Self::Error> {
                let parts: Vec<&str> = id.0.split('_').collect();
                if parts.len() != 2 || parts[0] != $prefix {
                    return Err("Invalid or mismatched ID prefix");
                }

                let decoded = SQIDS.decode(parts[1]);
                let raw_id = decoded.first().copied().ok_or("Empty Sqid payload")?;
                Ok(raw_id as i64)
            }
        }

        impl std::fmt::Display for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl sqlx::Type<sqlx::Sqlite> for $struct_name {
            fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
                <i64 as sqlx::Type<sqlx::Sqlite>>::type_info()
            }
        }

        impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for $struct_name {
            fn encode_by_ref(
                &self,
                args: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                let raw_id = i64::try_from(self)?;
                <i64 as sqlx::Encode<'q, sqlx::Sqlite>>::encode_by_ref(&raw_id, args)
            }
        }

        impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for $struct_name {
            fn decode(
                value: sqlx::sqlite::SqliteValueRef<'r>,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                // Read the raw i64 from SQLite
                let raw_id = <i64 as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;

                // Fixed: Use From<i64> directly here since it handles formatting safely
                Ok($struct_name::from(raw_id))
            }
        }
    };
}

const ACCOUNT_ID_PREFIX: &str = "ac";
const BALANCE_ID_PREFIX: &str = "ba";
const TRANSACTION_ID_PREFIX: &str = "tx";

define_id_type!(AccountId, ACCOUNT_ID_PREFIX);
define_id_type!(BalanceId, BALANCE_ID_PREFIX);
define_id_type!(TransactionId, TRANSACTION_ID_PREFIX);

#[derive(Clone, clap::ValueEnum)]
pub enum ResourceType {
    Account,
    Balance,
    Transaction,
}

impl ResourceType {
    pub fn name(&self) -> &'static str {
        match self {
            ResourceType::Account => "account",
            ResourceType::Balance => "balance",
            ResourceType::Transaction => "transaction",
        }
    }
}

#[derive(Clone)]
pub enum Resource {
    Account(AccountId),
    Balance(BalanceId),
    Transaction(TransactionId),
}

impl std::str::FromStr for Resource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let prefix = s.split('_').next().unwrap_or("");
        match prefix {
            ACCOUNT_ID_PREFIX => Ok(Self::Account(std::str::FromStr::from_str(s)?)),
            BALANCE_ID_PREFIX => Ok(Self::Balance(std::str::FromStr::from_str(s)?)),
            TRANSACTION_ID_PREFIX => Ok(Self::Transaction(std::str::FromStr::from_str(s)?)),
            _ => Err(format!(
                "Unknown ID prefix '{}'. Must start with {ACCOUNT_ID_PREFIX}_, {BALANCE_ID_PREFIX}_, or {TRANSACTION_ID_PREFIX}_.",
                prefix
            )),
        }
    }
}

impl From<AccountId> for Resource {
    fn from(id: AccountId) -> Self {
        Resource::Account(id)
    }
}

impl From<BalanceId> for Resource {
    fn from(id: BalanceId) -> Self {
        Resource::Balance(id)
    }
}

impl From<TransactionId> for Resource {
    fn from(id: TransactionId) -> Self {
        Resource::Transaction(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentHash(pub u64);

impl ContentHash {
    pub fn new(data: &[u8]) -> Self {
        ContentHash(xxh3_64(data))
    }
}

impl Type<Sqlite> for ContentHash {
    fn type_info() -> SqliteTypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }
}

impl Encode<'_, Sqlite> for ContentHash {
    fn encode_by_ref(
        &self,
        args: &mut SqliteArgumentsBuffer,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        Encode::<Sqlite>::encode(self.0 as i64, args)
    }
}

impl<'r> Decode<'r, Sqlite> for ContentHash {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let raw = <i64 as Decode<Sqlite>>::decode(value)?;
        Ok(ContentHash(raw as u64))
    }
}
