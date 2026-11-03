use crate::auth::UserInfo;
use crate::error::HubError;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const COOKIE_NAME: &str = "agy_session";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub uid: u32,
    pub gid: u32,
    pub groups: Vec<String>,
    pub role: String,
    pub iat: usize,
    pub exp: usize,
    pub nbf: usize,
    pub iss: String,
}

#[derive(Clone)]
pub struct SessionManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    ttl_secs: u64,
    secure_cookie: bool,
}

impl SessionManager {
    pub fn new(secret: &str, ttl_secs: u64, secure_cookie: bool) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            ttl_secs,
            secure_cookie,
        }
    }

    pub fn create_token(&self, user: &UserInfo) -> Result<String, HubError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| HubError::SessionError(e.to_string()))?
            .as_secs() as usize;

        let exp = now + (self.ttl_secs as usize);

        let claims = Claims {
            sub: user.username.clone(),
            uid: user.uid,
            gid: user.gid,
            groups: user.groups.clone(),
            role: user.role.clone(),
            iat: now,
            exp,
            nbf: now,
            iss: "antigravity-hub".to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| HubError::SessionError(format!("JWT encoding error: {}", e)))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, HubError> {
        let mut validation = Validation::default();
        validation.set_issuer(&["antigravity-hub"]);
        validation.validate_exp = true;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| HubError::SessionError(format!("Invalid token: {}", e)))?;

        Ok(token_data.claims)
    }

    pub fn build_cookie_header(&self, token: &str) -> String {
        let secure_flag = if self.secure_cookie { "; Secure" } else { "" };
        format!(
            "{}={}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax{}",
            COOKIE_NAME, token, self.ttl_secs, secure_flag
        )
    }

    pub fn build_logout_cookie_header(&self) -> String {
        let secure_flag = if self.secure_cookie { "; Secure" } else { "" };
        format!(
            "{}=deleted; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{}",
            COOKIE_NAME, secure_flag
        )
    }

    pub fn extract_token_from_cookie(&self, cookie_header: &str) -> Option<String> {
        for cookie in cookie_header.split(';') {
            let cookie = cookie.trim();
            if let Some(rest) = cookie.strip_prefix(COOKIE_NAME) {
                if let Some(token) = rest.strip_prefix('=') {
                    return Some(token.to_string());
                }
            }
        }
        None
    }
}
