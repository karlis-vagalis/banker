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

Local list commands accept `--bank <alias>` when multiple banks are configured. Live fetch commands accept `--app <key>` when multiple applications are configured. `transaction fetch` needs a time frame (`--last`, `--from`/`--to`, or `--all`); keep the range bounded unless full history is explicitly requested.

For shell pipelines, choose one of:

```sh
banker transaction list --jsonl
banker account list --lines
```

`--jsonl` emits full JSON records. `--lines` emits pipe-separated fields and may include IBANs. Treat both as sensitive; avoid dumping complete output into chat or logs. Summarize only the fields the user needs.
