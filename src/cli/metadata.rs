use clap::Subcommand;
use std::error::Error as StdError;
use std::str::FromStr;

#[derive(Subcommand)]
pub enum MetadataAction<T>
where
    T: FromStr + Clone + Send + Sync + 'static,
    <T as FromStr>::Err: Into<Box<dyn StdError + Send + Sync>>,
{
    /// Manage the global JSON Schema for metadata
    Schema {
        #[command(subcommand)]
        action: SchemaAction,
    },
    /// Attach metadata
    Set {
        /// Internal ID from the database
        id: T,
        /// Raw JSON metadata string, a file path, or "-" for stdin. Omit to pipe from stdin.
        metadata: Option<String>,
        /// Skip the confirmation prompt when overwriting existing metadata
        #[arg(short, long)]
        yes: bool,
    },
    /// Retrieve metadata
    Get {
        /// Internal ID from the database
        id: T,
    },
    /// Delete metadata
    Delete {
        /// Internal ID from the database
        id: T,
        /// Skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum SchemaAction {
    /// Save a JSON Schema to the database
    Set {
        /// JSON Schema string, a file path, or "-" for stdin. Omit to pipe from stdin.
        schema: Option<String>,
        /// Skip the confirmation prompt when overwriting an existing schema
        #[arg(short, long)]
        yes: bool,
    },
    /// Print the active JSON Schema
    Get,
    /// Delete the stored JSON Schema
    Delete {
        /// Skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}
