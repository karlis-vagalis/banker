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
        pub struct $struct_name(i64);

        impl TryFrom<String> for $struct_name {
            type Error = &'static str;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                let (prefix, payload) = value.split_once('_').ok_or("Invalid ID format")?;
                if prefix != $prefix {
                    return Err("Invalid ID prefix");
                }
                let numbers = SQIDS.decode(payload);
                let [number] = numbers.as_slice() else {
                    return Err("ID must encode exactly one number");
                };
                let raw = i64::try_from(*number).map_err(|_| "ID exceeds SQLite integer range")?;
                if raw <= 0 {
                    return Err("ID must be positive");
                }
                if SQIDS.encode(&[*number]).as_deref() != Ok(payload) {
                    return Err("Non-canonical ID encoding");
                }
                Ok(Self(raw))
            }
        }

        impl std::str::FromStr for $struct_name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::try_from(s.to_string())
                    .map_err(|e| format!("Failed to parse {}: {}", stringify!($struct_name), e))
            }
        }

        impl TryFrom<i64> for $struct_name {
            type Error = &'static str;

            fn try_from(value: i64) -> Result<Self, Self::Error> {
                if value <= 0 {
                    return Err("ID must be positive");
                }
                Ok(Self(value))
            }
        }

        impl $struct_name {
            pub fn raw(&self) -> i64 {
                self.0
            }
        }

        impl std::fmt::Display for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let encoded = SQIDS
                    .encode(&[self.0 as u64])
                    .map_err(|_| std::fmt::Error)?;
                write!(f, "{}_{}", $prefix, encoded)
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
                <i64 as sqlx::Encode<'q, sqlx::Sqlite>>::encode_by_ref(&self.0, args)
            }
        }

        impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for $struct_name {
            fn decode(
                value: sqlx::sqlite::SqliteValueRef<'r>,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                let raw_id = <i64 as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;
                Ok($struct_name::try_from(raw_id)?)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
