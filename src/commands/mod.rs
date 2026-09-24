mod accounts;
mod auth;
mod balance;
mod bank;
mod cli_self;
mod id;
mod systemd;
mod local;
mod sync;
mod transaction;

use crate::api::models::EnableBankingAccountId;
use crate::cli::{
    AccountAction, BalanceAction, Cli, Commands, SelfFormat, TransactionAction,
    time_frame_from_args,
};
use crate::config::{Config, default_config_path};

pub async fn dispatch(cli: Cli) -> anyhow::Result<()> {
    let config_path = cli.config.clone().unwrap_or_else(default_config_path);

    match cli.command {
        Commands::Auth { app, action } => {
            auth::run(&config_path, app.app.as_deref(), action).await?
        }
        Commands::Bank { country, search } => {
            let config = Config::load(&config_path)?;
            let (_name, app_cfg) = config.resolve_app(None)?;
            bank::run(&config, app_cfg, country, search).await?;
        }
        Commands::Sync {
            time_frame_args,
            bank,
        } => {
            let time_frame = time_frame_from_args(time_frame_args);
            sync::run(&config_path, time_frame, bank).await?
        }
        Commands::Transaction { action } => match action {
            TransactionAction::Fetch {
                app,
                output,
                account_id,
                time_frame_args,
            } => {
                let config = Config::load(&config_path)?;
                let (_name, app_cfg) = config.resolve_app(app.app.as_deref())?;
                let time_frame = time_frame_from_args(Some(time_frame_args));
                let id = EnableBankingAccountId::try_from(account_id)?;
                transaction::fetch(&config, app_cfg, &id, time_frame.as_ref(), output.format()).await?;
            }
            TransactionAction::List { bank, output } => {
                let bank_filter = local::resolve_filter_bank(&config_path, bank.bank.as_deref())?;
                transaction::list_local(
                    &config_path,
                    bank_filter.as_ref().map(|(n, _)| n.as_str()),
                    bank_filter.as_ref().map(|(_, c)| c.as_str()),
                    output.format(),
                )
                .await?;
            }
            TransactionAction::Metadata { action } => {
                transaction::metadata_local(&config_path, action).await?;
            }
        },
        Commands::Account { action } => match action {
            AccountAction::Fetch {
                app,
                output,
                account_id,
            } => {
                let config = Config::load(&config_path)?;
                let (_name, app_cfg) = config.resolve_app(app.app.as_deref())?;
                let id = EnableBankingAccountId::try_from(account_id)?;
                accounts::fetch(&config, app_cfg, &id, output.format()).await?;
            }
            AccountAction::List { bank, output } => {
                let bank_filter = local::resolve_filter_bank(&config_path, bank.bank.as_deref())?;
                accounts::list_local(
                    &config_path,
                    bank_filter.as_ref().map(|(n, _)| n.as_str()),
                    bank_filter.as_ref().map(|(_, c)| c.as_str()),
                    output.format(),
                )
                .await?;
            }
            AccountAction::Metadata { action } => {
                accounts::metadata_local(&config_path, action).await?;
            }
        },
        Commands::Balance { action } => match action {
            BalanceAction::Fetch {
                app,
                output,
                account_id,
            } => {
                let config = Config::load(&config_path)?;
                let (_name, app_cfg) = config.resolve_app(app.app.as_deref())?;

                let id = EnableBankingAccountId::try_from(account_id)?;
                balance::fetch(&config, app_cfg, &id, output.format()).await?;
            }
            BalanceAction::List { bank, output } => {
                let bank_filter = local::resolve_filter_bank(&config_path, bank.bank.as_deref())?;
                balance::list_local(
                    &config_path,
                    bank_filter.as_ref().map(|(n, _)| n.as_str()),
                    bank_filter.as_ref().map(|(_, c)| c.as_str()),
                    output.format(),
                )
                .await?;
            }
            BalanceAction::Current { bank, output } => {
                let bank_filter = local::resolve_filter_bank(&config_path, bank.bank.as_deref())?;
                balance::current_local(
                    &config_path,
                    bank_filter.as_ref().map(|(n, _)| n.as_str()),
                    bank_filter.as_ref().map(|(_, c)| c.as_str()),
                    output.format(),
                )
                .await?;
            }
            BalanceAction::Metadata { action } => {
                balance::metadata_local(&config_path, action).await?;
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
