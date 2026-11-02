use banker::cli::{Cli, Commands, SelfFormat, SystemdAction};
use clap::{CommandFactory, Parser};
use clap_discovery::Discovery;
use std::process::Command;

#[test]
fn discovery_commands_keep_their_existing_paths_and_alias() {
    Cli::command().debug_assert();
    for (name, expected) in [
        ("tree", Discovery::Tree),
        ("list", Discovery::List),
        ("ls", Discovery::List),
    ] {
        let cli = Cli::try_parse_from(["banker", "self", name]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::CliSelf {
                action: SelfFormat::Discovery(format)
            } if format == expected
        ));
    }
}

#[test]
fn systemd_commands_remain_separate_from_discovery() {
    for action in ["install", "uninstall"] {
        let cli = Cli::try_parse_from(["banker", "self", "systemd", action]).unwrap();
        match cli.command {
            Commands::CliSelf {
                action: SelfFormat::Systemd { action: parsed },
            } => assert!(matches!(
                (action, parsed),
                ("install", SystemdAction::Install) | ("uninstall", SystemdAction::Uninstall)
            )),
            _ => panic!("expected a systemd action"),
        }
    }
}

#[test]
fn discovery_runs_without_configuration_and_uses_the_released_renderer() {
    for (action, format) in [
        ("tree", Discovery::Tree),
        ("list", Discovery::List),
        ("ls", Discovery::List),
    ] {
        let missing_config = std::env::temp_dir().join(format!(
            "banker-discovery-missing-{}-{action}.toml",
            std::process::id()
        ));
        assert!(!missing_config.exists());

        let output = Command::new(env!("CARGO_BIN_EXE_banker"))
            .arg("--config")
            .arg(&missing_config)
            .args(["self", action])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{action} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format.render(&Cli::command())
        );
        assert!(!missing_config.exists());
    }
}
