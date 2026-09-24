# Sync account data

`banker sync` processes active, unexpired sessions for all linked banks unless filtered with `--bank`. It stores account and balance data and fetches transactions from Enable Banking.

**Important:** with no time-frame option, sync requests the longest transaction history available. It is not a recent-only incremental sync. For routine narrow syncs, specify a period:

```sh
banker sync --last 7d
banker sync --bank mybank --last 30d
banker sync --from 2025-01-01 --to 2025-01-31
```

- `--bank` takes a configured bank alias and may be repeated.
- `--from` and `--to` must be supplied together. Accepts `YYYY-MM-DD` or RFC 3339 timestamps.
- `--last` accepts durations such as `30d` or `1month`.
- `--all` explicitly requests the entire available history; it has the same broad-history effect as omitting the time frame.

Before running a broad sync, tell the user which banks and range it will cover and get explicit confirmation. Prefer bounded ranges. A session may have limited access to older transactions; if old history is unavailable, check `banker auth status` and explain that reauthorization may be needed.

If there are no sessions, guide the user through `banker auth login` using [the local authorization flow](setup-and-auth.md). Never ask them to share the redirect URL, session token, or private key.
