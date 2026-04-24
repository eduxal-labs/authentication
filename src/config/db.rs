use aws_config::load_from_env;
pub use aws_sdk_dynamodb::Client;

pub async fn db() -> Client {
    let config = load_from_env().await;
    Client::new(&config)
}
