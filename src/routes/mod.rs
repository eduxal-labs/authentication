use aws_sdk_dynamodb::Client;
use axum::{Router, routing::get};

pub fn router(client: Client) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(client)
}

pub async fn health() -> &'static str {
    "OK"
}
