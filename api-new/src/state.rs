//! Application state

use std::{ops::Deref, sync::Arc};

use axum_plugin::{AppState, TypeMap};

use crate::{
    auth::{ClientToken, TokenEncryption},
    config::AppConfig,
    redis::{ExclusiveClientPool, ExclusiveClientPoolError, RedisClient, RedisReader},
};

/// App state stored in the Axum router
#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

#[derive(AppState)]
pub struct AppStateInner {
    pub config: Arc<AppConfig>,
    pub encryptor: TokenEncryption,
    pub static_pool: fred::clients::Pool,
    pub exclusive_pool: ExclusiveClientPool,
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<TypeMap> for AppState {
    type Error = anyhow::Error;

    fn try_from(map: TypeMap) -> Result<Self, Self::Error> {
        Ok(Self(Arc::new(AppStateInner::try_from(map)?)))
    }
}

impl AppState {
    pub fn client_tokens(&self) -> ClientToken<'_> {
        ClientToken::new(&self.encryptor)
    }

    pub fn redis_client(&self) -> RedisClient<'_> {
        RedisClient::new(self.static_pool.next(), self.config.max_stream_len)
    }

    pub async fn redis_reader(&self) -> Result<RedisReader, ExclusiveClientPoolError> {
        let client = self.exclusive_pool.get().await?;
        Ok(RedisReader::new(client, self.config.client_timeout))
    }
}
