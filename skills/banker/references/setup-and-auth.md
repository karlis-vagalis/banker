# Install, configure, and authorize

## Install

The crate is available on crates.io:

```sh
cargo install banker
```

See the [project README](../../../README.md) for additional usage examples.

## Configuration

Default config is `~/.config/banker/config.toml` on Linux; paths vary by platform. The file requires at least one Enable Banking application:

```toml
# Optional top-level settings:
# db_path = "/path/to/banker/data.db"
# redirect_url = "https://your-registered-redirect.example/callback"

[application.personal]
id = "YOUR_ENABLEBANKING_APP_ID"
key_file = "/absolute/path/to/enablebanking-private-key.pem"

[bank.mybank]
name = "Exact ASPSP name returned by Enable Banking"
country = "DE"
```

Keep the config, session file, and private key out of version control. The private key stays at `key_file`; Banker reads it to sign API requests. The database defaults to the platform's local data directory (`~/.local/share/banker/data.db` on Linux). `--config <path>` is a global option placed before the command, e.g. `banker --config /path/config.toml auth status`.

Use the exact provider name and ISO country code in bank aliases. Find providers with `banker bank --country DE` or `banker bank --search name`. `banker bank` currently uses the sole configured application and has no `--app` option. When multiple applications are configured, pass `--app <key>` to commands that support it; when multiple banks are configured, select one with `--bank <key>` where supported.

## Authorization

```sh
banker auth login --bank mybank
banker auth status
banker auth logout --bank mybank
```

Login opens the authorization URL unless `--no-browser` is set, then prompts for the full redirect URL. The redirect contains a short-lived authorization code: have the user paste it directly into the local CLI prompt, never into chat. Session lifetime defaults to 90 days and can be set with `--valid-days`.

Status checks sessions remotely and prints session details; don't copy those details into public logs. Logout revokes the remote session and removes it locally, so only do this when the user explicitly asks to disconnect. On Linux the session file is `~/.config/banker/session.json`; on Unix it is owner-only.
