use axum::{routing::get, Router};
use aws_sdk_dynamodb::Client;

pub fn router(client: Client) -> Router {
    Router::new().with_state(client).route("/health", get(health))
}

pub async fn health() -> &'static str {
    "OK"
}
