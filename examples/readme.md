# Transaction playground

Create synthetic data using the same generator as the end-to-end tests, then run the real CLI against it. No bank connection, credentials, or changes to your normal configuration are required.

Run these commands from the project root in Bash or another POSIX shell:

```sh
# Default: 8 accounts, 4 banks, 11,110 transactions in total.
CONFIG=$(cargo run --quiet --example transaction-playground)

cargo run -- --config "$CONFIG" account list
cargo run -- --config "$CONFIG" transaction list --bank pine --limit 10
```

The example writes **only the generated config path to stdout**, so it can be captured in `CONFIG`. Dataset details, account IDs, and suggested commands go to stderr. Each run creates a new temporary directory and deliberately keeps it after exit. The config uses an absolute database path, so it works from another directory too.

For a larger dataset:

```sh
# 100,000 transactions in Pine, plus 10 / 100 / 1,000 in the other banks.
# Still 8 accounts and 4 banks; 101,110 transactions in total.
CONFIG=$(cargo run --quiet --example transaction-playground -- --pine-transactions 100000)

# Optionally change the number of accounts per bank (must be nonzero).
CONFIG=$(cargo run --quiet --example transaction-playground -- \
  --pine-transactions 100000 --accounts-per-bank 2)
```

## Commands to explore

```sh
# Compact output with selected annotations.
cargo run -- --config "$CONFIG" transaction list --bank pine \
  --fields id,date,amount,currency,metadata.category,metadata.nested.weight

# Description search within an explicit booking-date range.
cargo run -- --config "$CONFIG" transaction list \
  --from 2025-02-01 --to 2025-03-01 --search household --jsonl

# Use an account ID printed by the example or account list.
cargo run -- --config "$CONFIG" transaction list --account <ACCOUNT-ID> --limit 10

# Use a transaction ID from transaction list for full bank content and metadata.
cargo run -- --config "$CONFIG" transaction show <TRANSACTION-ID>

# Export the largest bank without the default 50-row limit.
cargo run -- --config "$CONFIG" transaction list --bank pine --all --jsonl > transactions.jsonl
```

Replace `<ACCOUNT-ID>` and `<TRANSACTION-ID>` before running those commands. For a quieter shell after the first compilation, use `cargo run --quiet -- ...`.

## What's in the database?

Bank keys: `birch`, `linden`, `oak`, and `pine`. Amounts use EUR, USD, GBP, and CHF, with credits and debits. Records mix absent metadata, empty metadata objects, populated nested annotations, tags, and explicit nulls. Descriptions include Unicode, newlines, and delimiters. Some records are pending or lack booking dates/references.

Dates are deterministic, starting at **2025-01-01** and spanning 400 days. Recent `--last` windows may return no matches; use the absolute bounds above to inspect populated periods.

The fixture is for **local commands only**. API/auth/sync commands are not demonstrated: the config has dummy credentials and points at loopback, not Enable Banking. Account and transaction data are seeded; balances and authorization sessions are not.

Files are not automatically removed after the example exits. Delete the generated directory printed by the example when finished. Re-running the example creates another independent dataset rather than overwriting a previous one.

For fixture customization in tests, see [tests/fixtures.md](../tests/fixtures.md). The implementation is shared through `tests/support/seed.rs`; no duplicate generator or production CLI seeding command is added.
