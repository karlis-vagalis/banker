use crate::api::EnableBankingClient;
use crate::api::models::EnableBankingAccountId;
use crate::api::openapi::types::AccountResource;
use crate::cli::MetadataAction;
use crate::config::{ApplicationConfig, Config};
use crate::db::{self, row::AccountRow};
use crate::error::AppError;
use crate::models::AccountId;
use crate::output::{print_api_accounts, print_db_accounts, OutputFormat};
use std::path::Path;

pub async fn fetch(
    config: &Config,
    app_cfg: &ApplicationConfig,
    account_id: &EnableBankingAccountId,
    format: OutputFormat,
) -> Result<(), AppError> {
    let client = EnableBankingClient::new(config, app_cfg).await?;
    let details = client.get_account_details(account_id).await?;
    print_api_accounts(&[details], format)?;
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

    let rows: Vec<AccountRow> = sqlx::query_as(
        "SELECT id, aspsp_name, aspsp_country, identification_hash, content, content_hash, inserted_at, updated_at
         FROM accounts
         WHERE (?1 IS NULL OR (aspsp_name = ?1 AND aspsp_country = ?2))
         ORDER BY updated_at DESC",
    )
    .bind(bank_name)
    .bind(bank_country)
    .fetch_all(&pool)
    .await?;

    if rows.is_empty() {
        tracing::warn!("no accounts in database — run `banker sync` first");
        return Ok(());
    }

    let accounts: Vec<&AccountResource> = rows.iter().map(|r| &r.content.0).collect();
    let ids: Vec<String> = rows.iter().map(|r| r.id.to_string()).collect();
    let updated_ats: Vec<chrono::DateTime<chrono::Utc>> =
        rows.iter().map(|r| r.updated_at).collect();
    print_db_accounts(&accounts, &ids, &updated_ats, format)?;
    Ok(())
}

pub async fn metadata_local(
    config_path: &Path,
    action: MetadataAction<AccountId>,
) -> Result<(), AppError> {
    let config = Config::load(config_path)?;
    let pool = db::init_pool(&config.db_path()).await?;
    crate::commands::local::metadata_dispatch(&pool, &crate::models::ResourceType::Account, action)
        .await?;
    Ok(())
}
