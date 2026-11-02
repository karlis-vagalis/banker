use clap::CommandFactory;
use clap_discovery::Discovery;

pub fn run(format: Discovery) -> std::io::Result<()> {
    format.print(&crate::cli::Cli::command())
}
