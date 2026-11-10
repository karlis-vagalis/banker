//! Shared database seeding plus integration-test-only command helpers.
mod seed;
pub use seed::{BankSeed, SeedSpec, SeededDatabase, TestBank, seeded_database};

use std::process::Command;

pub trait FixtureCommand {
    fn command(&self) -> Command;
}

impl FixtureCommand for SeededDatabase {
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_banker"));
        command.arg("--config").arg(self.config_path());
        command
    }
}
