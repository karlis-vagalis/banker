use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn default_api_base_url() -> String {
    "https://api.enablebanking.com".to_string()
}

/// User-editable configuration, loaded from a TOML file.
///
/// Example `config.toml`:
///
/// ```toml
/// db_path = "/home/you/.local/share/banker/data.db"
///
/// [application.comdirect]
/// id = "YOUR_ENABLEBANKING_APP_ID"
/// key_file = "/home/you/.config/banker/comdirect.pem"
///
/// [bank.comdirect]
/// name = "Comdirect"
/// country = "DE"
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// Where to store the SQLite database. Defaults to a per-user data directory.
    #[serde(default)]
    pub db_path: Option<PathBuf>,
    /// Per-application credentials, keyed by a friendly name.
    pub application: HashMap<String, ApplicationConfig>,
    /// Named bank entries mapping short keys to ASPSP name and country.
    #[serde(default)]
    pub bank: HashMap<String, BankConfig>,
    /// EnableBanking API base URL. Defaults to the production API.
    #[serde(default = "default_api_base_url")]
    pub api_base_url: String,
    /// Redirect URL to use when starting an authorization. If omitted, the first
    /// redirect URL registered on your application (via `GET /application`) is used.
    #[serde(default)]
    pub redirect_url: Option<String>,
}

/// A named bank entry that maps a short key to the actual ASPSP name and country.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BankConfig {
    /// The actual ASPSP name, e.g. "Comdirect"
    pub name: String,
    /// ISO country code, e.g. "DE"
    pub country: String,
}

/// Credentials for a single EnableBanking application.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationConfig {
    /// Your EnableBanking application id (also used as the JWT `kid`).
    pub id: String,
    /// Path to the RSA private key (.pem) matching this application.
    pub key_file: PathBuf,
}

impl ApplicationConfig {
    pub fn key_bytes(&self) -> Result<Vec<u8>, AppError> {
        Ok(std::fs::read(&self.key_file)?)
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, AppError> {
        if !path.exists() {
            return Err(AppError::ConfigNotFound(path.to_path_buf()));
        }
        let text = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&text)?;
        if cfg.application.is_empty() {
            return Err(AppError::Other(
                "no applications configured – add at least one [application.<name>] section to your config"
                    .to_string(),
            ));
        }
        Ok(cfg)
    }

    /// Resolve which application to use.
    ///
    /// - If `name` is `Some`, returns that entry.
    /// - If `name` is `None` and there is exactly one application, returns it.
    /// - If `name` is `None` and there are multiple applications, returns an error
    ///   asking the caller to pass `--app`.
    pub fn resolve_app(&self, name: Option<&str>) -> Result<(&str, &ApplicationConfig), AppError> {
        match name {
            Some(n) => self
                .application
                .get_key_value(n)
                .map(|(k, v)| (k.as_str(), v))
                .ok_or_else(|| {
                    AppError::Other(format!(
                        "application \"{n}\" not found in config – available: {}",
                        self.application
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                }),
            None => {
                if self.application.len() == 1 {
                    Ok(self
                        .application
                        .iter()
                        .next()
                        .map(|(k, v)| (k.as_str(), v))
                        .unwrap())
                } else {
                    Err(AppError::Other(
                        "multiple applications configured – use --app <name> to select one"
                            .to_string(),
                    ))
                }
            }
        }
    }

    /// Look up an application by its `id` field (the UUID), returning the name and config.
    pub fn find_app_by_id(&self, id: &str) -> Option<(&str, &ApplicationConfig)> {
        self.application
            .iter()
            .find(|(_, v)| v.id == id)
            .map(|(k, v)| (k.as_str(), v))
    }

    /// Resolve which bank config to use.
    ///
    /// - If `name` is `Some`, returns that entry.
    /// - If `name` is `None` and there is exactly one bank, returns it.
    /// - If `name` is `None` and there are zero banks, returns an error.
    /// - If `name` is `None` and there are multiple banks, returns an error
    ///   asking the caller to pass `--bank`.
    pub fn resolve_bank(&self, name: Option<&str>) -> Result<(&str, &BankConfig), AppError> {
        match name {
            Some(n) => self
                .bank
                .get_key_value(n)
                .map(|(k, v)| (k.as_str(), v))
                .ok_or_else(|| {
                    AppError::Other(format!(
                        "bank \"{n}\" not found in config – available: {}",
                        self.bank.keys().cloned().collect::<Vec<_>>().join(", ")
                    ))
                }),
            None => {
                if self.bank.len() == 1 {
                    Ok(self
                        .bank
                        .iter()
                        .next()
                        .map(|(k, v)| (k.as_str(), v))
                        .unwrap())
                } else if self.bank.is_empty() {
                    Err(AppError::Other(
                        "no banks configured – add at least one [bank.<name>] section to your config"
                            .to_string(),
                    ))
                } else {
                    Err(AppError::Other(
                        "multiple banks configured – use --bank <name> to select one".to_string(),
                    ))
                }
            }
        }
    }

    pub fn db_path(&self) -> PathBuf {
        self.db_path
            .clone()
            .unwrap_or_else(|| default_data_dir().join("data.db"))
    }
}

pub fn default_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("banker")
}

pub fn default_config_path() -> PathBuf {
    default_config_dir().join("config.toml")
}

pub fn default_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("banker")
}

/// Where the (single) active session is persisted between CLI invocations.
pub fn session_path() -> PathBuf {
    default_config_dir().join("session.json")
}
