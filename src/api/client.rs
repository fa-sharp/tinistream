use rocket::{get, routes, serde::json::Json, Route};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{verify_client_token, ClientTokenAuth},
    errors::ApiError,
};

#[derive(Serialize, Deserialize)]
pub struct Client {
    pub id: Uuid,
    pub name: String,
    // Add more fields as needed
}

#[derive(Deserialize)]
pub struct CreateClientRequest {
    pub name: String,
    // Add more fields as needed
}

pub fn get_routes() -> Vec<Route> {
    routes![connect_sse]
}

/// Connect to a stream and receive SSE events
#[get("/sse?<key>")]
async fn connect_sse(token: ClientTokenAuth, key: &str) -> Result<Json<Vec<Client>>, ApiError> {
    verify_client_token(&token, key)?;

    // TODO: Implement database query
    let items = vec![Client {
        id: Uuid::new_v4(),
        name: "Example Client".to_string(),
    }];

    Ok(Json(items))
}
