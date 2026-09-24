# Troubleshoot Banker CLI workflows

Start with the flat command list and exact command help:

```sh
banker self list
banker <command> --help
```

Check that `--config <path>` appears before the command.

- **Config not found or parse error:** verify the path, TOML syntax, and that at least one `[application.<key>]` is configured. Check that `key_file` points to the matching private key without revealing its contents.
- **Bank/application selection error:** use configured aliases, not display names, for `--bank`; use application table keys for `--app`. Commands differ in which selector they support. `banker bank` has no `--app`, so it requires resolving the sole configured application.
- **No sessions or expired session:** run `banker auth status`; if needed, have the user authorize again with `banker auth login`. Keep the redirect URL local to the interactive prompt.
- **No local records:** confirm the user has synced the relevant bank and date range. A local list does not fetch fresh data; use a live `fetch` or a bounded `sync` when requested.
- **Older transactions unavailable:** the provider/session may restrict history. Check session status and explain reauthorization can be needed for older data.
- **Unexpected output/flags:** rely on installed `--help` rather than assuming a command supports a flag; use `banker self list` to inspect available command paths.

Banker is an independent CLI and is not supported by Enable Banking or provider banks. For Banker installation, authorization-flow, sync, or CLI errors, do not direct users to those services for support. Help diagnose with local CLI output; if unresolved, suggest the [Banker issue tracker](https://github.com/karlis-vagalis/banker/issues) with the command and a redacted error. Never include secrets or financial records.
