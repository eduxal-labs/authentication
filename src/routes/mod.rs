use axum::{Router, routing::get};

pub fn router<T: Send + Sync + Clone + 'static>(config: T) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(config)
}

pub async fn health() -> &'static str {
    "OK"
}
