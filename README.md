# Banker

[![skills.sh](https://skills.sh/b/karlis-vagalis/banker)](https://skills.sh/karlis-vagalis/banker)

> **AI disclosure:** AI tools were used to assist with code and documentation in this repository. The maintainer reviews changes and is responsible for the project.

A command-line client for [Enable Banking](https://enablebanking.com/) that links to your banks, syncs account data, and lets you inspect it locally from a SQLite database.

> **Status:** Early-stage personal project. The CLI and configuration format may change. It is not affiliated with or endorsed by Enable Banking or any bank.

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

# Emit machine-readable or pipe-friendly output
banker transaction list --jsonl
banker account list --lines
```

`--from` and `--to` accept `YYYY-MM-DD` dates or RFC 3339 timestamps; only the UTC calendar date is sent to Enable Banking, so time-of-day precision is not preserved. Alternatively, `--last` accepts positive durations such as `30d` or `1month`. Omitting a time-frame option requests the longest transaction history available; use `--last` or a bounded `--from`/`--to` range for a narrower sync. `--all` also requests the full available history. Sync exits nonzero if any selected session or account fails, even when other accounts were stored successfully. Use `banker <command> --help` for the complete command options.

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

## Releasing

The source manifest stays at version `0.0.0`; release workflows derive the package version from the tag. To release, manually create and push a `vX.Y.Z` tag at the desired commit. GitHub Actions builds the Linux and Windows assets, creates a GitHub Release, and publishes the crate to crates.io using Trusted Publishing (GitHub OIDC) with the `crates-io` environment. No long-lived crates.io API token is required.

## Development

Building and testing from source requires a Rust toolchain supporting edition 2024; install one with [rustup](https://rustup.rs/).

```sh
cargo build
cargo test
cargo run -- --help
```

The Enable Banking client is generated at build time from `enablebanking-api.yaml`.

## License

Licensed under the GNU Affero General Public License, version 3 or (at your option) any later version. See [LICENSE](LICENSE) for the full license text.
