# Seeded CLI demo

Run the real CLI against synthetic data, without configuration or bank credentials:

```sh
cargo run --example demo -- --help
cargo run --example demo -- transaction list --help
cargo run --example demo -- account list
cargo run --example demo -- transaction list --bank birch
cargo run --example demo -- transaction list --bank pine --limit 10 \
  --fields id,date,amount,currency,metadata.category,metadata.nested.weight
cargo run --example demo -- transaction list --from 2025-02-01 --to 2025-03-01 --search household --jsonl
cargo run --example demo -- transaction list --bank pine --all --jsonl > transactions.jsonl
```

Everything after `--` uses the actual Banker CLI parser and normal dispatcher. There are no example-only dataset flags or renamed subcommands.

Use an account ID from `account list` with `transaction list --account <ACCOUNT-ID>`, or a transaction ID from `transaction list` with `transaction show <TRANSACTION-ID>`:

```sh
cargo run --example demo -- transaction list --account <ACCOUNT-ID> --limit 10
cargo run --example demo -- transaction show <TRANSACTION-ID>
```

Replace the placeholders before running those commands. For quieter Cargo output, use `cargo run --quiet --example demo -- ...`.

## Dataset and lifecycle

Each invocation seeds the same deterministic database: **11,110 transactions across 8 accounts and 4 fictional banks**. Bank keys are `birch`, `linden`, `oak`, and `pine`, with 10, 100, 1,000, and 10,000 transactions respectively.

Currencies include EUR, USD, GBP, and CHF. Records mix credits/debits, missing metadata, empty metadata objects, nested annotations, tags, explicit nulls, Unicode, multiline descriptions, pending statuses, and missing booking dates/references.

Dates start at **2025-01-01** and span 400 days. Recent `--last` windows may return no matches; use the absolute date bounds above to inspect populated periods.

IDs are stable between invocations. The generated database and config are deleted automatically after every command; no manual cleanup is needed.

For safety, the demo supports local read commands (`transaction list/show`, `account list`, `balance list/current`) and supplies its own isolated config. Explicit `--config` overrides and other operations are rejected before seeding. Balances and authorization sessions are not seeded, so balance commands return empty results. Help and argument-syntax errors are handled before seeding.

The generator is shared with end-to-end tests through `tests/support/seed.rs`. For custom test layouts and the optional 100,000-transaction test, see [tests/fixtures.md](../tests/fixtures.md).
