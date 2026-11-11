use crate::api::EnableBankingClient;
use crate::api::models::EnableBankingAccountId;
use crate::cli::{MetadataAction, TransactionListRequest};
use crate::config::{ApplicationConfig, Config};
use crate::error::AppError;
use crate::models::{ResourceType, TimeFrame, TransactionId, transaction_query::RowLimit};
use crate::output::{OutputFormat, TransactionRenderer, print_api_transactions, write_detail};
use futures_util::TryStreamExt;

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
    bank: Option<(&str, &str)>,
    request: TransactionListRequest,
) -> Result<(), AppError> {
    let stdout = std::io::stdout();
    let writer = std::io::BufWriter::new(stdout.lock());
    let (query, fields, format) = request.into_parts();
    let mut renderer = TransactionRenderer::new(writer, format, fields);
    let mut sql = crate::db::transactions::list_query(&query, bank);
    let mut rows = sql.build_query_as().fetch(database.pool());
    let mut count = 0u64;
    let mut more = false;
    let result = async {
        while let Some(row) = rows.try_next().await? {
            if let RowLimit::Bounded(limit) = query.limit
                && count == u64::from(limit.get())
            {
                more = true;
                break;
            }
            renderer.write_record(&row)?;
            count += 1;
        }
        renderer.finish()?;
        Ok::<_, AppError>(())
    }
    .await;
    match result {
        Err(AppError::Io(e)) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(error),
        Ok(()) => {
            if more {
                eprintln!(
                    "Showing {count} records; more results available. Increase --limit or use --all with --jsonl/--lines."
                );
            }
            Ok(())
        }
    }
}

pub async fn show_local(
    database: &crate::db::Database,
    id: TransactionId,
    format: OutputFormat,
) -> Result<(), AppError> {
    let label = id.to_string();
    let row = crate::db::transactions::find(database.pool(), id)
        .await?
        .ok_or_else(|| AppError::Other(format!("transaction not found: {label}")))?;
    let stdout = std::io::stdout();
    let mut writer = std::io::BufWriter::new(stdout.lock());
    match write_detail(&mut writer, &row, format) {
        Err(AppError::Io(e)) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}

pub async fn metadata_local(
    database: &crate::db::Database,
    action: MetadataAction<TransactionId>,
) -> Result<(), AppError> {
    crate::commands::local::metadata_dispatch(database.pool(), &ResourceType::Transaction, action)
        .await?;
    Ok(())
}
