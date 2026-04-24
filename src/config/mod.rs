mod db;
mod messenger;

pub struct Config {
    db: db::Client,
    messenger: messenger::Messenger,
}

impl Config {
    pub async fn new() -> Self {
        let db = db::db().await;
        let messenger = messenger::Messenger::default();
        Self { db, messenger }
    }

    pub fn db(&self) -> &db::Client {
        &self.db
    }

    pub fn messenger(&self) -> &messenger::Messenger {
        &self.messenger
    }
}
