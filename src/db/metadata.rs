use crate::error::AppError;
use crate::models::{Resource, ResourceType};
use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;

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
            id.raw(),
        ),
        Resource::Account(id) => (
            "INSERT INTO account_metadata (account_id, user_metadata, inserted_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(account_id) DO UPDATE SET
                 user_metadata = excluded.user_metadata,
                 updated_at = excluded.updated_at",
            id.raw(),
        ),
        Resource::Balance(id) => (
            "INSERT INTO balance_metadata (balance_id, user_metadata, inserted_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(balance_id) DO UPDATE SET
                 user_metadata = excluded.user_metadata,
                 updated_at = excluded.updated_at",
            id.raw(),
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
