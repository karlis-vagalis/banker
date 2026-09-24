mod accounts;
mod auth;
mod balance;
mod bank;
mod cli_self;
mod id;
mod local;
mod sync;
mod systemd;
mod transaction;

use crate::api::models::EnableBankingAccountId;
use crate::cli::{
    AccountAction, BalanceAction, Cli, Commands, SelfFormat, TransactionAction,
    time_frame_from_args,
};
use crate::config::{Config, default_config_path};
use std::future::Future;
use std::path::Path;

pub async fn dispatch(cli: Cli) -> anyhow::Result<()> {
    let config_path = cli.config.clone().unwrap_or_else(default_config_path);
    dispatch_inner(cli, &config_path).await
}

async fn with_database<T, E, F, Fut>(config_path: &Path, operation: F) -> anyhow::Result<T>
where
    E: Into<anyhow::Error>,
    F: FnOnce(Config, crate::db::Database) -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let config = Config::load(config_path)?;
    let database = crate::db::Database::open(&config.db_path()).await?;
    let result = operation(config, database.clone())
        .await
        .map_err(Into::into);
    let checkpoint_result = database.checkpoint().await;
    database.close().await;

    match (result, checkpoint_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(command_error), Ok(())) => Err(command_error),
        (Ok(_), Err(checkpoint_error)) => Err(checkpoint_error.into()),
        (Err(command_error), Err(checkpoint_error)) => {
            tracing::error!(
                "database checkpoint also failed during command shutdown: {checkpoint_error}"
            );
            Err(command_error)
        }
    }
}

async fn dispatch_inner(cli: Cli, config_path: &std::path::Path) -> anyhow::Result<()> {
    match cli.command {
        Commands::Auth { app, action } => {
            auth::run(config_path, app.app.as_deref(), action).await?
        }
        Commands::Bank { country, search } => {
            let config = Config::load(config_path)?;
            let (_name, app_cfg) = config.resolve_app(None)?;
            bank::run(&config, app_cfg, country, search).await?;
        }
        Commands::Sync {
            time_frame_args,
            bank,
        } => {
            let time_frame = time_frame_from_args(time_frame_args).map_err(anyhow::Error::msg)?;
            with_database(config_path, |config, database| async move {
                sync::run(&config, time_frame, bank, &database).await
            })
            .await?;
        }
        Commands::Transaction { action } => match action {
            TransactionAction::Fetch {
                app,
                output,
                account_id,
                time_frame_args,
            } => {
                let config = Config::load(config_path)?;
                let (_name, app_cfg) = config.resolve_app(app.app.as_deref())?;
                let time_frame =
                    time_frame_from_args(Some(time_frame_args)).map_err(anyhow::Error::msg)?;
                let id = EnableBankingAccountId::try_from(account_id)?;
                transaction::fetch(&config, app_cfg, &id, time_frame.as_ref(), output.format())
                    .await?;
            }
            TransactionAction::List { bank, output } => {
                with_database(config_path, |config, database| async move {
                    let bank_filter = local::resolve_filter_bank(&config, bank.bank.as_deref())?;
                    transaction::list_local(
                        &database,
                        bank_filter.as_ref().map(|(n, _)| n.as_str()),
                        bank_filter.as_ref().map(|(_, c)| c.as_str()),
                        output.format(),
                    )
                    .await
                })
                .await?;
            }
            TransactionAction::Metadata { action } => {
                with_database(config_path, |_, database| async move {
                    transaction::metadata_local(&database, action).await
                })
                .await?;
            }
        },
        Commands::Account { action } => match action {
            AccountAction::Fetch {
                app,
                output,
                account_id,
            } => {
                let config = Config::load(config_path)?;
                let (_name, app_cfg) = config.resolve_app(app.app.as_deref())?;
                let id = EnableBankingAccountId::try_from(account_id)?;
                accounts::fetch(&config, app_cfg, &id, output.format()).await?;
            }
            AccountAction::List { bank, output } => {
                with_database(config_path, |config, database| async move {
                    let bank_filter = local::resolve_filter_bank(&config, bank.bank.as_deref())?;
                    accounts::list_local(
                        &database,
                        bank_filter.as_ref().map(|(n, _)| n.as_str()),
                        bank_filter.as_ref().map(|(_, c)| c.as_str()),
                        output.format(),
                    )
                    .await
                })
                .await?;
            }
            AccountAction::Metadata { action } => {
                with_database(config_path, |_, database| async move {
                    accounts::metadata_local(&database, action).await
                })
                .await?;
            }
        },
        Commands::Balance { action } => match action {
            BalanceAction::Fetch {
                app,
                output,
                account_id,
            } => {
                let config = Config::load(config_path)?;
                let (_name, app_cfg) = config.resolve_app(app.app.as_deref())?;

                let id = EnableBankingAccountId::try_from(account_id)?;
                balance::fetch(&config, app_cfg, &id, output.format()).await?;
            }
            BalanceAction::List { bank, output } => {
                with_database(config_path, |config, database| async move {
                    let bank_filter = local::resolve_filter_bank(&config, bank.bank.as_deref())?;
                    balance::list_local(
                        &database,
                        bank_filter.as_ref().map(|(n, _)| n.as_str()),
                        bank_filter.as_ref().map(|(_, c)| c.as_str()),
                        output.format(),
                    )
                    .await
                })
                .await?;
            }
            BalanceAction::Current { bank, output } => {
                with_database(config_path, |config, database| async move {
                    let bank_filter = local::resolve_filter_bank(&config, bank.bank.as_deref())?;
                    balance::current_local(
                        &database,
                        bank_filter.as_ref().map(|(n, _)| n.as_str()),
                        bank_filter.as_ref().map(|(_, c)| c.as_str()),
                        output.format(),
                    )
                    .await
                })
                .await?;
            }
            BalanceAction::Metadata { action } => {
                with_database(config_path, |_, database| async move {
                    balance::metadata_local(&database, action).await
                })
                .await?;
            }
        },
        Commands::Id { action } => id::run(action).await?,
        Commands::CliSelf { action } => match action {
            SelfFormat::Tree | SelfFormat::List => cli_self::run(action).await?,
            SelfFormat::Systemd { action } => systemd::run(action).await?,
        },
    }
    Ok(())
}
