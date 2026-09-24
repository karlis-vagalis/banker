use crate::api::EnableBankingClient;
use crate::api::models::EnableBankingAccountId;
use crate::cli::MetadataAction;
use crate::config::{ApplicationConfig, Config};
use crate::db::row::BalanceRow;
use crate::error::AppError;
use crate::models::{BalanceId, ResourceType};
use crate::output::{OutputFormat, print_api_balances, print_db_balances};

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
    database: &crate::db::Database,
    bank_name: Option<&str>,
    bank_country: Option<&str>,
    format: OutputFormat,
) -> Result<(), AppError> {
    let rows: Vec<BalanceRow> = sqlx::query_as(
        "SELECT b.id, b.account_id, b.balance_type, b.content, b.inserted_at
         FROM balances b
         JOIN accounts a ON a.id = b.account_id
         WHERE (?1 IS NULL OR (a.aspsp_name = ?1 AND a.aspsp_country = ?2))
         ORDER BY b.inserted_at DESC",
    )
    .bind(bank_name)
    .bind(bank_country)
    .fetch_all(database.pool())
    .await?;

    if rows.is_empty() {
        tracing::warn!("no balances in database — run `banker sync` first");
        return Ok(());
    }

    print_db_balances(&rows, format)?;
    Ok(())
}

pub async fn current_local(
    database: &crate::db::Database,
    bank_name: Option<&str>,
    bank_country: Option<&str>,
    format: OutputFormat,
) -> Result<(), AppError> {
    let rows: Vec<BalanceRow> = sqlx::query_as(
        "SELECT b.id, b.account_id, b.balance_type, b.content, b.inserted_at
         FROM balances b
         JOIN accounts a ON a.id = b.account_id
         WHERE b.id = (
             SELECT b2.id FROM balances b2
             WHERE b2.account_id = b.account_id AND b2.balance_type = b.balance_type
             ORDER BY b2.inserted_at DESC, b2.id DESC LIMIT 1
         )
         AND (?1 IS NULL OR (a.aspsp_name = ?1 AND a.aspsp_country = ?2))
         ORDER BY a.id, b.balance_type",
    )
    .bind(bank_name)
    .bind(bank_country)
    .fetch_all(database.pool())
    .await?;

    if rows.is_empty() {
        tracing::warn!("no balances in database — run `banker sync` first");
        return Ok(());
    }

    print_db_balances(&rows, format)?;
    Ok(())
}

pub async fn metadata_local(
    database: &crate::db::Database,
    action: MetadataAction<BalanceId>,
) -> Result<(), AppError> {
    crate::commands::local::metadata_dispatch(database.pool(), &ResourceType::Balance, action)
        .await?;
    Ok(())
}
