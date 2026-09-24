use crate::api::EnableBankingClient;
use crate::auth::session::Sessions;
use crate::config::{Config, session_path};
use crate::error::AppError;
use crate::models::TimeFrame;
mod account;
use account::sync_account;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Default)]
struct AccountSyncReport {
    account_name: String,
    account_iban: String,
    account_added: bool,
    account_updated: bool,
    accounts: usize,
    added: usize,
    updated: usize,
    unchanged: usize,
    errors: usize,
    balances: usize,
    tx_new: usize,
    tx_updated: usize,
    tx_unchanged: usize,
}

pub async fn run(
    config: &Config,
    time_frame: Option<TimeFrame>,
    bank_filter: Option<Vec<String>>,
    database: &crate::db::Database,
) -> Result<(), AppError> {
    match &time_frame {
        Some(tf) => tracing::info!("syncing transactions from {} to {}", tf.from, tf.to),
        None => tracing::info!("syncing all available transactions"),
    }

    let resolved_banks: Option<Vec<(String, String)>> = match bank_filter {
        Some(keys) => {
            let pairs: Result<Vec<_>, AppError> = keys
                .iter()
                .map(|key| {
                    let (_k, cfg) = config.resolve_bank(Some(key))?;
                    Ok((cfg.name.clone(), cfg.country.clone()))
                })
                .collect();
            Some(pairs?)
        }
        None => None,
    };

    let sessions = Sessions::load(&session_path())?;

    if sessions.0.is_empty() {
        return Err(AppError::Other(
            "no sessions found — run `banker auth login` first".into(),
        ));
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );
    pb.set_message("Loading sessions...");

    let mut account_lines: Vec<String> = Vec::new();
    let mut balance_lines: Vec<String> = Vec::new();
    let mut tx_lines: Vec<String> = Vec::new();
    let mut total = AccountSyncReport::default();
    let mut matched_sessions = 0;

    for session in &sessions.0 {
        if let Some(ref banks) = resolved_banks
            && !banks
                .iter()
                .any(|(n, c)| n == &session.aspsp_name && c == &session.aspsp_country)
        {
            continue;
        }
        matched_sessions += 1;
        if session.is_expired() {
            tracing::warn!(
                "session for {} ({}) has expired, skipping",
                session.aspsp_name,
                session.aspsp_country
            );
            total.errors += 1;
            continue;
        }

        let Some((_app_name, app_cfg)) = config.find_app_by_id(&session.app_id) else {
            tracing::warn!(
                "no app config found for app_id {} (session for {} {}), skipping",
                session.app_id,
                session.aspsp_name,
                session.aspsp_country
            );
            total.errors += 1;
            continue;
        };

        let client = match EnableBankingClient::new(config, app_cfg).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    "failed to create API client for {} ({}): {e}, skipping",
                    session.aspsp_name,
                    session.aspsp_country
                );
                total.errors += 1;
                continue;
            }
        };

        account_lines.push(format!(
            "  {} ({}) · {} account(s)",
            session.aspsp_name,
            session.aspsp_country,
            session.accounts.len()
        ));

        let mut session_accounts = Vec::new();

        for (i, account_id) in session.accounts.iter().enumerate() {
            pb.set_message(format!(
                "{} ({}) · Processing account {}/{}...",
                session.aspsp_name,
                session.aspsp_country,
                i + 1,
                session.accounts.len()
            ));
            let outcome = match sync_account(
                &client,
                database.pool(),
                account_id,
                &session.aspsp_name,
                &session.aspsp_country,
                time_frame.as_ref(),
            )
            .await
            {
                Ok(outcome) => outcome,
                Err(e) => {
                    account_lines.push(format!("    ✗ {} — error: {e}", account_id));
                    total.errors += 1;
                    continue;
                }
            };

            let status = if outcome.account_added {
                "added"
            } else if outcome.account_updated {
                "updated"
            } else {
                "unchanged"
            };
            let desc = if outcome.account_iban.is_empty() {
                outcome.account_name.clone()
            } else {
                format!("{} ({})", outcome.account_name, outcome.account_iban)
            };

            account_lines.push(format!("    ✓ {desc} — {status}"));

            total.accounts += 1;
            if outcome.account_added {
                total.added += 1;
            } else if outcome.account_updated {
                total.updated += 1;
            } else {
                total.unchanged += 1;
            }
            total.balances += outcome.balances;
            total.tx_new += outcome.tx_new;
            total.tx_updated += outcome.tx_updated;
            total.tx_unchanged += outcome.tx_unchanged;

            session_accounts.push(outcome);
        }

        let session_balance_count: usize = session_accounts.iter().map(|a| a.balances).sum();
        if session_balance_count > 0 {
            balance_lines.push(format!(
                "  {} ({})",
                session.aspsp_name, session.aspsp_country
            ));
            balance_lines.push(format!("    ✓ {session_balance_count} balance(s) stored"));
        }

        let mut session_tx_lines: Vec<String> = Vec::new();
        for outcome in &session_accounts {
            let has_tx = outcome.tx_new + outcome.tx_updated + outcome.tx_unchanged > 0;
            if has_tx {
                let desc = if outcome.account_iban.is_empty() {
                    outcome.account_name.clone()
                } else {
                    format!("{} ({})", outcome.account_name, outcome.account_iban)
                };
                session_tx_lines.push(format!(
                    "    ✓ {desc} · {} new · {} updated · {} unchanged",
                    outcome.tx_new, outcome.tx_updated, outcome.tx_unchanged
                ));
            }
        }

        if !session_tx_lines.is_empty() {
            tx_lines.push(format!(
                "  {} ({})",
                session.aspsp_name, session.aspsp_country
            ));
            tx_lines.extend(session_tx_lines);
        }
    }

    pb.finish_and_clear();
    if matched_sessions == 0 {
        return Err(AppError::Other(
            "no sessions match the selected bank(s) — run `banker auth login` first".into(),
        ));
    }

    println!("\n[1/3] Accounts ──────────────────────────");
    for line in &account_lines {
        println!("{line}");
    }

    if !balance_lines.is_empty() {
        println!("\n[2/3] Balances ──────────────────────────");
        for line in &balance_lines {
            println!("{line}");
        }
    }

    if !tx_lines.is_empty() {
        println!("\n[3/3] Transactions ──────────────────────");
        for line in &tx_lines {
            println!("{line}");
        }
    }

    if total.errors > 0 {
        println!(
            "\nSync complete ({} error(s)). Data stored in {}",
            total.errors,
            config.db_path().display()
        );
    } else {
        println!(
            "\nSync complete. Data stored in {}",
            config.db_path().display()
        );
    }
    println!(
        "({} accounts · {} added, {} updated, {} unchanged; \
         {} balances; \
         {} new txs, {} updated, {} unchanged)",
        total.accounts,
        total.added,
        total.updated,
        total.unchanged,
        total.balances,
        total.tx_new,
        total.tx_updated,
        total.tx_unchanged,
    );

    if total.errors > 0 {
        return Err(AppError::Other(format!(
            "sync completed with {} error(s)",
            total.errors
        )));
    }
    Ok(())
}
