use axum_aide_macros::api_routes;

use crate::state::AppState;

api_routes! {
    state: AppState,
    GET "/" => health, "Health route";
}

async fn health() -> &'static str {
    "OK"
}
