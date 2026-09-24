use crate::{
    api::models::{EnableBankingAccountId, EnableBankingSessionId},
    api::{EnableBankingClient, openapi},
    error::AppError,
};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
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
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

        Ok(())
    }

    pub fn add(&mut self, session: Session) {
        self.0.push(session);
    }

    pub fn remove(&mut self, session_id: &EnableBankingSessionId) {
        self.0.retain(|s| s.session_id != *session_id);
    }

    /// Remove sessions that are locally expired or not `Authorized` on the remote,
    /// optionally revoking them first.
    pub async fn prune_inactive(&mut self, client: &EnableBankingClient) -> Result<(), AppError> {
        let mut stale = Vec::new();

        for session in &self.0 {
            if session.is_expired() {
                stale.push(session.session_id.clone());
                continue;
            }
            match client.get_session(&session.session_id).await {
                Ok(resp) if resp.status == openapi::types::SessionStatus::Authorized => {}
                _ => stale.push(session.session_id.clone()),
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
        Ok(())
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
