use axum_plugin::AdHocPlugin;
use tower::ServiceBuilder;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

/// # Security plugin
/// - Includes body request limit and security headers.
/// - Adds CORS headers: allowed origins can be specified via the `STREAMER_ALLOWED_ORIGINS`
///   environment variable, otherwise all origins are allowed.
pub fn plugin() -> AdHocPlugin<AppState> {
    AdHocPlugin::named("Security").on_setup(|router, state: &AppState| {
        let security_headers = axum_helmet::Helmet::new()
            .add(axum_helmet::CrossOriginOpenerPolicy::same_origin())
            .add(axum_helmet::CrossOriginResourcePolicy::same_origin())
            .add(axum_helmet::ReferrerPolicy::no_referrer())
            .add(axum_helmet::XContentTypeOptions::nosniff())
            .add(axum_helmet::XFrameOptions::same_origin())
            .into_layer()?;

        let cors = tower_http::cors::CorsLayer::new()
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
            .allow_origin(match &state.config.allowed_origins {
                None => tower_http::cors::AllowOrigin::any(),
                Some(origins) => tower_http::cors::AllowOrigin::list(
                    origins
                        .split(',')
                        .filter_map(|o| axum::http::HeaderValue::from_str(o).ok()),
                ),
            });

        let service = ServiceBuilder::new()
            .layer(RequestBodyLimitLayer::new(state.config.body_limit))
            .layer(cors)
            .layer(security_headers);

        Ok(router.layer(service))
    })
}
