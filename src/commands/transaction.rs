use crate::api::EnableBankingClient;
use crate::api::models::EnableBankingAccountId;
use crate::api::openapi::types::Transaction;
use crate::cli::MetadataAction;
use crate::config::{ApplicationConfig, Config};
use crate::db::{self, row::TransactionRow};
use crate::error::AppError;
use crate::models::{ResourceType, TimeFrame, TransactionId};
use crate::output::{print_api_transactions, print_db_transactions, OutputFormat};
use std::path::Path;

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
    config_path: &Path,
    bank_name: Option<&str>,
    bank_country: Option<&str>,
    format: OutputFormat,
) -> Result<(), AppError> {
    let config = Config::load(config_path)?;
    let pool = db::init_pool(&config.db_path()).await?;

    let rows: Vec<TransactionRow> = sqlx::query_as(
        "SELECT t.id, t.account_id, t.entry_reference, t.content, t.content_hash, t.inserted_at, t.updated_at
         FROM transactions t
         JOIN accounts a ON a.id = t.account_id
         WHERE (?1 IS NULL OR (a.aspsp_name = ?1 AND a.aspsp_country = ?2))
         ORDER BY json_extract(t.content, '$.booking_date') ASC",
    )
    .bind(bank_name)
    .bind(bank_country)
    .fetch_all(&pool)
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
    config_path: &Path,
    action: MetadataAction<TransactionId>,
) -> Result<(), AppError> {
    let config = Config::load(config_path)?;
    let pool = db::init_pool(&config.db_path()).await?;
    crate::commands::local::metadata_dispatch(&pool, &ResourceType::Transaction, action).await?;
    Ok(())
}
