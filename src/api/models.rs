use serde::{Deserialize, Serialize};

use crate::error::AppError;
use std::ops::Deref;

macro_rules! define_uuid_type {
    ($struct_name:ident, $type:expr) => {
        #[derive(PartialEq, Eq, Clone, Deserialize, Serialize, Debug)]
        pub struct $struct_name(pub uuid::Uuid);

        impl TryFrom<&str> for $struct_name {
            type Error = AppError;
            fn try_from(value: &str) -> Result<Self, Self::Error> {
                let id = uuid::Uuid::parse_str(value)
                    .map_err(|e| AppError::Other(format!("invalid {} id: {e}", $type)))?;
                Ok(Self(id))
            }
        }

        impl TryFrom<String> for $struct_name {
            type Error = AppError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::try_from(value.as_str())
            }
        }

        impl Deref for $struct_name {
            type Target = uuid::Uuid;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::fmt::Display for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

define_uuid_type!(EnableBankingAccountId, "account");
define_uuid_type!(EnableBankingSessionId, "session");
