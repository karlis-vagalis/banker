mod support;

use banker::{db::Database, db::row::AccountRow};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read},
    num::NonZeroU8,
    process::Stdio,
};
use support::{BankSeed, FixtureCommand, SeedSpec, TestBank, seeded_database};

#[tokio::test]
async fn local_commands_bound_stream_inspect_and_handle_closed_pipes() {
    let fixture = seeded_database(SeedSpec::default()).await.unwrap();
    let output = fixture
        .command()
        .args(["transaction", "list", "--jsonl"])
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
    assert!(rows.iter().any(|row| row["metadata"].is_object()));
    assert!(rows.iter().any(|row| row["metadata"].is_null()));
    assert!(String::from_utf8_lossy(&output.stderr).contains("more results available"));

    let output = fixture
        .command()
        .args(["transaction", "list", "--all", "--jsonl"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().lines().count() as u64,
        fixture.transaction_count()
    );
    // Long exports may emit SQLx slow-query diagnostics, but must never report truncation.
    assert!(!String::from_utf8_lossy(&output.stderr).contains("more results available"));

    let output = fixture
        .command()
        .args(["transaction", "list", "--jsonl", "--search", "no match"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());

    let id = rows[0]["id"].as_str().unwrap();
    let output = fixture
        .command()
        .args(["transaction", "show", id, "--jsonl"])
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
            "transaction",
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
    let sign = if rows[0]["content"]["credit_debit_indicator"] == "CRDT" {
        "+"
    } else {
        "-"
    };
    let mut expected = json!({"amount":format!("{sign}{}", rows[0]["content"]["transaction_amount"]["amount"].as_str().unwrap())});
    if let Some(category) = rows[0]["metadata"].get("category") {
        expected["metadata.category"] = category.clone();
    }
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        expected
    );

    let mut child = fixture
        .command()
        .args(["transaction", "list", "--all", "--jsonl"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    stdout.read_exact(&mut [0]).unwrap();
    drop(stdout);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
}

#[tokio::test]
async fn seeded_banks_and_accounts_are_isolated_by_cli_filters() {
    let fixture = seeded_database(SeedSpec::default()).await.unwrap();
    assert_eq!(fixture.banks.len(), 4);
    assert_eq!(
        fixture
            .banks
            .iter()
            .map(|b| b.accounts.len())
            .sum::<usize>(),
        8
    );
    assert_eq!(fixture.transaction_count(), 11_110);
    for bank in &fixture.banks {
        let output = fixture
            .command()
            .args([
                "transaction",
                "list",
                "--bank",
                bank.bank.key(),
                "--all",
                "--jsonl",
                "--fields",
                "account_id,currency",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!String::from_utf8_lossy(&output.stderr).contains("more results available"));
        let rows: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let expected = bank
            .accounts
            .iter()
            .map(|a| u64::from(a.transaction_count))
            .sum::<u64>();
        assert_eq!(rows.len() as u64, expected);
        let accounts: BTreeMap<_, _> = bank
            .accounts
            .iter()
            .map(|a| (a.id.to_string(), a.currency))
            .collect();
        assert!(rows.iter().all(|row| {
            accounts.get(row["account_id"].as_str().unwrap()).copied() == row["currency"].as_str()
        }));
        for account in &bank.accounts {
            let output = fixture
                .command()
                .args([
                    "transaction",
                    "list",
                    "--bank",
                    bank.bank.key(),
                    "--account",
                    &account.id.to_string(),
                    "--all",
                    "--jsonl",
                    "--fields",
                    "account_id",
                ])
                .output()
                .unwrap();
            assert!(output.status.success());
            let lines = String::from_utf8(output.stdout).unwrap();
            assert_eq!(lines.lines().count() as u32, account.transaction_count);
            assert!(lines.lines().all(
                |line| serde_json::from_str::<Value>(line).unwrap()["account_id"]
                    == account.id.to_string()
            ));
        }
    }
    let output = fixture
        .command()
        .args([
            "transaction",
            "list",
            "--bank",
            "pine",
            "--from",
            "2025-02-01",
            "--to",
            "2025-03-01",
            "--all",
            "--jsonl",
            "--fields",
            "date",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let lines = String::from_utf8(output.stdout).unwrap();
    assert!(!lines.is_empty());
    assert!(lines.lines().all(|line| {
        let value: Value = serde_json::from_str(line).unwrap();
        let date = value["date"].as_str().unwrap();
        ("2025-02-01".."2025-03-01").contains(&date)
    }));
    let output = fixture
        .command()
        .args([
            "transaction",
            "list",
            "--bank",
            "pine",
            "--search",
            "HOUSEHOLD",
            "--all",
            "--jsonl",
            "--fields",
            "description",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let lines = String::from_utf8(output.stdout).unwrap();
    assert!(!lines.is_empty());
    assert!(lines.lines().all(|line| {
        serde_json::from_str::<Value>(line).unwrap()["description"]
            .as_str()
            .unwrap()
            .contains("household")
    }));
}

#[tokio::test]
async fn seeded_database_contains_mixed_data_and_valid_account_content() {
    let fixture = seeded_database(SeedSpec::default()).await.unwrap();
    let database = Database::open(&fixture.db_path()).await.unwrap();
    let accounts: Vec<AccountRow> = sqlx::query_as("SELECT * FROM accounts ORDER BY id")
        .fetch_all(database.pool())
        .await
        .unwrap();
    assert_eq!(accounts.len(), 8);
    let bank_counts: Vec<(String, String, i64)> = sqlx::query_as("SELECT a.aspsp_name, a.aspsp_country, count(t.id) FROM accounts a JOIN transactions t ON t.account_id=a.id GROUP BY a.aspsp_name, a.aspsp_country ORDER BY count(t.id)")
        .fetch_all(database.pool()).await.unwrap();
    assert_eq!(
        bank_counts.iter().map(|row| row.2).collect::<Vec<_>>(),
        [10, 100, 1000, 10000]
    );
    for (bank, actual) in fixture.banks.iter().zip(&bank_counts) {
        assert_eq!(bank.bank.name(), actual.0);
        assert_eq!(bank.bank.country(), actual.1);
    }
    for predicate in [
        "m.transaction_id IS NULL",
        "m.user_metadata = '{}'",
        "json_type(m.user_metadata, '$.review') = 'null'",
        "json_type(m.user_metadata, '$.nested.weight') = 'real'",
        "json_type(m.user_metadata, '$.tags') = 'array'",
        "json_extract(t.content, '$.booking_date') IS NULL",
        "json_extract(t.content, '$.status') = 'PDNG'",
        "t.entry_reference IS NULL",
        "json_extract(t.content, '$.credit_debit_indicator') = 'CRDT'",
        "json_extract(t.content, '$.credit_debit_indicator') = 'DBIT'",
    ] {
        let mut query = sqlx::QueryBuilder::new(
            "SELECT count(*) FROM transactions t LEFT JOIN transaction_metadata m ON m.transaction_id=t.id WHERE ",
        );
        query.push(predicate);
        let count: i64 = query
            .build_query_scalar()
            .fetch_one(database.pool())
            .await
            .unwrap();
        assert!(count > 0, "fixture is missing variation: {predicate}");
    }
    let currencies: Vec<String> = sqlx::query_scalar("SELECT DISTINCT json_extract(content, '$.transaction_amount.currency') FROM transactions ORDER BY 1").fetch_all(database.pool()).await.unwrap();
    assert_eq!(currencies, ["CHF", "EUR", "GBP", "USD"]);
    let distinct_amounts: i64 = sqlx::query_scalar("SELECT count(DISTINCT json_extract(content, '$.transaction_amount.amount')) FROM transactions").fetch_one(database.pool()).await.unwrap();
    assert!(distinct_amounts > 1000);
    let violations: Vec<(String, Option<i64>, String, i64)> =
        sqlx::query_as("PRAGMA foreign_key_check")
            .fetch_all(database.pool())
            .await
            .unwrap();
    assert!(violations.is_empty());
    database.close().await;
    let path = fixture.directory().to_path_buf();
    drop(fixture);
    assert!(!path.exists());
}

#[tokio::test]
async fn custom_seeds_are_repeatable_and_support_empty_accounts() {
    let accounts = NonZeroU8::new(3).unwrap();
    let spec = SeedSpec::new([
        BankSeed::new(TestBank::Birch, accounts, 7),
        BankSeed::new(TestBank::Pine, accounts, 0),
    ])
    .unwrap();
    let first = seeded_database(spec.clone()).await.unwrap();
    let second = seeded_database(spec).await.unwrap();
    assert_ne!(first.directory(), second.directory());
    assert_eq!(first.transaction_count(), 7);
    assert_eq!(
        first.banks[0]
            .accounts
            .iter()
            .map(|a| a.transaction_count)
            .collect::<Vec<_>>(),
        [3, 2, 2]
    );
    assert!(
        first.banks[1]
            .accounts
            .iter()
            .all(|a| a.transaction_count == 0)
    );
    let first_output = first
        .command()
        .args(["transaction", "list", "--all", "--jsonl"])
        .output()
        .unwrap();
    let second_output = second
        .command()
        .args(["transaction", "list", "--all", "--jsonl"])
        .output()
        .unwrap();
    assert!(first_output.status.success() && second_output.status.success());
    assert_eq!(first_output.stdout, second_output.stdout);
    let output = first
        .command()
        .args(["transaction", "list", "--bank", "pine", "--all", "--jsonl"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[tokio::test]
#[ignore = "larger dataset; run cargo test --test transactions hundred_thousand -- --ignored"]
async fn hundred_thousand_transactions_can_be_streamed_from_a_custom_seed() {
    let spec = SeedSpec::new([BankSeed::new(
        TestBank::Pine,
        NonZeroU8::new(5).unwrap(),
        100_000,
    )])
    .unwrap();
    let fixture = seeded_database(spec).await.unwrap();
    let mut child = fixture
        .command()
        .args([
            "transaction",
            "list",
            "--bank",
            "pine",
            "--all",
            "--jsonl",
            "--fields",
            "id",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let mut count = 0u64;
    for line in stdout.lines() {
        let row: Value = serde_json::from_str(&line.unwrap()).unwrap();
        assert!(row["id"].as_str().unwrap().starts_with("tx_"));
        count += 1;
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("more results available"));
    assert_eq!(count, fixture.transaction_count());
    assert_eq!(count, 100_000);
}

#[tokio::test]
async fn persisted_seed_survives_drop_and_can_be_browsed_with_the_cli() {
    let spec = SeedSpec::new([BankSeed::new(
        TestBank::Birch,
        NonZeroU8::new(2).unwrap(),
        10,
    )])
    .unwrap();
    let fixture = seeded_database(spec).await.unwrap();
    let config = fixture.config_path();
    let directory = fixture.persist();
    // Persistence is explicit; this test removes its generated directory after use.
    assert!(directory.join("data.db").is_file());
    assert!(config.is_file());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_banker"))
        .arg("--config")
        .arg(&config)
        .args(["transaction", "list", "--all", "--jsonl"])
        .output()
        .unwrap();
    std::fs::remove_dir_all(&directory).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().lines().count(),
        10
    );
}

#[test]
fn seed_spec_rejects_empty_and_duplicate_bank_layouts() {
    assert!(SeedSpec::new([]).is_err());
    let bank = BankSeed::new(TestBank::Birch, NonZeroU8::new(2).unwrap(), 10);
    assert!(SeedSpec::new([bank.clone(), bank]).is_err());
}
