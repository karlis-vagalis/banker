use crate::api::EnableBankingClient;
use crate::api::models::EnableBankingAccountId;
use crate::auth::session::Sessions;
use crate::config::{Config, session_path};
use crate::db::{self};
use crate::error::AppError;
use crate::models::{ContentHash, TimeFrame, TransactionId};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

#[allow(dead_code)]
struct EntitySyncReport {
    new: usize,
    changed: usize,
    unchanged: usize,
}

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

impl AccountSyncReport {
    fn new() -> Self {
        Self::default()
    }
}

pub async fn run(
    config_path: &Path,
    time_frame: Option<TimeFrame>,
    bank_filter: Option<Vec<String>>,
    database: &crate::db::Database,
) -> Result<(), AppError> {
    let config = Config::load(config_path)?;
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
        tracing::warn!("no sessions found — run `banker auth login` first");
        return Ok(());
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
    let mut total = AccountSyncReport::new();

    for session in &sessions.0 {
        if session.is_expired() {
            tracing::warn!(
                "session for {} ({}) has expired, skipping",
                session.aspsp_name,
                session.aspsp_country
            );
            total.errors += 1;
            continue;
        }

        if let Some(ref banks) = resolved_banks {
            if !banks
                .iter()
                .any(|(n, c)| n == &session.aspsp_name && c == &session.aspsp_country)
            {
                continue;
            }
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

        let client = match EnableBankingClient::new(&config, app_cfg).await {
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
                &account_id,
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

    Ok(())
}

async fn sync_account(
    client: &EnableBankingClient,
    pool: &sqlx::Pool<sqlx::Sqlite>,
    account_id: &EnableBankingAccountId,
    aspsp_name: &str,
    aspsp_country: &str,
    time_frame: Option<&TimeFrame>,
) -> Result<AccountSyncReport, AppError> {
    let account = client.get_account_details(account_id).await?;
    let account_json = serde_json::to_value(&account)?;
    let new_hash = ContentHash::new(account_json.to_string().as_bytes());

    let account_name = account.name.as_deref().unwrap_or("Account").to_string();
    let account_iban = account
        .account_id
        .as_ref()
        .and_then(|id| id.iban.as_deref())
        .unwrap_or("")
        .to_string();

    let account_identification_hash = &account.identification_hash;

    let (account_fk, account_added, account_updated) =
        match db::account_exists(pool, account_identification_hash).await? {
            Some((existing_id, stored_hash)) => {
                if stored_hash != new_hash {
                    db::update_account(pool, existing_id.clone(), &account_json, new_hash).await?;
                    (existing_id, false, true)
                } else {
                    (existing_id, false, false)
                }
            }
            None => {
                let new_id = db::insert_account(
                    pool,
                    account_identification_hash,
                    aspsp_name,
                    aspsp_country,
                    &account_json,
                    new_hash,
                )
                .await?;
                (new_id, true, false)
            }
        };

    let mut report = AccountSyncReport::new();
    report.account_name = account_name;
    report.account_iban = account_iban;
    report.account_added = account_added;
    report.account_updated = account_updated;

    let mut tx = pool.begin().await?;

    let hal_balances = client.get_account_balances(account_id).await?;
    for b in &hal_balances.balances {
        db::insert_balance(
            &mut *tx,
            account_fk.clone(),
            &b.balance_type.to_string(),
            &serde_json::to_value(b)?,
        )
        .await?;
        report.balances += 1;
    }

    let txs = client
        .get_account_transactions(account_id, time_frame)
        .await?;

    let entry_refs: Vec<&str> = txs
        .iter()
        .filter_map(|t| t.entry_reference.as_deref())
        .collect();
    let existing_map: std::collections::HashMap<String, (TransactionId, ContentHash)> =
        if entry_refs.is_empty() {
            std::collections::HashMap::new()
        } else {
            db::transactions_existing_hashes(&mut *tx, account_fk.clone(), &entry_refs)
                .await?
                .into_iter()
                .map(|(ref_, id, h)| (ref_, (id, h)))
                .collect()
        };

    let mut to_insert: Vec<(Option<String>, String, ContentHash)> = Vec::new();
    for t in &txs {
        let tx_json = serde_json::to_value(t)?;
        let tx_hash = ContentHash::new(tx_json.to_string().as_bytes());
        let entry_ref = t.entry_reference.clone();

        match entry_ref.as_deref().and_then(|r| existing_map.get(r)) {
            Some(&(ref existing_tx_id, stored_hash)) => {
                if stored_hash != tx_hash {
                    db::update_transaction(&mut *tx, existing_tx_id.clone(), &tx_json, tx_hash)
                        .await?;
                    report.tx_updated += 1;
                } else {
                    report.tx_unchanged += 1;
                }
            }
            None => {
                to_insert.push((entry_ref, tx_json.to_string(), tx_hash));
            }
        }
    }

    if !to_insert.is_empty() {
        let batch: Vec<(Option<&str>, &str, ContentHash)> = to_insert
            .iter()
            .map(|(ref_, content, h)| (ref_.as_deref(), content.as_str(), *h))
            .collect();
        db::insert_transactions_batch(&mut *tx, account_fk.clone(), &batch).await?;
        report.tx_new = batch.len();
    }

    tx.commit().await?;

    Ok(report)
}
