use lambda_http::run;

mod config;
mod db;
mod routes;
mod services;
mod types;

type Result<T, E = Box<dyn std::error::Error + Send + Sync>> = std::result::Result<T, E>;

#[tokio::main]
async fn main() -> Result<()> {
    let config = std::sync::Arc::new(config::Config::new().await);
    let router = routes::router(config);
    run(router).await?;
    Ok(())
}
