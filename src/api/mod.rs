use crate::api::models::EnableBankingSessionId;
use crate::auth::jwt::build_app_jwt;
use crate::auth::session::{EXTENDED_ACCESS_DURATION, REGULAR_ACCESS_DAYS, Sessions};
use crate::config::{ApplicationConfig, Config, session_path};
use crate::error::AppError;
use crate::models::TimeFrame;
use chrono::{TimeDelta, Utc};
use models::EnableBankingAccountId;
use progenitor_client::Error;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

fn api_err<T>(
    r: Result<T, progenitor_client::Error<openapi::types::ErrorResponse>>,
) -> Result<T, AppError> {
    r.map_err(|e| -> AppError {
        match e {
            Error::CommunicationError(e) => e.into(),
            Error::ErrorResponse(rv) => AppError::Api {
                status: rv.status().as_u16(),
                body: format!("{:?}", rv.into_inner()),
            },
            Error::UnexpectedResponse(r) => AppError::Api {
                status: r.status().as_u16(),
                body: format!("{:?}", r),
            },
            other => AppError::Other(other.to_string()),
        }
    })
}

pub struct EnableBankingClient {
    client: openapi::Client,
}

impl EnableBankingClient {
    pub async fn new(config: &Config, app: &ApplicationConfig) -> Result<Self, AppError> {
        let key = app.key_bytes()?;
        let token = build_app_jwt(&app.id, &key)?;
        let auth_value = HeaderValue::from_str(&format!("Bearer {}", token))?;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth_value);
        let reqwest_client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()?;
        let base_url = config.api_base_url.trim_end_matches('/').to_string();
        let client = openapi::Client::new_with_client(&base_url, reqwest_client);
        Ok(Self { client })
    }

    pub async fn get_application(
        &self,
    ) -> Result<progenitor_client::ResponseValue<openapi::types::GetApplicationResponse>, AppError>
    {
        api_err(self.client.get_application_application_get().await)
    }

    pub async fn list_aspsps(
        &self,
        country: Option<&str>,
    ) -> Result<Vec<openapi::types::AspspData>, AppError> {
        let country_owned = country.and_then(|c| c.parse::<openapi::types::Country>().ok());
        let country_ref = country_owned.as_ref();
        let resp = api_err(
            self.client
                .get_aspsps_aspsps_get(country_ref, None, None, None)
                .await,
        )?;
        Ok(resp.into_inner().aspsps)
    }

    pub async fn start_auth(
        &self,
        req: &openapi::types::StartAuthorizationRequest,
    ) -> Result<
        progenitor_client::ResponseValue<openapi::types::StartAuthorizationResponse>,
        AppError,
    > {
        api_err(self.client.initialize_session_auth_post(req).await)
    }

    pub async fn create_session(
        &self,
        code: &str,
    ) -> Result<progenitor_client::ResponseValue<openapi::types::AuthorizeSessionResponse>, AppError>
    {
        let req = openapi::types::AuthorizeSessionRequest {
            code: code.to_string(),
        };
        api_err(self.client.authorize_session_sessions_post(&req).await)
    }

    pub async fn get_session(
        &self,
        session_id: &EnableBankingSessionId,
    ) -> Result<progenitor_client::ResponseValue<openapi::types::GetSessionResponse>, AppError>
    {
        api_err(
            self.client
                .get_session_sessions_session_id_get(&session_id)
                .await,
        )
    }

    pub async fn delete_session(
        &self,
        session_id: &EnableBankingSessionId,
    ) -> Result<(), AppError> {
        api_err(
            self.client
                .delete_session_sessions_session_id_delete(
                    &session_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await,
        )?;
        Ok(())
    }

    pub async fn get_account_details(
        &self,
        account_id: &EnableBankingAccountId,
    ) -> Result<openapi::types::AccountResource, AppError> {
        let resp = api_err(
            self.client
                .get_account_accounts_account_id_details_get(
                    &account_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await,
        )?;
        Ok(resp.into_inner())
    }

    pub async fn get_account_balances(
        &self,
        account_id: &EnableBankingAccountId,
    ) -> Result<openapi::types::HalBalances, AppError> {
        let resp = api_err(
            self.client
                .get_account_balances_accounts_account_id_balances_get(
                    &account_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await,
        )?;
        Ok(resp.into_inner())
    }

    pub async fn get_account_transactions(
        &self,
        account_id: &EnableBankingAccountId,
        time_frame: Option<&TimeFrame>,
    ) -> Result<Vec<openapi::types::Transaction>, AppError> {
        if let Ok(sessions) = Sessions::load(&session_path()) {
            if let Some(session) = sessions.find_by_account_id(account_id) {
                let access_level: crate::auth::session::SessionAccessLevel = session.into();
                if matches!(
                    access_level,
                    crate::auth::session::SessionAccessLevel::Regular
                ) {
                    let cutoff = Utc::now() - TimeDelta::days(REGULAR_ACCESS_DAYS);
                    let may_include_old_data = time_frame.map_or(true, |tf| tf.from < cutoff);
                    if may_include_old_data {
                        tracing::warn!(
                            "session for {} ({}) is older than {}, so access to transaction data \
                             older than {} days might be limited (see https://enablebanking.com/docs/faq/#how-far-back-transactions-history-can-be-fetched-for-an-account). If requests fail, reauthenticate \
                             the bank with `banker auth login` and try again for full access",
                            session.aspsp_name,
                            session.aspsp_country,
                            humantime::format_duration(EXTENDED_ACCESS_DURATION.to_std().unwrap()),
                            REGULAR_ACCESS_DAYS,
                        );
                    }
                }
            }
        }

        let (date_from, date_to, strategy) = match time_frame {
            Some(tf) => (
                Some(tf.from.date_naive()),
                Some(tf.to.date_naive()),
                Some(openapi::types::TransactionsFetchStrategy::Default),
            ),
            None => (
                None,
                None,
                Some(openapi::types::TransactionsFetchStrategy::Longest),
            ),
        };

        let mut all_transactions: Vec<openapi::types::Transaction> = Vec::new();
        let mut continuation_key: Option<String> = None;

        loop {
            let resp = api_err(
                self.client
                    .get_account_transactions_accounts_account_id_transactions_get(
                        &account_id,
                        continuation_key.as_deref(),
                        date_from.as_ref(),
                        date_to.as_ref(),
                        strategy,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                    .await,
            )?;

            let page = resp.into_inner();
            all_transactions.extend(page.transactions);

            continuation_key = page.continuation_key;
            if continuation_key.is_none() {
                break;
            }
        }

        Ok(all_transactions)
    }
}

pub mod models;
pub mod openapi {
    include!(concat!(env!("OUT_DIR"), "/codegen.rs"));
}
