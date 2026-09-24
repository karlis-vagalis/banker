use crate::api::EnableBankingClient;
use crate::api::models::EnableBankingAccountId;
use crate::api::openapi::types::Transaction;
use crate::cli::MetadataAction;
use crate::config::{ApplicationConfig, Config};
use crate::db::row::TransactionRow;
use crate::error::AppError;
use crate::models::{ResourceType, TimeFrame, TransactionId};
use crate::output::{OutputFormat, print_api_transactions, print_db_transactions};

pub async fn fetch(
    config: &Config,
    app_cfg: &ApplicationConfig,
    account_id: &EnableBankingAccountId,
    time_frame: Option<&TimeFrame>,
    format: OutputFormat,
) -> Result<(), AppError> {
    let client = EnableBankingClient::new(config, app_cfg).await?;

    let txs = client
        .get_account_transactions(account_id, time_frame)
        .await?;
    print_api_transactions(&txs, format)?;
    Ok(())
}

pub async fn list_local(
    database: &crate::db::Database,
    bank_name: Option<&str>,
    bank_country: Option<&str>,
    format: OutputFormat,
) -> Result<(), AppError> {
    let rows: Vec<TransactionRow> = sqlx::query_as(
        "SELECT t.id, t.account_id, t.entry_reference, t.content, t.content_hash, t.inserted_at, t.updated_at
         FROM transactions t
         JOIN accounts a ON a.id = t.account_id
         WHERE (?1 IS NULL OR (a.aspsp_name = ?1 AND a.aspsp_country = ?2))
         ORDER BY json_extract(t.content, '$.booking_date') ASC",
    )
    .bind(bank_name)
    .bind(bank_country)
    .fetch_all(database.pool())
    .await?;

    if rows.is_empty() {
        tracing::warn!("no transactions in database — run `banker sync` first");
        return Ok(());
    }

    let txs: Vec<&Transaction> = rows.iter().map(|r| &r.content.0).collect();
    let ids: Vec<String> = rows.iter().map(|r| r.id.to_string()).collect();
    let account_ids: Vec<String> = rows.iter().map(|r| r.account_id.to_string()).collect();
    let updated_ats: Vec<chrono::DateTime<chrono::Utc>> =
        rows.iter().map(|r| r.updated_at).collect();
    print_db_transactions(&txs, &ids, &account_ids, &updated_ats, format)?;
    Ok(())
}

pub async fn metadata_local(
    database: &crate::db::Database,
    action: MetadataAction<TransactionId>,
) -> Result<(), AppError> {
    crate::commands::local::metadata_dispatch(database.pool(), &ResourceType::Transaction, action)
        .await?;
    Ok(())
}
