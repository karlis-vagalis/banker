---
name: banker
description: "Help users operate the installed Banker CLI binary for Enable Banking: configure it, authorize banks, sync and inspect data, manage metadata, and troubleshoot commands. Use whenever the user asks how to use or operate banker."
---

# Banker CLI

Use the CLI to manage Enable Banking authorizations and locally synced financial data. Work from the user's request, choose the narrowest relevant workflow reference below, and check the installed CLI's help for current flags before assuming syntax:

```sh
banker self list
banker <command> --help
```

Use `banker self list` to see available commands. The global `--config <path>` option goes before the command.

## Workflow references

| User task | Read |
|---|---|
| Install, configure, authorize, or disconnect a bank | [Setup and authentication](references/setup-and-auth.md) |
| Sync account data or choose a date range | [Sync](references/sync.md) |
| List local records, fetch live data, or use output formats | [Inspect and fetch records](references/records.md) |
| Manage metadata or periodic systemd sync | [Metadata and scheduling](references/metadata-and-scheduling.md) |
| Diagnose a configuration, authorization, or CLI error | [Troubleshooting](references/troubleshooting.md) |
| Explicit schema question | [Database reference](references/db/index.md) |

## Safety

Treat private keys, authorization redirect URLs, session details, IBANs, transaction descriptions, and command output as sensitive. Never ask the user to paste secrets or full financial records into chat. Prefer summaries and narrow date ranges. Ask before revoking sessions, requesting full transaction history, installing or removing a recurring timer, or bypassing an interactive confirmation with `--yes`.
