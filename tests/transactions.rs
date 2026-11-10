use banker::db::Database;
use serde_json::{Value, json};
use std::{
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
impl Fixture {
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_banker"));
        command
            .arg("--config")
            .arg(self.0.join("config.toml"))
            .arg("transaction");
        command
    }
}

#[tokio::test]
async fn local_commands_bound_stream_inspect_and_handle_closed_pipes() {
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "banker-transactions-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    )));
    std::fs::create_dir_all(&fixture.0).unwrap();
    let db_path = fixture.0.join("data.db");
    std::fs::write(
        fixture.0.join("config.toml"),
        format!(
            "db_path = {:?}\n[application.test]\nid = 'unused'\nkey_file = 'unused'\n",
            db_path.to_str().unwrap()
        ),
    )
    .unwrap();
    let database = Database::open(&db_path).await.unwrap();
    sqlx::query("INSERT INTO accounts (id, aspsp_name, aspsp_country, content, content_hash, updated_at) VALUES (1, 'Test', 'DE', '{}', 0, '2026-01-01T00:00:00Z')")
        .execute(database.pool()).await.unwrap();
    let content = json!({"booking_date":"2026-01-15", "transaction_amount":{"amount":"42.30","currency":"EUR"},
        "credit_debit_indicator":"DBIT","status":"BOOK","remittance_information":["Example payment"]});
    sqlx::query("WITH RECURSIVE ids(id) AS (SELECT 1 UNION ALL SELECT id+1 FROM ids WHERE id<2000) INSERT INTO transactions (id, account_id, content, content_hash, updated_at) SELECT id, 1, ?, 0, '2026-01-02T00:00:00Z' FROM ids")
        .bind(content.to_string()).execute(database.pool()).await.unwrap();
    sqlx::query("INSERT INTO transaction_metadata (transaction_id, user_metadata, updated_at) VALUES (2000, '{\"category\":\"groceries\"}', '2026-01-02T00:00:00Z')")
        .execute(database.pool()).await.unwrap();
    database.checkpoint().await.unwrap();
    database.close().await;

    let output = fixture
        .command()
        .args(["list", "--jsonl"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(rows.len(), 50);
    assert_eq!(rows[0]["metadata"]["category"], "groceries");
    assert!(String::from_utf8_lossy(&output.stderr).contains("more results available"));

    let output = fixture
        .command()
        .args(["list", "--all", "--jsonl"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().lines().count(),
        2000
    );
    assert!(output.stderr.is_empty());

    let output = fixture
        .command()
        .args(["list", "--jsonl", "--search", "no match"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());

    let id = rows[0]["id"].as_str().unwrap();
    let output = fixture
        .command()
        .args(["show", id, "--jsonl"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        rows[0]
    );

    let output = fixture
        .command()
        .args([
            "list",
            "--limit",
            "1",
            "--jsonl",
            "--fields",
            "metadata.category,amount",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"metadata.category":"groceries","amount":"-42.30"})
    );

    let mut child = fixture
        .command()
        .args(["list", "--all", "--jsonl"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut byte = [0];
    stdout.read_exact(&mut byte).unwrap();
    drop(stdout);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
}
