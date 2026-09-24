use crate::api::openapi::types::{AccountResource, BalanceResource, Transaction};
use crate::models::{AccountId, BalanceId, ContentHash, TransactionId};
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use sqlx::types::Json;

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct AccountRow {
    pub id: AccountId,
    pub aspsp_name: String,
    pub aspsp_country: String,
    pub identification_hash: Option<String>,
    pub content: Json<AccountResource>,
    pub content_hash: ContentHash,
    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct BalanceRow {
    pub id: BalanceId,
    pub account_id: AccountId,
    pub balance_type: String,
    pub content: Json<BalanceResource>,
    pub inserted_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct TransactionRow {
    pub id: TransactionId,
    pub account_id: AccountId,
    pub entry_reference: Option<String>,
    pub content: Json<Transaction>,
    pub content_hash: ContentHash,
    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
