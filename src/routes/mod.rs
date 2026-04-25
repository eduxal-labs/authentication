use crate::services::Authenticator;
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

mod authentication;
mod sessions;
mod user;

pub fn router(authenticator: Authenticator) -> Router {
    let auth = authentication::router(authenticator.clone());
    let user = user::router(authenticator.clone());
    let sessions = sessions::router(authenticator.clone());
    Router::new()
        .nest("/auth", auth)
        .nest("/user", user)
        .nest("/sessions", sessions)
        .route("/health", get(health))
}

pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "Ok"
    }))
}
