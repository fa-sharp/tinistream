use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Nonce,
};
use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use rocket::fairing::AdHoc;

use crate::{auth::AuthError, config::get_app_config};

const VERSION: &[u8] = b"v1";
const VERSION_LEN: usize = VERSION.len();
const NONCE_LEN: usize = 12;

/// Service for encrypting and decrypting tokens using the secret key
#[derive(Clone)]
pub struct Crypto {
    cipher: Aes256Gcm,
}

impl Crypto {
    pub fn new(key: &str) -> Result<Self, AuthError> {
        let key_bytes = hex::decode(key).map_err(|_| AuthError::InvalidKey)?;
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|_| AuthError::InvalidKey)?;
        Ok(Self { cipher })
    }

    /// Encrypts a string using AES-256-GCM and returns a base64-encoded token with the version, nonce, and ciphertext.
    pub fn encrypt_base64(&self, plaintext: &str) -> Result<String, AuthError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| AuthError::Encrypt)?;

        let token_bytes = [VERSION, &nonce, &ciphertext].concat();
        Ok(BASE64_URL_SAFE_NO_PAD.encode(&token_bytes))
    }

    /// Decrypts a base64-encoded token into the plaintext message
    pub fn decrypt_base64(&self, token: &str) -> Result<String, AuthError> {
        let bytes = BASE64_URL_SAFE_NO_PAD
            .decode(token)
            .map_err(|_| AuthError::InvalidToken)?;

        let (version, rest) = bytes
            .split_at_checked(VERSION_LEN)
            .ok_or(AuthError::InvalidToken)?;
        if version != VERSION {
            return Err(AuthError::InvalidToken);
        }

        let (nonce, ciphertext) = rest
            .split_at_checked(NONCE_LEN)
            .ok_or(AuthError::InvalidToken)?;
        let plaintext = self
            .cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| AuthError::Decrypt)?;

        Ok(String::from_utf8(plaintext)?)
    }
}

/// Fairing that sets up an encryption service
pub fn setup_encryption() -> AdHoc {
    AdHoc::on_ignite("Encryption", |rocket| async {
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
