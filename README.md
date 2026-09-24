# Banker

> **AI disclosure:** AI tools were used to assist with code and documentation in this repository. The maintainer reviews changes and is responsible for the project.

A command-line client for [Enable Banking](https://enablebanking.com/) that links to your banks, syncs account data, and lets you inspect it locally from a SQLite database.

> **Status:** Early-stage personal project. The CLI and configuration format may change. It is not affiliated with or endorsed by Enable Banking or any bank.

## Features

- Authorize and manage bank sessions through Enable Banking.
- Sync account details, balances, and transactions into a local SQLite database.
- Browse local records or fetch individual resources from the API.
- Add validated JSON metadata to accounts, balances, and transactions.
- Print tables, line-delimited JSON (`--jsonl`), or pipe-friendly text (`--lines`).
- Install an optional systemd user timer for periodic synchronization.

## Requirements

- Rust toolchain (edition 2024; install with [rustup](https://rustup.rs/)).
- An Enable Banking application with its application ID and matching private key.
- A bank supported by Enable Banking in your region.

## Installation

Install the latest source from GitHub:

```sh
cargo install --git https://github.com/karlis-vagalis/banker.git
```

Or build from a local checkout:

```sh
git clone https://github.com/karlis-vagalis/banker.git
cd banker
cargo install --path .
```

The crate is not yet published on crates.io.

## Configuration

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
# See available banks (requires one application in config, or pass --app)
banker bank --country DE

# Start bank authorization; opens the authorization URL in your browser
banker auth login --bank mybank

# Check or revoke linked sessions
banker auth status
# banker auth logout --bank mybank

# Sync the last seven days (the default)
banker sync

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

`--from` and `--to` accept `YYYY-MM-DD` dates or RFC 3339 timestamps. Alternatively, `--last` accepts durations such as `30d` or `1month`. Use `banker <command> --help` for the complete command options.

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

The source manifest stays at version `0.0.0`. Install [`doxxer`](https://github.com/karlis-vagalis/doxxer), then run `just release` from a clean working tree to create and push the next `vX.Y.Z` tag on the current commit. The cargo-dist-generated GitHub workflow injects that tag's version while building the binaries; its crates.io publish job injects the same version before publishing.

Before the first release, add a `CARGO_REGISTRY_TOKEN` secret to the GitHub repository. The release workflow creates a GitHub Release with Linux and Windows assets, then publishes the crate to crates.io.

## Development

```sh
cargo build
cargo test
cargo run -- --help
```

The Enable Banking client is generated at build time from `enablebanking-api.yaml`.

## License

Licensed under the GNU Affero General Public License, version 3 or (at your option) any later version. See [LICENSE](LICENSE) for the full license text.
