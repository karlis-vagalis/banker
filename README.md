# Banker

[![skills.sh](https://skills.sh/b/karlis-vagalis/banker)](https://skills.sh/karlis-vagalis/banker)

> **AI disclosure:** AI tools were used to assist with code and documentation in this repository. The maintainer reviews changes and is responsible for the project.

A command-line client for [Enable Banking](https://enablebanking.com/) that links to your banks, syncs account data, and lets you inspect it locally from a SQLite database.

> **Status:** Early-stage personal project. The CLI and configuration format may change. It is not affiliated with or endorsed by Enable Banking or any bank.

## Why Banker?

I built Banker to keep transactions locally for querying and lightweight bookkeeping, including categories, tags, and other metadata. Enable Banking provides standardized access to banks for syncing; the local data can also help AI agents analyze transactions and suggest or apply categories with LLMs.

## Support

Enable Banking and provider banks do not support Banker. For issues using this CLI, please do not contact them; report the problem in the [Banker issue tracker](https://github.com/karlis-vagalis/banker/issues) with the command and a redacted error message. Never include private keys, session details, IBANs, or transaction data in an issue.

## Features

- Authorize and manage bank sessions through Enable Banking.
- Sync account details, balances, and transactions into a local SQLite database.
- Browse local records or fetch individual resources from the API.
- Add validated JSON metadata to accounts, balances, and transactions.
- Print tables, line-delimited JSON (`--jsonl`), or pipe-friendly text (`--lines`). Local JSONL records include `id`, timestamps, and bank data under `content`; balance/transaction records also include `account_id`.
- Install an optional systemd user timer for periodic synchronization.

## Installation

Install the latest published version from crates.io with Cargo (requires Rust and Cargo):

```sh
cargo install banker
```

To update an existing Cargo installation, run `cargo install banker --force`. If you don't have a Rust toolchain, download a prebuilt Linux or Windows binary from the [latest GitHub Release](https://github.com/karlis-vagalis/banker/releases/latest); no toolchain is needed to run the binary.

## Configuration

You'll need an Enable Banking application ID, its matching private key, and a bank supported by Enable Banking in your region.

Check out the official guide [here](https://enablebanking.com/docs/api/quick-start/#registering-an-application).

Create `~/.config/banker/config.toml` (or pass another path with `--config`). Keep this file and your private key out of version control. Replace the example values with your own Enable Banking application details and the actual ASPSP name/country returned by the service:

```toml
# Optional. Defaults to your platform's local data directory:
# ~/.local/share/banker/data.db on Linux.
# db_path = "/path/to/banker/data.db"

[application.personal]
id = "YOUR_ENABLEBANKING_APP_ID"
key_file = "/path/to/your/enablebanking-private-key.pem"

# Optional bank aliases for login and sync filtering.
[bank.mybank]
name = "ASPSP name from Enable Banking"
country = "DE"

# Optional; by default the first redirect URL registered on your application is used.
# redirect_url = "https://your-registered-redirect.example/callback"

# Optional; defaults to https://api.enablebanking.com.
# api_base_url = "https://api.enablebanking.com"
```

On Linux, create the configuration directory and restrict its permissions:

```sh
mkdir -p ~/.config/banker
chmod 700 ~/.config/banker
chmod 600 ~/.config/banker/config.toml
```

The configured key file must be the private key matching the Enable Banking application. Never commit or share it. If you configure multiple applications or banks, select one with `--app <name>` or `--bank <name>` where the command supports it.

## Quick start

```sh
# List available banks (uses the sole configured application)
banker bank --country DE

# Start bank authorization; opens the authorization URL in your browser
banker auth login --bank mybank

# Check or revoke linked sessions
banker auth status
# banker auth logout --bank mybank

# Sync a recent period explicitly; without a time frame, sync requests the longest available history
banker sync --last 7d

# Sync a specific period or all available history
banker sync --from 2025-01-01 --to 2025-02-01
banker sync --all

# Browse locally stored data
banker account list
banker balance current
banker transaction list

# Emit machine-readable output (the same default limit of 50 applies)
banker transaction list --jsonl
# Explicitly stream all matching transactions, including local metadata
banker transaction list --all --jsonl
banker account list --lines
```

For live fetch/sync commands, `--from` and `--to` accept `YYYY-MM-DD` dates or RFC 3339 timestamps; only the UTC calendar date is sent to Enable Banking, so time-of-day precision is not preserved. Alternatively, `--last` accepts positive durations such as `30d` or `1month`. Omitting a time-frame option requests the longest transaction history available; use `--last` or a bounded `--from`/`--to` range for a narrower sync. `--all` also requests the full available history. Sync exits nonzero if any selected session or account fails, even when other accounts were stored successfully. Use `banker <command> --help` for the complete command options.

### Querying local transactions

`transaction list` returns the newest 50 matching records by default, ordered by booking date and then transaction ID descending. Undated transactions come last. A message on stderr indicates when more records match; use `--limit <positive-number>` to increase the bound, or `--all --jsonl` / `--all --lines` to stream every match without loading the full result into memory. Unbounded tables are not supported. Empty matches succeed without a sync warning.

```sh
# Account filters can be repeated; combine with --bank when needed
banker transaction list --account <ACCOUNT-ID> --last 30d

# Local booking dates: inclusive --from, exclusive --to (either may be omitted)
banker transaction list --from 2026-01-01 --to 2026-02-01 --search supermarket

# Select and order fields, including nested local metadata keys
banker transaction list --fields id,date,amount,currency,counterparty,metadata.category

# Inspect the complete bank record and metadata without truncation
banker transaction show <TRANSACTION-ID>
banker transaction show <TRANSACTION-ID> --jsonl
```

Local date filters accept `YYYY-MM-DD`. Unlike live fetch/sync time frames, local `--last` is mutually exclusive with date bounds, and `--all` controls the **row limit**, not the date range. Relative windows include today and start at the UTC calendar date of now minus the duration. Search is a literal, ASCII-case-insensitive substring match across the description and direction-appropriate counterparty; `%`, `_`, and backslashes are not wildcards.

Available fields: `id`, `account_id`, `date`, `amount`, `currency`, `counterparty`, `description`, `updated_at`, `metadata`, and `metadata.<key>[.<key>...]`. Metadata paths address object keys (letters, digits, `_`, or `-`), not array indices. Duplicate or unknown fields are rejected. Tables truncate long cells; `show` and streaming formats preserve complete values. Table/lines defaults are `id,date,amount,currency,counterparty,description`, with signed decimal-string amounts.

Default JSONL retains `id`, `account_id`, `updated_at`, and the original bank `content`, and adds `metadata` (JSON `null` when no annotation exists). With `--fields`, each JSONL object contains only selected fields; metadata paths use literal dotted output keys, such as `"metadata.category"`. Missing fields are omitted, whereas explicit JSON nulls remain `null`. In tables/lines, missing values appear as `—` and explicit nulls as `null`.

`--lines` uses ` | ` separators, with backslash escapes for `\\`, `|`, newline, carriage return, and tab. Prefer JSONL for robust integrations. Limits and filters apply identically to every format, and status messages go to stderr, never JSONL stdout. `show --lines` emits one complete JSON object on one line.

### Metadata

Accounts, balances, and transactions support `metadata set`, `get`, `delete`, and `schema` subcommands. Metadata is stored locally and validated against the active JSON Schema when one is configured. For example:

```sh
banker transaction metadata set <ID> '{"category":"groceries"}'
banker transaction metadata get <ID>
```

Use `banker account metadata --help` (or the balance/transaction equivalents) for schema management and input options.

## Data and security

- Synced account information, balances, transactions, and metadata are stored in a local SQLite database. The default is your operating system's per-user local data directory under `banker/`; override it with `db_path`.
- Authorization sessions are stored in `~/.config/banker/session.json` on Linux (or the corresponding platform configuration directory). On Unix, the session file is written with owner-only permissions.
- The private key stays at the path configured by `key_file`; Banker reads it to authenticate API requests.
- Syncing sends requests to Enable Banking and retrieves financial data from the providers you authorize. Treat the database, session file, configuration, and backups as sensitive. Protect them with appropriate filesystem permissions and disk encryption.
- Banker is an early-stage project. Review the code and your provider's terms before connecting a real account; use it at your own risk.

## Try synthetic data

From a source checkout, run example subcommands against a freshly seeded database, with automatic cleanup:

```sh
cargo run --example transactions -- accounts
cargo run --example transactions -- list --bank pine --limit 10
cargo run --example transactions -- list --fields id,date,amount,currency,metadata.category
cargo run --example transactions -- --help
```

To keep a larger example database and run your own commands:

```sh
CONFIG=$(cargo run --quiet --example transaction-playground -- --pine-transactions 100000)
cargo run -- --config "$CONFIG" account list
cargo run -- --config "$CONFIG" transaction list --bank pine --limit 10
```

This creates 8 accounts across 4 fictional banks, with 101,110 transactions and mixed metadata/currencies. See [examples/readme.md](examples/readme.md) for more commands, smaller datasets, and cleanup instructions.

## Development

Building and testing from source requires a Rust toolchain supporting edition 2024; install one with [rustup](https://rustup.rs/).

```sh
cargo build
cargo test
cargo run -- --help
```

Reusable seeded end-to-end databases are documented in [tests/fixtures.md](tests/fixtures.md), including custom sizes and the optional 100,000-transaction test.

The Enable Banking client is generated at build time from `enablebanking-api.yaml`.

## License

Licensed under the GNU Affero General Public License, version 3 or (at your option) any later version. See [LICENSE](LICENSE) for the full license text.
