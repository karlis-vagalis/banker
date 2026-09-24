use crate::api::EnableBankingClient;
use crate::api::models::{EnableBankingAccountId, EnableBankingSessionId};
use crate::api::openapi;
use crate::auth::session::{Session, Sessions};
use crate::cli::AuthAction;
use crate::config::{Config, session_path};
use crate::error::AppError;
use chrono::{Duration, Utc};
use std::path::Path;
use uuid::Uuid;

pub async fn run(
    config_path: &Path,
    app_name: Option<&str>,
    action: AuthAction,
) -> Result<(), AppError> {
    let config = Config::load(config_path)?;

    match action {
        AuthAction::Login {
            target,
            valid_days,
            no_browser,
        } => {
            let (_bank_name, bank_cfg) = config.resolve_bank(target.bank.as_deref())?;
            login(
                &config,
                app_name,
                bank_cfg.name.clone(),
                bank_cfg.country.clone(),
                valid_days,
                no_browser,
            )
            .await
        }
        AuthAction::Logout { target } => {
            let (_bank_name, bank_cfg) = config.resolve_bank(target.bank.as_deref())?;
            logout(&config, app_name, &bank_cfg.name, &bank_cfg.country).await
        }
        AuthAction::Status => status(&config).await,
    }
}

async fn login(
    config: &Config,
    app_name: Option<&str>,
    bank: String,
    country: String,
    valid_days: i64,
    no_browser: bool,
) -> Result<(), AppError> {
    let (_app_name, app_cfg) = config.resolve_app(app_name)?;
    let client = EnableBankingClient::new(config, app_cfg).await?;

    // Prune stale/expired sessions before starting new auth
    let path = session_path();
    let mut sessions = Sessions::load(&path)?;
    if sessions.prune_inactive(&app_cfg.id, &client).await {
        sessions.save(&path)?;
    }

    let redirect_url = match &config.redirect_url {
        Some(u) => u.to_string(),
        None => {
            let application = client.get_application().await?;

            application.redirect_urls.first().cloned().ok_or_else(|| {
                AppError::Other(
                    "no redirect_url configured and the application has none registered"
                        .to_string(),
                )
            })?
        }
    };

    let valid_until = Utc::now() + Duration::days(valid_days);
    let state = Uuid::new_v4().to_string();

    let auth_request = openapi::types::StartAuthorizationRequest {
        auth_method: None,
        access: openapi::types::Access {
            accounts: None,
            balances: true,
            transactions: true,
            valid_until,
        },
        aspsp: openapi::types::Aspsp {
            country: country.clone(),
            name: bank.clone(),
        },
        state: state.clone(),
        redirect_url,
        psu_id: None,
        psu_type: Some(openapi::types::PsuType::Personal),
        credentials: serde_json::Map::new(),
        credentials_autosubmit: false,
        language: None,
    };

    let auth_response = client.start_auth(&auth_request).await?;

    println!("Open this URL in your browser to authorize access to {bank} ({country}):\n");
    println!("  {}\n", auth_response.url);

    if !no_browser {
        let _ = webbrowser::open(&auth_response.url);
    }

    let code = {
        println!("After completing the login, paste the full redirect URL you were sent to:");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        extract_code_from_url(input.trim(), &state)?
    };

    let session_response = client.create_session(&code).await?;

    // Remove the old local session entry for this bank+app before adding the new one.
    // EnableBanking already revokes the previous session server-side when a new one is created.
    sessions = Sessions::load(&path)?;
    if let Some(old) = sessions.find_by_bank_and_app(&bank, &country, &app_cfg.id) {
        let old_id = old.session_id.clone();
        sessions.remove(&old_id);
    }

    let id = EnableBankingSessionId::try_from(session_response.session_id.clone())?;

    let accounts = session_response
        .accounts
        .iter()
        .map(|account| {
            let uid = account.uid.ok_or_else(|| {
                AppError::Other("authorization returned an account without a UID".into())
            })?;
            EnableBankingAccountId::try_from(uid.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let session = Session {
        session_id: id,
        app_id: app_cfg.id.clone(),
        aspsp_name: bank,
        aspsp_country: country,
        accounts,
        valid_until,
        created_at: Utc::now(),
    };

    sessions.add(session);
    sessions.save(&path)?;

    println!("Login successful. Session stored.");

    Ok(())
}

async fn logout(
    config: &Config,
    app_name: Option<&str>,
    bank: &str,
    country: &str,
) -> Result<(), AppError> {
    let (_name, app_cfg) = config.resolve_app(app_name)?;
    let path = session_path();
    let mut sessions = Sessions::load(&path)?;

    let session = sessions
        .find_by_bank_and_app(bank, country, &app_cfg.id)
        .ok_or_else(|| {
            AppError::Other(format!(
                "no session found for {bank} ({country}) with app {}",
                app_cfg.id
            ))
        })?
        .clone();

    let client = EnableBankingClient::new(config, app_cfg).await?;
    if let Err(e) = client.delete_session(&session.session_id).await {
        tracing::warn!("failed to revoke session remotely: {e}");
    }

    sessions.remove(&session.session_id);
    sessions.save(&path)?;
    println!("Logged out and removed the session for {bank} ({country}).");
    Ok(())
}

async fn status(config: &Config) -> Result<(), AppError> {
    let path = session_path();
    let sessions = Sessions::load(&path)?;

    if sessions.0.is_empty() {
        tracing::warn!("no active sessions — run `banker auth login` first");
        return Ok(());
    }

    for session in &sessions.0 {
        println!(
            "Bank:         {} ({})",
            session.aspsp_name, session.aspsp_country
        );
        println!("App ID:       {}", session.app_id);
        println!("Session ID:   {}", session.session_id);
        println!("Accounts:     {}", session.accounts.len());
        println!("Valid until:  {}", session.valid_until);
        println!("Created:      {}", session.created_at);

        let app = config.find_app_by_id(&session.app_id);
        let client = match app {
            Some((_name, app_cfg)) => EnableBankingClient::new(config, app_cfg).await,
            None => {
                println!(
                    "  (application not found in config for app_id {})",
                    session.app_id
                );
                println!();
                continue;
            }
        };
        match client {
            Ok(c) => match c.get_session(&session.session_id).await {
                Ok(remote) => println!("Remote status: OK ({} account(s))", remote.accounts.len()),
                Err(e) => println!("Remote status check failed: {e}"),
            },
            Err(e) => println!("Client init error: {e}"),
        }
        println!();
    }

    Ok(())
}

fn extract_code_from_url(url_str: &str, expected_state: &str) -> Result<String, AppError> {
    let parsed = url::Url::parse(url_str)
        .map_err(|_| AppError::Other("could not parse the provided URL".to_string()))?;

    let mut code = None;
    let mut state = None;
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.to_string()),
            "state" => state = Some(v.to_string()),
            _ => {}
        }
    }

    match (code, state) {
        (Some(c), Some(s)) if s == expected_state => Ok(c),
        (Some(_), Some(_)) => Err(AppError::Other(
            "state mismatch in callback URL".to_string(),
        )),
        _ => Err(AppError::Other(
            "no code/state found in the given URL".to_string(),
        )),
    }
}
