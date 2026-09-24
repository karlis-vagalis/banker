# Inspect and fetch records

Distinguish local list commands from live API fetches:

```sh
# Read already-synced data locally
banker account list
banker balance current
banker transaction list

# Fetch live data for an account UID shown by account list
banker account fetch <ACCOUNT_UID> --app personal
banker balance fetch <ACCOUNT_UID> --app personal
banker transaction fetch <ACCOUNT_UID> --app personal --last 7d
```

Local list commands accept `--bank <alias>` when multiple banks are configured. Live fetch commands accept `--app <key>` when multiple applications are configured. Omitting a time frame from `transaction fetch` requests the longest available history, just like `--all`. Specify `--last` or `--from`/`--to` for a bounded range; get the user's approval before fetching full history. RFC 3339 timestamps are accepted, but only their UTC calendar dates are sent to Enable Banking.

For shell pipelines, choose one of:

```sh
banker transaction list --jsonl
banker account list --lines
```

Live `fetch --jsonl` emits API records. Local `list --jsonl` wraps each record in `content` alongside its local `id` and timestamp; balance and transaction records also include `account_id`. `--lines` emits pipe-separated fields and may include IBANs. Treat both formats as sensitive; avoid dumping complete output into chat or logs. Summarize only the fields the user needs.
