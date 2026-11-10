use crate::{
    cli::{Cli, Commands, TransactionAction},
    db::transactions::{self, TransactionRecord},
    models::{
        AccountId, TransactionId,
        transaction_query::{BookingDateRange, RowLimit, TransactionQuery},
    },
    output::{OutputFormat, TransactionRenderer},
};
use clap::Parser;
use serde_json::{Value, json};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::num::NonZeroU32;

async fn fixture() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("db/migrations").run(&pool).await.unwrap();
    for (id, name) in [(1, "First"), (2, "Second")] {
        sqlx::query("INSERT INTO accounts (id, aspsp_name, aspsp_country, content, content_hash, updated_at) VALUES (?, ?, 'DE', '{}', 0, '2026-01-01T00:00:00Z')")
            .bind(id).bind(name).execute(&pool).await.unwrap();
    }
    for (id, account, date, desc, indicator) in [
        (1, 1, Some("2026-01-01"), "Old", "DBIT"),
        (2, 1, Some("2026-01-15"), "Supermarket 100%", "DBIT"),
        (3, 2, Some("2026-01-15"), "Refund", "CRDT"),
        (4, 1, None, "Pending", "DBIT"),
        (5, 1, Some("2026-02-01"), "Boundary", "DBIT"),
    ] {
        let content = json!({"booking_date":date, "transaction_amount":{"amount":"42.30","currency":"EUR"},
            "credit_debit_indicator":indicator,"status":"BOOK","remittance_information":[desc],
            "creditor":{"name":"Example Store"},"debtor":{"name":"Refund Sender"}});
        sqlx::query("INSERT INTO transactions (id, account_id, content, content_hash, updated_at) VALUES (?, ?, ?, 0, '2026-01-02T00:00:00Z')")
            .bind(id).bind(account).bind(content.to_string()).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO transaction_metadata (transaction_id, user_metadata, updated_at) VALUES (2, ?, '2026-01-02T00:00:00Z')")
        .bind(json!({"category":"groceries","review":null,"nested":{"weight":0.8}}).to_string()).execute(&pool).await.unwrap();
    pool
}

fn query() -> TransactionQuery {
    TransactionQuery {
        accounts: vec![],
        dates: BookingDateRange::new(None, None).unwrap(),
        search: None,
        limit: RowLimit::All,
    }
}

async fn records(
    pool: &SqlitePool,
    query: &TransactionQuery,
    bank: Option<(&str, &str)>,
) -> Vec<TransactionRecord> {
    transactions::list_query(query, bank)
        .build_query_as()
        .fetch_all(pool)
        .await
        .unwrap()
}

fn ids(rows: &[TransactionRecord]) -> Vec<i64> {
    rows.iter().map(|r| r.id.raw()).collect()
}

#[tokio::test]
async fn ordering_filters_and_metadata() {
    let pool = fixture().await;
    let mut q = query();
    let all = records(&pool, &q, None).await;
    assert_eq!(ids(&all), [5, 3, 2, 1, 4]);
    assert!(all[0].metadata.is_none());
    assert_eq!(all[2].metadata.as_ref().unwrap().0["category"], "groceries");
    q.accounts = vec![AccountId::try_from(1).unwrap()];
    q.dates = BookingDateRange::new(
        Some("2026-01-01".parse().unwrap()),
        Some("2026-02-01".parse().unwrap()),
    )
    .unwrap();
    q.search = Some("SUPERMARKET".into());
    assert_eq!(ids(&records(&pool, &q, Some(("First", "DE"))).await), [2]);
    assert!(records(&pool, &q, Some(("Second", "DE"))).await.is_empty());
    q.search = Some("%".into());
    assert_eq!(ids(&records(&pool, &q, None).await), [2]);
    q.search = Some("_".into());
    assert!(records(&pool, &q, None).await.is_empty());
    q = query();
    q.search = Some("refund sender".into());
    assert_eq!(ids(&records(&pool, &q, None).await), [3]);
    q.accounts = vec![
        AccountId::try_from(1).unwrap(),
        AccountId::try_from(2).unwrap(),
    ];
    assert_eq!(ids(&records(&pool, &q, None).await), [3]);
}

#[tokio::test]
async fn limit_probes_one_extra_and_find_handles_missing() {
    let pool = fixture().await;
    let mut q = query();
    q.limit = RowLimit::Bounded(NonZeroU32::new(2).unwrap());
    assert_eq!(ids(&records(&pool, &q, None).await), [5, 3, 2]);
    assert!(
        transactions::find(&pool, TransactionId::try_from(99).unwrap())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        transactions::find(&pool, TransactionId::try_from(2).unwrap())
            .await
            .unwrap()
            .unwrap()
            .metadata
            .unwrap()
            .0["category"],
        "groceries"
    );
}

#[tokio::test]
async fn jsonl_preserves_content_and_missing_vs_null() {
    let pool = fixture().await;
    let row = transactions::find(&pool, TransactionId::try_from(2).unwrap())
        .await
        .unwrap()
        .unwrap();
    let mut output = Vec::new();
    let mut renderer = TransactionRenderer::new(&mut output, OutputFormat::Jsonl, None);
    renderer.write_record(&row).unwrap();
    renderer.finish().unwrap();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["metadata"]["category"], "groceries");
    assert_eq!(value["content"]["transaction_amount"]["amount"], "42.30");
    output.clear();
    let fields = "amount,metadata.review,metadata.missing,metadata.nested.weight"
        .parse()
        .unwrap();
    let mut renderer = TransactionRenderer::new(&mut output, OutputFormat::Jsonl, Some(fields));
    renderer.write_record(&row).unwrap();
    renderer.finish().unwrap();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["amount"], "-42.30");
    assert_eq!(value.get("metadata.review"), Some(&Value::Null));
    assert!(value.get("metadata.missing").is_none());
    assert_eq!(value["metadata.nested.weight"], 0.8);
}

#[tokio::test]
async fn lines_escape_and_tables_truncate_without_losing_json_data() {
    let pool = fixture().await;
    let mut row = transactions::find(&pool, TransactionId::try_from(3).unwrap())
        .await
        .unwrap()
        .unwrap();
    row.content.0.remittance_information = Some(vec![format!("a|b\nc\\d{}", "é".repeat(60))]);
    let mut output = Vec::new();
    let mut renderer = TransactionRenderer::new(
        &mut output,
        OutputFormat::Lines,
        Some("amount,description,metadata".parse().unwrap()),
    );
    renderer.write_record(&row).unwrap();
    renderer.finish().unwrap();
    let text = String::from_utf8(output).unwrap();
    assert!(text.starts_with("+42.30 | a\\|b\\nc\\\\d"));
    assert_eq!(text.lines().count(), 1);
    assert!(text.contains(&"é".repeat(60)));
    let mut output = Vec::new();
    let mut renderer = TransactionRenderer::new(&mut output, OutputFormat::Table, None);
    renderer.write_record(&row).unwrap();
    renderer.finish().unwrap();
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains('…'));
    assert!(!text.contains(&"é".repeat(60)));
}

#[tokio::test]
async fn empty_stream_and_broken_pipe() {
    let mut output = Vec::new();
    TransactionRenderer::new(&mut output, OutputFormat::Jsonl, None)
        .finish()
        .unwrap();
    assert!(output.is_empty());
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let pool = fixture().await;
    let row = transactions::find(&pool, TransactionId::try_from(2).unwrap())
        .await
        .unwrap()
        .unwrap();
    let mut renderer = TransactionRenderer::new(Broken, OutputFormat::Jsonl, None);
    assert!(
        matches!(renderer.write_record(&row), Err(crate::error::AppError::Io(e)) if e.kind() == std::io::ErrorKind::BrokenPipe)
    );
}

#[tokio::test]
async fn query_plans_use_booking_date_indexes() {
    let pool = fixture().await;
    for (account, index) in [
        (false, "idx_transactions_booking_date"),
        (true, "idx_transactions_account_booking_date"),
    ] {
        let mut q = query();
        q.limit = RowLimit::Bounded(NonZeroU32::new(50).unwrap());
        if account {
            q.accounts = vec![AccountId::try_from(1).unwrap()];
            q.dates = BookingDateRange::new(Some("2026-01-01".parse().unwrap()), None).unwrap();
        }
        let builder = transactions::list_query(&q, None);
        let mut explain = sqlx::QueryBuilder::<sqlx::Sqlite>::new("EXPLAIN QUERY PLAN ");
        explain.push(builder.sql().as_str());
        let mut sql = explain.build_query_as::<(i64, i64, i64, String)>();
        if account {
            sql = sql.bind(1i64).bind("2026-01-01");
        }
        let plans = sql.bind(51i64).fetch_all(&pool).await.unwrap();
        assert!(plans.iter().any(|p| p.3.contains(index)), "{plans:?}");
        assert!(
            !plans.iter().any(|p| p.3.contains("TEMP B-TREE")),
            "{plans:?}"
        );
    }
}

#[test]
fn cli_rejects_invalid_inputs_at_boundary() {
    for args in [
        vec!["--limit", "0"],
        vec!["--fields", "metadata..x"],
        vec!["--last", "0d"],
        vec!["--all", "--limit", "10"],
        vec!["--last", "1d", "--from", "2026-01-01"],
    ] {
        let mut command = vec!["banker", "transaction", "list"];
        command.extend(args);
        assert!(Cli::try_parse_from(command).is_err());
    }
    for args in [
        vec!["--all"],
        vec!["--from", "2026-02-01", "--to", "2026-01-01"],
    ] {
        let mut command = vec!["banker", "transaction", "list"];
        command.extend(args);
        let cli = Cli::try_parse_from(command).unwrap();
        let Commands::Transaction {
            action: TransactionAction::List { output, query, .. },
        } = cli.command
        else {
            panic!()
        };
        assert!(query.parse_request(output.format()).is_err());
    }
    let cli = Cli::try_parse_from(["banker", "transaction", "list", "--all", "--jsonl"]).unwrap();
    let Commands::Transaction {
        action: TransactionAction::List { output, query, .. },
    } = cli.command
    else {
        panic!()
    };
    assert!(query.parse_request(output.format()).is_ok());
}
