use super::AccountSyncReport;
use crate::api::EnableBankingClient;
use crate::api::models::EnableBankingAccountId;
use crate::db;
use crate::error::AppError;
use crate::models::{ContentHash, TimeFrame};

pub(super) async fn sync_account(
    client: &EnableBankingClient,
    pool: &sqlx::SqlitePool,
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

    let hal_balances = client.get_account_balances(account_id).await?;
    let txs = client
        .get_account_transactions(account_id, time_frame)
        .await?;
    let mut tx = pool.begin().await?;

    let (account_fk, account_added, account_updated) =
        match db::account_exists(&mut *tx, &account.identification_hash).await? {
            Some((existing_id, stored_hash)) => {
                if stored_hash != new_hash {
                    db::update_account(&mut *tx, existing_id.clone(), &account_json, new_hash)
                        .await?;
                    (existing_id, false, true)
                } else {
                    (existing_id, false, false)
                }
            }
            None => {
                let new_id = db::insert_account(
                    &mut *tx,
                    &account.identification_hash,
                    aspsp_name,
                    aspsp_country,
                    &account_json,
                    new_hash,
                )
                .await?;
                (new_id, true, false)
            }
        };

    let mut report = AccountSyncReport {
        account_name,
        account_iban,
        account_added,
        account_updated,
        ..Default::default()
    };

    for balance in &hal_balances.balances {
        db::insert_balance(
            &mut *tx,
            account_fk.clone(),
            &balance.balance_type.to_string(),
            &serde_json::to_value(balance)?,
        )
        .await?;
        report.balances += 1;
    }

    let entry_refs: Vec<&str> = txs
        .iter()
        .filter_map(|t| t.entry_reference.as_deref())
        .collect();
    let mut existing_map = std::collections::HashMap::new();
    for chunk in entry_refs.chunks(900) {
        existing_map.extend(
            db::transactions_existing_hashes(&mut *tx, account_fk.clone(), chunk)
                .await?
                .into_iter()
                .map(|(reference, id, hash)| (reference, (id, hash))),
        );
    }
    let mut unreferenced_hashes: std::collections::HashSet<ContentHash> =
        db::unreferenced_transaction_hashes(&mut *tx, account_fk.clone())
            .await?
            .into_iter()
            .collect();

    let mut to_insert: Vec<(Option<String>, String, ContentHash)> = Vec::new();
    let mut new_references = std::collections::HashSet::new();
    for transaction in &txs {
        let json = serde_json::to_value(transaction)?;
        let hash = ContentHash::new(json.to_string().as_bytes());
        let entry_ref = transaction.entry_reference.clone();

        match entry_ref.as_deref().and_then(|r| existing_map.get(r)) {
            Some(&(ref existing_id, stored_hash)) => {
                if stored_hash != hash {
                    db::update_transaction(&mut *tx, existing_id.clone(), &json, hash).await?;
                    report.tx_updated += 1;
                } else {
                    report.tx_unchanged += 1;
                }
            }
            None if entry_ref.is_none() && !unreferenced_hashes.insert(hash) => {
                report.tx_unchanged += 1;
            }
            None if entry_ref
                .as_ref()
                .is_some_and(|r| !new_references.insert(r.clone())) =>
            {
                report.tx_unchanged += 1;
            }
            None => to_insert.push((entry_ref, json.to_string(), hash)),
        }
    }

    if !to_insert.is_empty() {
        let batch: Vec<(Option<&str>, &str, ContentHash)> = to_insert
            .iter()
            .map(|(reference, content, hash)| (reference.as_deref(), content.as_str(), *hash))
            .collect();
        for chunk in batch.chunks(900) {
            db::insert_transactions_batch(&mut *tx, account_fk.clone(), chunk).await?;
        }
        report.tx_new = batch.len();
    }

    tx.commit().await?;
    Ok(report)
}
