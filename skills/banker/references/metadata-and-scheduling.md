# Metadata and scheduled sync

## Metadata

Accounts, balances, and transactions support `metadata set`, `get`, `delete`, and `schema` subcommands. Use IDs from the corresponding local list output. Metadata can be supplied as inline JSON, a file path, `-` for stdin, or stdin when the argument is omitted:

```sh
banker transaction metadata set <ID> '{"category":"groceries"}'
banker transaction metadata get <ID>
banker account metadata schema get
```

Overwriting existing metadata, replacing a schema, and deleting a schema prompt for confirmation. Do not pass `--yes` unless the user explicitly asks to bypass the prompt. Replacing a schema affects future validation and may leave existing metadata nonconforming.

## Periodic sync (Linux/systemd)

`banker self systemd install` installs and enables a user timer that runs bare `banker sync` four times daily. Bare sync requests the longest available transaction history for every active session; it is not a recent-only incremental sync. Explain this broad, recurring data access and get approval before installing. Use `banker self systemd uninstall` only when the user asks to remove it.
