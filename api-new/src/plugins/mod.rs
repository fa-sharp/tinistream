use axum_plugin::AdHocPlugin;

use crate::{config::AppConfig, state::AppState};

/// Type alias for all ad-hoc plugins
type Plugin = AdHocPlugin<AppState, AppConfig>;

pub mod logging;
pub mod security;
