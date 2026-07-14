use crate::{auth::TokenEncryption, plugins::Plugin};

pub fn plugin() -> Plugin {
    Plugin::named("Crypto").on_init(async |mut app| {
        let token_encryptor = TokenEncryption::new(&app.config().secret_key)?;
        app.insert(token_encryptor)?;

        Ok(app)
    })
}
