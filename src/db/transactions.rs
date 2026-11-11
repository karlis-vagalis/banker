use crate::api::openapi::types::Transaction;
use crate::models::{
    AccountId, TransactionId,
    transaction_query::{RowLimit, TransactionQuery},
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, QueryBuilder, Sqlite, SqlitePool, types::Json};

/// Read model: bank data and optional local annotations, fetched in one query.
#[derive(Debug, FromRow)]
pub struct TransactionRecord {
    pub id: TransactionId,
    pub account_id: AccountId,
    pub updated_at: DateTime<Utc>,
    pub content: Json<Transaction>,
    pub metadata: Option<Json<Value>>,
}

const SELECT: &str =
    "SELECT t.id, t.account_id, t.updated_at, t.content, m.user_metadata AS metadata
    FROM transactions t
    JOIN accounts a ON a.id = t.account_id
    LEFT JOIN transaction_metadata m ON m.transaction_id = t.id";

pub fn list_query(query: &TransactionQuery, bank: Option<(&str, &str)>) -> QueryBuilder<Sqlite> {
    let mut sql = QueryBuilder::new(SELECT);
    sql.push(" WHERE 1=1");
    if let Some((name, country)) = bank {
        sql.push(" AND a.aspsp_name = ").push_bind(name.to_owned());
        sql.push(" AND a.aspsp_country = ")
            .push_bind(country.to_owned());
    }
    if !query.accounts.is_empty() {
        sql.push(" AND t.account_id IN (");
        let mut values = sql.separated(", ");
        for account in &query.accounts {
            values.push_bind(account.clone());
        }
        sql.push(")");
    }
    if let Some(from) = query.dates.from() {
        sql.push(" AND json_extract(t.content, '$.booking_date') >= ")
            .push_bind(from.to_string());
    }
    if let Some(to) = query.dates.to() {
        sql.push(" AND json_extract(t.content, '$.booking_date') < ")
            .push_bind(to.to_string());
    }
    if let Some(search) = &query.search {
        // instr, unlike LIKE, treats %, _ and backslashes literally. SQLite lower is ASCII-only.
        sql.push(" AND (instr(lower(COALESCE((SELECT group_concat(value, '. ') FROM json_each(t.content, '$.remittance_information')), '')), lower(")
            .push_bind(search.clone()).push(")) > 0 OR instr(lower(COALESCE(CASE json_extract(t.content, '$.credit_debit_indicator') WHEN 'CRDT' THEN json_extract(t.content, '$.debtor.name') WHEN 'DBIT' THEN json_extract(t.content, '$.creditor.name') END, '')), lower(")
            .push_bind(search.clone()).push(")) > 0)");
    }
    // SQLite DESC puts NULL after dated transactions, and id breaks date ties.
    sql.push(" ORDER BY json_extract(t.content, '$.booking_date') DESC, t.id DESC");
    if let RowLimit::Bounded(limit) = query.limit {
        sql.push(" LIMIT ").push_bind(i64::from(limit.get()) + 1);
    }
    sql
}

pub async fn find(
    pool: &SqlitePool,
    id: TransactionId,
) -> Result<Option<TransactionRecord>, sqlx::Error> {
    QueryBuilder::<Sqlite>::new(SELECT)
        .push(" WHERE t.id = ")
        .push_bind(id)
        .build_query_as()
        .fetch_optional(pool)
        .await
}
