use time::{Duration, UtcDateTime};

use crate::auth::{AuthError, TokenEncryption};

pub struct ClientToken<'r> {
    encryptor: &'r TokenEncryption,
}

struct TokenPayload<'t> {
    key: &'t str,
    expires_at: UtcDateTime,
}

impl<'r> ClientToken<'r> {
    pub fn new(encryptor: &'r TokenEncryption) -> Self {
        Self { encryptor }
    }

    /// Create an encrypted client token that gives access to the given stream key
    /// and is valid for the given length of time
    pub fn create(&self, key: &str, ttl: u32) -> Result<String, AuthError> {
        let payload = TokenPayload {
            key,
            expires_at: UtcDateTime::now() + Duration::seconds(ttl.into()),
        };
        let token_str = payload.to_token_str();

        self.encryptor.encrypt_base64(&token_str)
    }

    /// Verify that the encrypted client token is valid and matches the given stream key
    pub fn validate(&self, token: &str, key: &str) -> Result<(), AuthError> {
        let token_str = self.encryptor.decrypt_base64(token)?;
        let payload = TokenPayload::from_token_str(&token_str)?;
        if payload.key != key {
            return Err(AuthError::PermissionDenied);
        }

        Ok(())
    }
}

impl<'t> TokenPayload<'t> {
    fn to_token_str(&self) -> String {
        format!("{}:{}", self.expires_at.unix_timestamp(), self.key)
    }

    fn from_token_str(token_str: &'t str) -> Result<Self, AuthError> {
        let (unix_expires, key) = token_str.split_once(':').ok_or(AuthError::InvalidToken)?;
        let expires_at = UtcDateTime::from_unix_timestamp(unix_expires.parse().unwrap_or_default())
            .map_err(|_| AuthError::InvalidToken)?;
        if expires_at < UtcDateTime::now() {
            return Err(AuthError::ExpiredToken);
        }

        Ok(Self { key, expires_at })
    }
}
