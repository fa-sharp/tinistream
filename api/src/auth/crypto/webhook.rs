//! Webhook signatures using SHA-256 HMAC

use std::io::Write;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use time::OffsetDateTime;
use zeroize::Zeroizing;

use crate::auth::AuthError;

type HmacSha256 = Hmac<Sha256>;
const VERSION: &[u8] = b"v1";

/// Service for securing webhooks
pub struct WebhookEncryption {
    key: Zeroizing<Vec<u8>>,
}

impl WebhookEncryption {
    pub fn new(key: &str) -> Result<Self, AuthError> {
        let key_bytes = hex::decode(key).map_err(|_| AuthError::InvalidKey)?;
        Ok(Self {
            key: Zeroizing::new(key_bytes),
        })
    }

    /// Get a response writer wrapper that will generate a signature of
    /// `Version:UnixTime:BODY` -  e.g. `v1:1759xxxxxxx:<raw_body>`. After writing the
    /// response, use `finalize_and_get_headers()` to get the headers.
    pub fn webhook_writer<W: Write>(&self, writer: W) -> WebhookSigWriter<W> {
        WebhookSigWriter::new(&self.key, writer)
    }
}

/// Writer wrapper that generates a SHA-256 HMAC signature
pub struct WebhookSigWriter<W> {
    writer: W,
    mac: HmacSha256,
    time: String,
}

impl<W: Write> WebhookSigWriter<W> {
    fn new(key: &[u8], writer: W) -> Self {
        let mut mac = HmacSha256::new_from_slice(key).expect("should be validated key");

        // update HMAC with prefix (<version>:<timestamp>:)
        let time = OffsetDateTime::now_utc().unix_timestamp().to_string();
        let prefix = [VERSION, b":", time.as_bytes(), b":"].concat();
        mac.update(&prefix);

        Self { writer, mac, time }
    }

    /// Call after writing the body to get the `(signature, timestamp)` headers
    pub fn finalize_and_get_headers(self) -> (String, String) {
        let sig = self.mac.finalize();
        let sig_hex = hex::encode(sig.into_bytes());
        (sig_hex, self.time)
    }
}

impl<W: Write> Write for WebhookSigWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.mac.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
