use rocket::fairing::AdHoc;

mod token;
mod webhook;

pub use token::TokenEncryption;
pub use webhook::WebhookEncryption;

use crate::config::get_app_config;

/// Fairing that sets up an encryption service
pub fn setup_encryption() -> AdHoc {
    AdHoc::on_ignite("Encryption", |rocket| async {
        let app_config = get_app_config(&rocket);
        let token_crypto = TokenEncryption::new(&app_config.secret_key)
            .expect("Invalid secret key: must be 64-character hexadecimal string");
        let webhook_crypto = WebhookEncryption::new(&app_config.secret_key).expect("validated");

        rocket.manage(token_crypto).manage(webhook_crypto)
    })
}
