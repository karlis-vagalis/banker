# Seeded integration-test database

`tests/support/mod.rs` provides one reusable entry point (implemented in `tests/support/seed.rs`, shared with the CLI demo):

```rust
mod support;

use support::{FixtureCommand, SeedSpec, seeded_database};

// Inside an async test:
let fixture = seeded_database(SeedSpec::default()).await.unwrap();
let output = fixture.command()
    .args(["transaction", "list", "--bank", "pine", "--all", "--jsonl"])
    .output()
    .unwrap();
assert!(output.status.success());
```

## Default layout

Transaction counts are **per bank**, split evenly over its accounts.

| Config key | Fictional bank | Country | Accounts | Transactions |
| --- | --- | --- | --- | --- |
| `birch` | Fixture Birch | DE | 2 | 10 |
| `linden` | Fixture Linden | LV | 2 | 100 |
| `oak` | Fixture Oak | NL | 2 | 1,000 |
| `pine` | Fixture Pine | DE | 2 | 10,000 |
| Total | | | 8 | 11,110 |

The returned manifest contains each bank, its typed account IDs, currencies, and expected transaction counts. `db_path()` and `config_path()` allow direct database assertions or CLI invocation; `command()` always selects the fixture config.

## Data variations

- EUR, USD, GBP, and CHF accounts, with matching transaction currencies.
- Exact decimal-string amounts generated without floating-point arithmetic.
- Credits and debits, booked and pending transactions, missing booking dates, and missing bank references.
- Fixed booking/value dates spanning 400 days from 2025-01-01, including shared dates for tie-break tests.
- Populated metadata, no metadata row, empty metadata objects, explicit null values, nested numeric weights, tags, and review objects.
- Unicode names, multiline descriptions, pipes, backslashes, `%`, and `_`.
- Valid account content, unique identities/references, fixed insertion/update timestamps, and content hashes.

Payloads and IDs are deterministic for a given layout; no random seed or wall-clock-dependent records are used. Only the temporary directory name varies. Normal application migrations initialize each database. Batched inserts run in one transaction, then the database is checkpointed and its pool closed before returning.

## Custom sizes and empty accounts

```rust
use std::num::NonZeroU8;
use support::{BankSeed, SeedSpec, TestBank, seeded_database};

let spec = SeedSpec::new([
    BankSeed::new(TestBank::Birch, NonZeroU8::new(3).unwrap(), 7),
    BankSeed::new(TestBank::Pine, NonZeroU8::new(5).unwrap(), 100_000),
]).unwrap();
let fixture = seeded_database(spec).await.unwrap();
```

The first bank receives account counts `[3, 2, 2]`. A bank transaction count of zero creates empty accounts. Layouts must contain at least one bank, with no duplicate bank identities; account counts use `NonZeroU8`, so zero accounts cannot be represented.

Every fixture owns its own temporary directory, SQLite file, and config, enabling parallel tests without touching user data. Dropping the fixture removes the directory. The [CLI demo](../examples/readme.md) uses the same automatic cleanup. Close any pools you open against it before dropping it. Application credentials are dummy values, and the fixture API URL points at loopback port 9 rather than the live service; this fixture is intended for local commands, not API/sync tests.

## Running tests

```sh
cargo test --test transactions

# Explicitly run the larger 100,000-transaction streaming test:
cargo test --test transactions hundred_thousand -- --ignored
```

The larger test reads output incrementally rather than collecting the whole export. There are no timing thresholds: these are correctness tests, not performance benchmarks. SQLx may emit slow-query warnings on stderr during larger exports; tests verify complete machine-readable stdout and successful exit rather than requiring silence on stderr.

The small, hand-crafted fixture in `tests/transaction_query.rs` remains separate for exact boundary, ordering, and renderer assertions.
