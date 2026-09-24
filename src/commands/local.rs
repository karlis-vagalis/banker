use crate::cli::{MetadataAction, SchemaAction};
use crate::config::Config;
use crate::db::{self};
use crate::error::AppError;
use crate::models::{Resource, ResourceType};
use std::error::Error as StdError;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;

/// Given an optional `--bank <KEY>`, resolve it to the `(name, country)` pair
/// for use as SQL filter parameters. Returns `None` when no key was given.
pub fn resolve_filter_bank(
    config_path: &Path,
    bank_arg: Option<&str>,
) -> Result<Option<(String, String)>, AppError> {
    match bank_arg {
        Some(key) => {
            let config = Config::load(config_path)?;
            let (_name, bank_cfg) = config.resolve_bank(Some(key))?;
            Ok(Some((bank_cfg.name.clone(), bank_cfg.country.clone())))
        }
        None => Ok(None),
    }
}

/// Dispatch metadata subcommands (schema get/set/delete, metadata get/set/delete)
/// for a given target type / table.
pub async fn metadata_dispatch<T>(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    resource_type: &ResourceType,
    action: MetadataAction<T>,
) -> Result<(), AppError>
where
    T: Into<Resource> + FromStr + Clone + Send + Sync + 'static,
    T::Err: Into<Box<dyn StdError + Send + Sync>>,
{
    match action {
        MetadataAction::Schema { action } => match action {
            SchemaAction::Set { schema, yes } => {
                let raw = resolve_json(schema)?;
                let value: serde_json::Value = serde_json::from_str(&raw)?;

                validate_json_schema(&value)?;

                let existing = db::get_metadata_schema(pool, resource_type).await?;
                if existing.is_some() && !yes {
                    confirm_overwrite(
                        "Warning: a schema is already set. All future metadata must\n\
                         conform to the new schema. Existing metadata may become\n\
                         invalid.\n\nOverwrite? [y/N] ",
                    )?;
                }

                db::upsert_metadata_schema(pool, resource_type, &value).await?;
                println!("Metadata schema saved successfully.");
            }
            SchemaAction::Get => match db::get_metadata_schema(pool, resource_type).await? {
                Some(s) => println!("{}", serde_json::to_string_pretty(&s)?),
                None => println!("No metadata schema set."),
            },
            SchemaAction::Delete { yes } => {
                if !yes {
                    confirm_overwrite(
                        "Warning: deleting the schema will remove validation for future metadata.\n\
                         Continue? [y/N] ",
                    )?;
                }
                db::delete_metadata_schema(pool, resource_type).await?;
                println!("Metadata schema deleted.");
            }
        },
        MetadataAction::Set { id, metadata, yes } => {
            let resource: Resource = id.into();
            let existing = db::get_metadata(pool, &resource).await?;
            if existing.is_some() && !yes {
                confirm_overwrite(
                    "Warning: metadata already exists for this record. \
                     Overwrite? [y/N] ",
                )?;
            }

            let raw = resolve_json(metadata)?;
            let value: serde_json::Value = serde_json::from_str(&raw)?;

            if let Some(schema) = db::get_metadata_schema(pool, resource_type).await? {
                validate_instance(&value, &schema)?;
            }

            db::upsert_metadata(pool, &resource, &value).await?;
            println!("Metadata saved.");
        }
        MetadataAction::Get { id } => {
            let resource: Resource = id.into();
            match db::get_metadata(pool, &resource).await? {
                Some(m) => println!("{}", serde_json::to_string_pretty(&m)?),
                None => println!("No metadata found."),
            }
        }
        MetadataAction::Delete { id, yes } => {
            let resource: Resource = id.into();
            if !yes {
                let existing = db::get_metadata(pool, &resource).await?;
                if existing.is_some() {
                    confirm_overwrite(
                        "Warning: this will permanently delete the metadata. \
                         Continue? [y/N] ",
                    )?;
                }
            }
            db::delete_metadata(pool, &resource).await?;
            println!("Metadata deleted.");
        }
    }
    Ok(())
}

/// Read JSON from a CLI argument: file path, `"-"` for stdin, raw JSON string, or stdin if omitted.
pub fn resolve_json(arg: Option<String>) -> Result<String, AppError> {
    match arg {
        Some(ref s) if s == "-" => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        Some(ref s) if Path::new(s).exists() => Ok(std::fs::read_to_string(s)?),
        Some(s) => Ok(s),
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

fn confirm_overwrite(message: &str) -> Result<(), AppError> {
    let tty = std::fs::File::open("/dev/tty")
        .map_err(|_| AppError::Other("Use --yes to confirm non-interactively.".to_string()))?;
    use std::io::{BufRead, Write};
    let mut stdout = std::io::stdout();
    write!(stdout, "{message}")?;
    stdout.flush()?;
    let mut input = String::new();
    std::io::BufReader::new(tty).read_line(&mut input)?;
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => Err(AppError::Other("Operation cancelled.".to_string())),
    }
}

fn validate_json_schema(value: &serde_json::Value) -> Result<(), AppError> {
    jsonschema::validator_for(value)
        .map_err(|e| AppError::Other(format!("Invalid JSON Schema definition: {e}")))?;
    Ok(())
}

fn validate_instance(
    instance: &serde_json::Value,
    schema: &serde_json::Value,
) -> Result<(), AppError> {
    let validator = jsonschema::validator_for(schema)
        .map_err(|e| AppError::Other(format!("Invalid stored schema: {e}")))?;
    validator
        .validate(instance)
        .map_err(|e| AppError::Other(format!("Metadata does not conform to the schema:\n{e}")))?;
    Ok(())
}
