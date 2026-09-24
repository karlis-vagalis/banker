use crate::api::EnableBankingClient;
use crate::api::models::EnableBankingAccountId;
use crate::api::openapi::types::BalanceResource;
use crate::cli::MetadataAction;
use crate::config::{ApplicationConfig, Config};
use crate::db::{self, row::BalanceRow};
use crate::error::AppError;
use crate::models::{BalanceId, ResourceType};
use crate::output::{print_api_balances, print_db_balances, OutputFormat};
use std::path::Path;

pub async fn fetch(
    config: &Config,
    app_cfg: &ApplicationConfig,
    account_id: &EnableBankingAccountId,
    format: OutputFormat,
) -> Result<(), AppError> {
    let client = EnableBankingClient::new(config, app_cfg).await?;
    let balances = client.get_account_balances(account_id).await?;
    print_api_balances(&balances.balances, format)?;
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

    let rows: Vec<BalanceRow> = sqlx::query_as(
        "SELECT b.id, b.account_id, b.balance_type, b.content, b.inserted_at
         FROM balances b
         JOIN accounts a ON a.id = b.account_id
         WHERE (?1 IS NULL OR (a.aspsp_name = ?1 AND a.aspsp_country = ?2))
         ORDER BY b.inserted_at DESC",
    )
    .bind(bank_name)
    .bind(bank_country)
    .fetch_all(&pool)
    .await?;

    if rows.is_empty() {
        tracing::warn!("no balances in database — run `banker sync` first");
        return Ok(());
    }

    let balances: Vec<&BalanceResource> = rows.iter().map(|r| &r.content.0).collect();
    let ids: Vec<String> = rows.iter().map(|r| r.id.to_string()).collect();
    let account_ids: Vec<String> = rows.iter().map(|r| r.account_id.to_string()).collect();
    let inserted_ats: Vec<chrono::DateTime<chrono::Utc>> =
        rows.iter().map(|r| r.inserted_at).collect();
    print_db_balances(&balances, &ids, &account_ids, &inserted_ats, format)?;
    Ok(())
}

pub async fn current_local(
    config_path: &Path,
    bank_name: Option<&str>,
    bank_country: Option<&str>,
    format: OutputFormat,
) -> Result<(), AppError> {
    let config = Config::load(config_path)?;
    let pool = db::init_pool(&config.db_path()).await?;

    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct CurrentBalanceRow {
        id: crate::models::BalanceId,
        account_id: crate::models::AccountId,
        balance_type: String,
        content: sqlx::types::Json<BalanceResource>,
        inserted_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<CurrentBalanceRow> = sqlx::query_as(
        "SELECT b.id, b.account_id, b.balance_type, b.content, b.inserted_at
         FROM balances b
         JOIN (
             SELECT account_id, balance_type, MAX(inserted_at) AS max_ts
             FROM balances
             GROUP BY account_id, balance_type
         ) latest ON b.account_id = latest.account_id
                  AND b.balance_type = latest.balance_type
                  AND b.inserted_at = latest.max_ts
         JOIN accounts a ON a.id = b.account_id
         WHERE (?1 IS NULL OR (a.aspsp_name = ?1 AND a.aspsp_country = ?2))
         ORDER BY a.id, b.balance_type",
    )
    .bind(bank_name)
    .bind(bank_country)
    .fetch_all(&pool)
    .await?;

    if rows.is_empty() {
        tracing::warn!("no balances in database — run `banker sync` first");
        return Ok(());
    }

    let balances: Vec<&BalanceResource> = rows.iter().map(|r| &r.content.0).collect();
    let ids: Vec<String> = rows.iter().map(|r| r.id.to_string()).collect();
    let account_ids: Vec<String> = rows.iter().map(|r| r.account_id.to_string()).collect();
    let inserted_ats: Vec<chrono::DateTime<chrono::Utc>> =
        rows.iter().map(|r| r.inserted_at).collect();
    print_db_balances(&balances, &ids, &account_ids, &inserted_ats, format)?;
    Ok(())
}

pub async fn metadata_local(
    config_path: &Path,
    action: MetadataAction<BalanceId>,
) -> Result<(), AppError> {
    let config = Config::load(config_path)?;
    let pool = db::init_pool(&config.db_path()).await?;
    crate::commands::local::metadata_dispatch(&pool, &ResourceType::Balance, action).await?;
    Ok(())
}
