use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Nonce,
};
use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use rocket::fairing::AdHoc;

use crate::config::get_app_config;

/// Token version
const VERSION: &[u8] = b"v1";
const VERSION_LEN: usize = VERSION.len();
const NONCE_LEN: usize = 12;

/// Error that can occur while encrypting/decrypting tokens
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid secret key")]
    InvalidKey,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Missing token")]
    MissingToken,
    #[error("Failed to encrypt token")]
    Encrypt,
    #[error("Failed to decrypt token")]
    Decrypt,
    #[error("Failed to decode UTF-8 from decrypted bytes")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Service for encrypting and decrypting tokens using the secret key
#[derive(Clone)]
pub struct Crypto {
    cipher: Aes256Gcm,
}

impl Crypto {
    pub fn new(key: &str) -> Result<Self, CryptoError> {
        let key_bytes = hex::decode(key).map_err(|_| CryptoError::InvalidKey)?;
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|_| CryptoError::InvalidKey)?;
        Ok(Self { cipher })
    }

    /// Encrypts a string using AES-256-GCM and returns a base64-encoded token with the version, nonce, and ciphertext.
    pub fn encrypt_base64(&self, plaintext: &str) -> Result<String, CryptoError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::Encrypt)?;

        let capacity = VERSION_LEN + NONCE_LEN + ciphertext.len();
        let mut token_bytes: Vec<u8> = Vec::with_capacity(capacity);
        token_bytes.extend_from_slice(VERSION);
        token_bytes.extend_from_slice(&nonce);
        token_bytes.extend_from_slice(&ciphertext);

        Ok(BASE64_URL_SAFE_NO_PAD.encode(&token_bytes))
    }

    /// Decrypts a base64-encoded token into the plaintext message
    pub fn decrypt_base64(&self, token: &str) -> Result<String, CryptoError> {
        let bytes = BASE64_URL_SAFE_NO_PAD
            .decode(token)
            .map_err(|_| CryptoError::InvalidToken)?;

        let (version, rest) = bytes
            .split_at_checked(VERSION_LEN)
            .ok_or(CryptoError::InvalidToken)?;
        if version != VERSION {
            return Err(CryptoError::InvalidToken);
        }

        let (nonce, ciphertext) = rest
            .split_at_checked(NONCE_LEN)
            .ok_or(CryptoError::InvalidToken)?;
        let plaintext = self
            .cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| CryptoError::Decrypt)?;

        Ok(String::from_utf8(plaintext)?)
    }
}

/// Fairing that sets up an encryption service
pub fn setup_encryption() -> AdHoc {
    AdHoc::on_ignite("Encryption setup", |rocket| async {
        let app_config = get_app_config(&rocket);
        let crypto = Crypto::new(&app_config.secret_key)
            .expect("Invalid secret key: must be 64-character hexadecimal string");

        rocket.manage(crypto)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_crypto() -> Crypto {
        // 64-character hex key for AES-256 (32 bytes)
        let test_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        Crypto::new(test_key).unwrap()
    }

    #[test]
    fn create_and_decode_token() {
        let crypto = get_test_crypto();
        let plaintext = "hello world";

        let token = crypto.encrypt_base64(plaintext).unwrap();
        let decoded = crypto.decrypt_base64(&token).unwrap();

        assert_eq!(plaintext, decoded);
    }

    #[test]
    fn empty_string() {
        let crypto = get_test_crypto();
        let plaintext = "";

        let token = crypto.encrypt_base64(plaintext).unwrap();
        let decoded = crypto.decrypt_base64(&token).unwrap();

        assert_eq!(plaintext, decoded);
    }

    #[test]
    fn unicode_text() {
        let crypto = get_test_crypto();
        let plaintext = "🦀 Rust is awesome! 中文 العربية";

        let token = crypto.encrypt_base64(plaintext).unwrap();
        let decoded = crypto.decrypt_base64(&token).unwrap();

        assert_eq!(plaintext, decoded);
    }

    #[test]
    fn invalid_key() {
        assert!(Crypto::new("invalid").is_err());
        assert!(Crypto::new("").is_err());
        assert!(Crypto::new("short").is_err());
    }

    #[test]
    fn invalid_token() {
        let crypto = get_test_crypto();

        // Invalid base64
        assert!(crypto.decrypt_base64("not_base64!").is_err());

        // Too short token
        assert!(crypto.decrypt_base64("dGVzdA==").is_err());

        // Wrong version
        let valid_token = crypto.encrypt_base64("test").unwrap();
        let mut bytes = BASE64_URL_SAFE_NO_PAD.decode(&valid_token).unwrap();
        bytes[0] = b'v';
        bytes[1] = b'2';
        let invalid_version_token = BASE64_URL_SAFE_NO_PAD.encode(&bytes);
        assert!(crypto.decrypt_base64(&invalid_version_token).is_err());
    }

    #[test]
    fn different_keys_cant_decode() {
        let crypto1 = get_test_crypto();
        let key2 = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
        let crypto2 = Crypto::new(key2).unwrap();

        let token = crypto1.encrypt_base64("secret message").unwrap();
        assert!(crypto2.decrypt_base64(&token).is_err());
    }
}
