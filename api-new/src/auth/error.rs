use crate::error::AppError;

/// Authentication and authorization errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid secret key, must be 32 byte hex value")]
    InvalidKey,
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
}

impl From<AuthError> for AppError {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::PermissionDenied | AuthError::ExpiredToken | AuthError::InvalidToken => {
                Self::unauthorized("invalid token")
            }
            err => Self::internal(err.into()),
        }
    }
}
