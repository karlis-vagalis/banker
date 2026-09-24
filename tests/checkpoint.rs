use banker::db::Database;
use std::path::Path;

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
