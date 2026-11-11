use banker::{
    cli::{Cli, Commands, TransactionAction, time_frame_from_args},
    models::{Lookback, transaction_query::RowLimit},
};
use chrono::{DateTime, Utc};
use clap::Parser;

fn live_time_frame(
    prefix: &[&str],
    flags: &[&str],
) -> Result<Option<banker::models::TimeFrame>, String> {
    let args = prefix.iter().chain(flags).copied();
    let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;
    match cli.command {
        Commands::Sync {
            time_frame_args, ..
        } => time_frame_from_args(time_frame_args),
        Commands::Transaction {
            action: TransactionAction::Fetch {
                time_frame_args, ..
            },
        } => time_frame_from_args(time_frame_args),
        _ => panic!("expected a live time-frame consumer"),
    }
}

#[test]
fn shared_date_group_preserves_live_time_frame_behavior() {
    for prefix in [
        vec!["banker", "sync"],
        vec!["banker", "transaction", "fetch", "account-uid"],
    ] {
        assert!(live_time_frame(&prefix, &[]).unwrap().is_none());
        assert!(live_time_frame(&prefix, &["--all"]).unwrap().is_none());
        let frame = live_time_frame(&prefix, &["--last", "30d"])
            .unwrap()
            .unwrap();
        assert_eq!(frame.to - frame.from, chrono::TimeDelta::days(30));
        let frame = live_time_frame(
            &prefix,
            &["--from", "2026-01-01", "--to", "2026-02-01T01:00:00+01:00"],
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            frame.from,
            "2026-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
        );
        assert_eq!(
            frame.to,
            "2026-02-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
        );
        // Live API bounds retain their existing allowance for equal endpoints.
        assert!(
            live_time_frame(&prefix, &["--from", "2026-01-01", "--to", "2026-01-01"])
                .unwrap()
                .is_some()
        );
        for flags in [
            vec!["--from", "2026-01-01"],
            vec!["--to", "2026-02-01"],
            vec!["--last", "0d"],
            vec!["--last=-1d"],
            vec!["--last", "1d", "--from", "2026-01-01", "--to", "2026-02-01"],
            vec!["--all", "--last", "1d"],
            vec!["--all", "--from", "2026-01-01", "--to", "2026-02-01"],
            vec!["--from", "2026-02-01", "--to", "2026-01-01"],
        ] {
            assert!(
                live_time_frame(&prefix, &flags).is_err(),
                "{prefix:?} {flags:?}"
            );
        }
    }
}

#[test]
fn shared_date_group_keeps_local_bounds_and_all_independent() {
    for flags in [
        vec!["--from", "2026-01-01"],
        vec!["--to", "2026-02-01"],
        vec!["--from", "2026-01-01", "--to", "2026-02-01"],
        vec!["--last", "30d"],
    ] {
        let mut args = vec!["banker", "transaction", "list", "--all", "--jsonl"];
        args.extend(&flags);
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::Transaction {
            action: TransactionAction::List { query, output, .. },
        } = cli.command
        else {
            panic!()
        };
        let (query, _, _) = query.parse_request(output.format()).unwrap().into_parts();
        assert!(matches!(query.limit, RowLimit::All));
        if flags[0] == "--from" {
            assert_eq!(query.dates.from(), Some("2026-01-01".parse().unwrap()));
        }
        if flags[0] == "--to" {
            assert_eq!(query.dates.to(), Some("2026-02-01".parse().unwrap()));
        }
        if flags[0] == "--last" {
            assert_eq!(query.dates.to(), Utc::now().date_naive().succ_opt());
        }
    }
    assert!(
        Cli::try_parse_from([
            "banker",
            "transaction",
            "list",
            "--from",
            "2026-01-01T12:00:00Z"
        ])
        .is_err()
    );
    let cli = Cli::try_parse_from([
        "banker",
        "transaction",
        "list",
        "--from",
        "2026-01-01",
        "--to",
        "2026-01-01",
    ])
    .unwrap();
    let Commands::Transaction {
        action: TransactionAction::List { query, output, .. },
    } = cli.command
    else {
        panic!()
    };
    assert!(query.parse_request(output.format()).is_err());
}

#[test]
fn shared_lookback_carries_positive_duration_proof() {
    for input in ["0d", "-1d", "garbage"] {
        assert!(input.parse::<Lookback>().is_err());
    }
    let now = "2026-02-01T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
    assert_eq!(
        "1d".parse::<Lookback>().unwrap().start_at(now).unwrap(),
        "2026-01-31T12:00:00Z".parse::<DateTime<Utc>>().unwrap()
    );
}
