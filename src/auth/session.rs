use crate::{
    api::models::{EnableBankingAccountId, EnableBankingSessionId},
    api::{EnableBankingClient, openapi},
    error::AppError,
};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::LazyLock;

pub static EXTENDED_ACCESS_DURATION: LazyLock<TimeDelta> = LazyLock::new(|| TimeDelta::hours(1));
pub const REGULAR_ACCESS_DAYS: i64 = 90;

pub enum SessionAccessLevel {
    Extended,
    Regular,
}

impl From<&Session> for SessionAccessLevel {
    fn from(value: &Session) -> Self {
        if value.age() < *EXTENDED_ACCESS_DURATION {
            Self::Extended
        } else {
            Self::Regular
        }
    }
}

/// The locally persisted result of a completed bank authorization, so the CLI
/// doesn't need to re-run the browser login flow on every invocation.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Session {
    pub session_id: EnableBankingSessionId,
    pub app_id: String,
    pub aspsp_name: String,
    pub aspsp_country: String,
    pub accounts: Vec<EnableBankingAccountId>,
    pub valid_until: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.valid_until
    }

    pub fn age(&self) -> TimeDelta {
        Utc::now() - self.created_at
    }
}

/// A collection of sessions persisted as a JSON array, one per bank.
#[derive(Debug, Serialize, Deserialize)]
pub struct Sessions(pub Vec<Session>);

impl Sessions {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        if !path.exists() {
            return Ok(Sessions(Vec::new()));
        }
        let text = std::fs::read_to_string(path)?;
        serde_json::from_str(&text).map_err(|_| {
            AppError::Other(
                "session file format has changed — delete ~/.config/banker/session.json \
                 and re-authenticate with `banker auth login`"
                    .to_string(),
            )
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_vec_pretty(self)?;
        let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| -> Result<(), AppError> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(&text)?;
            file.sync_all()?;
            std::fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }

    pub fn add(&mut self, session: Session) {
        self.0.push(session);
    }

    pub fn remove(&mut self, session_id: &EnableBankingSessionId) {
        self.0.retain(|s| s.session_id != *session_id);
    }

    /// Remove confirmed inactive sessions for this application, revoking them best-effort.
    /// Keep sessions whose remote status could not be checked.
    pub async fn prune_inactive(&mut self, app_id: &str, client: &EnableBankingClient) -> bool {
        let mut stale = Vec::new();

        for session in self.0.iter().filter(|session| session.app_id == app_id) {
            if session.is_expired() {
                stale.push(session.session_id.clone());
                continue;
            }
            match client.get_session(&session.session_id).await {
                Ok(resp) if resp.status == openapi::types::SessionStatus::Authorized => {}
                Ok(_) => stale.push(session.session_id.clone()),
                Err(error) => {
                    tracing::warn!("could not check session {}: {error}", session.session_id)
                }
            }
        }

        for id in &stale {
            if let Err(e) = client.delete_session(id).await {
                tracing::warn!("failed to revoke stale session {id}: {e}");
            }
        }
        for id in &stale {
            self.remove(id);
        }
        !stale.is_empty()
    }

    #[allow(dead_code)]
    pub fn find_by_bank(&self, name: &str, country: &str) -> Option<&Session> {
        self.0
            .iter()
            .find(|s| s.aspsp_name == name && s.aspsp_country == country)
    }

    pub fn find_by_account_id(&self, account_id: &EnableBankingAccountId) -> Option<&Session> {
        self.0
            .iter()
            .find(|s| s.accounts.iter().any(|a| **a == **account_id))
    }

    pub fn find_by_bank_and_app(
        &self,
        name: &str,
        country: &str,
        app_id: &str,
    ) -> Option<&Session> {
        self.0
            .iter()
            .find(|s| s.aspsp_name == name && s.aspsp_country == country && s.app_id == app_id)
    }
}
