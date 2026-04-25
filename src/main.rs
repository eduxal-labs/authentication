use lambda_http::run;
use tracing_subscriber::{EnvFilter, fmt, prelude::*, registry};

mod config;
mod db;
mod routes;
mod services;
mod types;

type Result<T, E = Box<dyn std::error::Error + Send + Sync>> = std::result::Result<T, E>;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = fmt::layer().with_filter(filter);
    registry().with(subscriber).init();

    let authenticator = services::Authenticator::new().await;
    let router = routes::router(authenticator);
    run(router).await?;
    Ok(())
}
