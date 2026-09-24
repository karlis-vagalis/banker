pub mod row;

use crate::error::AppError;
use crate::models::{AccountId, ContentHash, Resource, ResourceType, TransactionId};
use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;
use std::path::Path;

/// Opens (creating if needed) the SQLite database, runs pending migrations,
/// applies performance PRAGMAs, and returns a connection pool.
async fn init_pool(path: &Path) -> Result<SqlitePool, AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pool = SqlitePool::connect(&format!("sqlite://{}?mode=rwc", path.display())).await?;

    sqlx::migrate!("db/migrations").run(&pool).await?;

    sqlx::raw_sql(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA cache_size = -2000;
        PRAGMA temp_store = MEMORY;
        PRAGMA mmap_size = 30000000000;
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

/// Owns the connection pool and its orderly shutdown lifecycle.
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn open(path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            pool: init_pool(path).await?,
        })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn checkpoint(&self) -> Result<(), AppError> {
        checkpoint_pool(&self.pool).await
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}

/// Checkpoint WAL changes into the main database file.
async fn checkpoint_pool(pool: &SqlitePool) -> Result<(), AppError> {
    let (busy, log_frames, checkpointed_frames): (i32, i32, i32) =
        sqlx::query_as("PRAGMA wal_checkpoint(PASSIVE)")
            .fetch_one(pool)
            .await?;

    if log_frames >= 0 && checkpointed_frames < log_frames {
        return Err(AppError::Other(format!(
            "SQLite WAL checkpoint incomplete (busy: {busy}, frames: {checkpointed_frames}/{log_frames})"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn checkpoint_merges_wal_frames() {
        let path = std::env::temp_dir().join(format!(
            "banker-checkpoint-test-{}-{}.db",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let database = Database::open(&path).await.unwrap();

        sqlx::query(
            "INSERT INTO metadata_schemas (target_type, json_schema, updated_at) VALUES (?, ?, ?)",
        )
        .bind("accounts")
        .bind("{}")
        .bind(chrono::Utc::now())
        .execute(database.pool())
        .await
        .unwrap();

        database.checkpoint().await.unwrap();
        let status: (i32, i32, i32) = sqlx::query_as("PRAGMA wal_checkpoint(NOOP)")
            .fetch_one(database.pool())
            .await
            .unwrap();
        assert_eq!(status.0, 0);
        assert_eq!(status.1, status.2);

        database.close().await;
        for suffix in ["", "-wal", "-shm"] {
            let file = Path::new(&format!("{}{suffix}", path.display())).to_path_buf();
            let _ = std::fs::remove_file(file);
        }
    }
}

/// Check if an account exists by identification_hash.
/// Returns `Some((id, stored_hash))` if found, `None` otherwise.
pub async fn account_exists(
    pool: &SqlitePool,
    identification_hash: &str,
) -> Result<Option<(AccountId, ContentHash)>, AppError> {
    let row: Option<(AccountId, ContentHash)> =
        sqlx::query_as("SELECT id, content_hash FROM accounts WHERE identification_hash = ?1")
            .bind(identification_hash)
            .fetch_optional(pool)
            .await?;
    Ok(row)
}

/// Insert a new account row and return its id.
pub async fn insert_account(
    pool: &SqlitePool,
    identification_hash: &str,
    aspsp_name: &str,
    aspsp_country: &str,
    content: &serde_json::Value,
    content_hash: ContentHash,
) -> Result<AccountId, AppError> {
    let now = Utc::now().to_rfc3339();
    let id: AccountId = sqlx::query_scalar(
        "INSERT INTO accounts (identification_hash, aspsp_name, aspsp_country, content, content_hash, inserted_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         RETURNING id",
    )
    .bind(identification_hash)
    .bind(aspsp_name)
    .bind(aspsp_country)
    .bind(content.to_string())
    .bind(content_hash)
    .bind(now.clone())
    .bind(now.clone())
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Update an existing account's content and hash. Sets `updated_at`.
pub async fn update_account(
    pool: &SqlitePool,
    id: AccountId,
    content: &serde_json::Value,
    content_hash: ContentHash,
) -> Result<(), AppError> {
    let updated_at = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE accounts
         SET content = ?1, content_hash = ?2, updated_at = ?3
         WHERE id = ?4",
    )
    .bind(content.to_string())
    .bind(content_hash)
    .bind(updated_at)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_balance<'e, E: sqlx::Executor<'e, Database = sqlx::Sqlite>>(
    executor: E,
    account_fk: AccountId,
    balance_type: &str,
    content: &serde_json::Value,
) -> Result<(), AppError> {
    let inserted_at = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO balances (account_id, balance_type, content, inserted_at)
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(account_fk)
    .bind(balance_type)
    .bind(content.to_string())
    .bind(inserted_at)
    .execute(executor)
    .await?;
    Ok(())
}

/// Update an existing transaction's content and hash. Sets `updated_at`.
pub async fn update_transaction<'e, E: sqlx::Executor<'e, Database = sqlx::Sqlite>>(
    executor: E,
    id: TransactionId,
    content: &serde_json::Value,
    content_hash: ContentHash,
) -> Result<(), AppError> {
    let updated_at = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE transactions
         SET content = ?1, content_hash = ?2, updated_at = ?3
         WHERE id = ?4",
    )
    .bind(content.to_string())
    .bind(content_hash)
    .bind(updated_at)
    .bind(id)
    .execute(executor)
    .await?;
    Ok(())
}

/// Given an account_fk and a list of entry_references, return all existing
/// `(entry_reference, id, content_hash)` rows for match-and-partition logic.
pub async fn transactions_existing_hashes<'e, E: sqlx::Executor<'e, Database = sqlx::Sqlite>>(
    executor: E,
    account_fk: AccountId,
    entry_refs: &[&str],
) -> Result<Vec<(String, TransactionId, ContentHash)>, AppError> {
    if entry_refs.is_empty() {
        return Ok(Vec::new());
    }
    let mut qb = sqlx::QueryBuilder::new(
        "SELECT entry_reference, id, content_hash FROM transactions WHERE account_id = ",
    );
    qb.push_bind(account_fk);
    qb.push(" AND entry_reference IN (");
    let mut sep = qb.separated(", ");
    for r in entry_refs {
        sep.push_bind(r);
    }
    qb.push(")");
    Ok(qb.build_query_as().fetch_all(executor).await?)
}

/// Batch-insert multiple transaction rows in a single statement.
/// `rows` must be non-empty. Each tuple is `(entry_reference, content_json, content_hash)`.
pub async fn insert_transactions_batch<'e, E: sqlx::Executor<'e, Database = sqlx::Sqlite>>(
    executor: E,
    account_fk: AccountId,
    rows: &[(Option<&str>, &str, ContentHash)],
) -> Result<(), AppError> {
    assert!(
        !rows.is_empty(),
        "insert_transactions_batch called with empty rows"
    );

    let now = Utc::now().to_rfc3339();
    let mut qb = sqlx::QueryBuilder::new(
        "INSERT INTO transactions (account_id, entry_reference, content, content_hash, inserted_at, updated_at) ",
    );
    qb.push_values(rows.iter(), |mut b, row| {
        let (entry_ref, content, hash) = row;
        b.push_bind(account_fk.clone())
            .push_bind(*entry_ref)
            .push_bind(*content)
            .push_bind(*hash)
            .push_bind(now.clone())
            .push_bind(now.clone());
    });
    qb.build().execute(executor).await?;
    Ok(())
}

pub async fn upsert_metadata_schema(
    pool: &SqlitePool,
    resource_type: &ResourceType,
    json_schema: &Value,
) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO metadata_schemas (target_type, json_schema, inserted_at, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(target_type) DO UPDATE SET
             json_schema = excluded.json_schema,
             updated_at = excluded.updated_at",
    )
    .bind(resource_type.name())
    .bind(json_schema.to_string())
    .bind(now.clone())
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_metadata_schema(
    pool: &SqlitePool,
    resource_type: &ResourceType,
) -> Result<Option<Value>, AppError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT json_schema FROM metadata_schemas WHERE target_type = ?1")
            .bind(resource_type.name())
            .fetch_optional(pool)
            .await?;
    match row {
        Some((s,)) => Ok(Some(serde_json::from_str(&s)?)),
        None => Ok(None),
    }
}

pub async fn delete_metadata_schema(
    pool: &SqlitePool,
    resource_type: &ResourceType,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM metadata_schemas WHERE target_type = ?1")
        .bind(resource_type.name())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn upsert_metadata(
    pool: &SqlitePool,
    resource: &Resource,
    user_metadata: &Value,
) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let (sql, id) = match resource {
        Resource::Transaction(id) => (
            "INSERT INTO transaction_metadata (transaction_id, user_metadata, inserted_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(transaction_id) DO UPDATE SET
                 user_metadata = excluded.user_metadata,
                 updated_at = excluded.updated_at",
            id.to_string(),
        ),
        Resource::Account(id) => (
            "INSERT INTO account_metadata (account_id, user_metadata, inserted_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(account_id) DO UPDATE SET
                 user_metadata = excluded.user_metadata,
                 updated_at = excluded.updated_at",
            id.to_string(),
        ),
        Resource::Balance(id) => (
            "INSERT INTO balance_metadata (balance_id, user_metadata, inserted_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(balance_id) DO UPDATE SET
                 user_metadata = excluded.user_metadata,
                 updated_at = excluded.updated_at",
            id.to_string(),
        ),
    };
    sqlx::query(sql)
        .bind(id)
        .bind(user_metadata.to_string())
        .bind(now.clone())
        .bind(now)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_metadata(
    pool: &SqlitePool,
    resource: &Resource,
) -> Result<Option<Value>, AppError> {
    let row: Option<(String,)> = match resource {
        Resource::Transaction(id) => {
            sqlx::query_as(
                "SELECT user_metadata FROM transaction_metadata WHERE transaction_id = ?1",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?
        }
        Resource::Account(id) => {
            sqlx::query_as("SELECT user_metadata FROM account_metadata WHERE account_id = ?1")
                .bind(id)
                .fetch_optional(pool)
                .await?
        }
        Resource::Balance(id) => {
            sqlx::query_as("SELECT user_metadata FROM balance_metadata WHERE balance_id = ?1")
                .bind(id)
                .fetch_optional(pool)
                .await?
        }
    };
    match row {
        Some((s,)) => Ok(Some(serde_json::from_str(&s)?)),
        None => Ok(None),
    }
}

pub async fn delete_metadata(pool: &SqlitePool, resource: &Resource) -> Result<(), AppError> {
    match resource {
        Resource::Transaction(id) => {
            sqlx::query("DELETE FROM transaction_metadata WHERE transaction_id = ?1")
                .bind(id)
                .execute(pool)
                .await?
        }
        Resource::Account(id) => {
            sqlx::query("DELETE FROM account_metadata WHERE account_id = ?1")
                .bind(id)
                .execute(pool)
                .await?
        }
        Resource::Balance(id) => {
            sqlx::query("DELETE FROM balance_metadata WHERE balance_id = ?1")
                .bind(id)
                .execute(pool)
                .await?
        }
    };
    Ok(())
}
