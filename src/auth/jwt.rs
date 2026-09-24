use crate::error::AppError;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::Serialize;

#[derive(Serialize)]
struct Claims {
    iss: String,
    aud: String,
    iat: i64,
    exp: i64,
}

/// Builds the short-lived RS256 JWT EnableBanking expects as a Bearer token,
/// mirroring the reference Python script: `kid` header = app id, signed with
/// the application's private key, `iss`/`aud` fixed, 1 hour validity.
pub fn build_app_jwt(app_id: &str, private_key_pem: &[u8]) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        iss: "enablebanking.com".to_string(),
        aud: "api.enablebanking.com".to_string(),
        iat: now,
        exp: now + 3600,
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(app_id.to_string());

    let key = EncodingKey::from_rsa_pem(private_key_pem)?;
    let token = encode(&header, &claims, &key)?;
    Ok(token)
}
