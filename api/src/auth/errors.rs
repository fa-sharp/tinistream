/// Authentication and authorization errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid secret key")]
    InvalidKey,
    #[error("Missing token")]
    MissingToken,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Expired token")]
    ExpiredToken,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Failed to encrypt token")]
    Encrypt,
    #[error("Failed to decrypt token")]
    Decrypt,
    #[error("Failed to decode UTF-8 from decrypted bytes")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Unauthorized")]
    Unauthorized,
}
