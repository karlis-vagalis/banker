use crate::api::EnableBankingClient;
use crate::api::models::EnableBankingAccountId;
use crate::cli::MetadataAction;
use crate::config::{ApplicationConfig, Config};
use crate::db::row::AccountRow;
use crate::error::AppError;
use crate::models::AccountId;
use crate::output::{OutputFormat, print_api_accounts, print_db_accounts};

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
    database: &crate::db::Database,
    bank_name: Option<&str>,
    bank_country: Option<&str>,
    format: OutputFormat,
) -> Result<(), AppError> {
    let rows: Vec<AccountRow> = sqlx::query_as(
        "SELECT id, aspsp_name, aspsp_country, identification_hash, content, content_hash, inserted_at, updated_at
         FROM accounts
         WHERE (?1 IS NULL OR (aspsp_name = ?1 AND aspsp_country = ?2))
         ORDER BY updated_at DESC",
    )
    .bind(bank_name)
    .bind(bank_country)
    .fetch_all(database.pool())
    .await?;

    if rows.is_empty() {
        tracing::warn!("no accounts in database — run `banker sync` first");
        return Ok(());
    }

    print_db_accounts(&rows, format)?;
    Ok(())
}

pub async fn metadata_local(
    database: &crate::db::Database,
    action: MetadataAction<AccountId>,
) -> Result<(), AppError> {
    crate::commands::local::metadata_dispatch(
        database.pool(),
        &crate::models::ResourceType::Account,
        action,
    )
    .await?;
    Ok(())
}
