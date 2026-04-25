use crate::config::Config;
use crate::db::{Item, Table};
use crate::types::{Authorized, Error, Id, Phone, Registered, Session, User, Verification};

mod authentication;
mod session;
mod user;
mod verification;

#[derive(Clone)]
pub struct Authenticator {
    config: std::sync::Arc<Config>,
}

impl Authenticator {
    pub async fn new() -> Self {
        let config = std::sync::Arc::new(Config::new().await);
        Self { config }
    }
}

unsafe impl Send for Authenticator {}
unsafe impl Sync for Authenticator {}

pub trait Get<K, I> {
    async fn get(&self, key: K) -> Result<Option<I>, Error>;
}

pub trait Find<K, I> {
    async fn find(&self, key: K) -> Result<I, Error>;
}

pub trait Put<I> {
    async fn put(&self, input: I) -> Result<(), Error>;
}

impl<I: Item, T: Table<I>> Put<I> for T {
    async fn put(&self, input: I) -> Result<(), Error> {
        <Self as Table<I>>::put(&self, input).await
    }
}

pub trait Update<K, I, O> {
    async fn update(&self, key: K, update: I) -> Result<Option<O>, Error>;
}

pub trait Delete<K, I> {
    async fn delete(&self, key: K) -> Result<(), Error>;
}

pub trait List<K, I> {
    async fn list(&self, key: K) -> Result<Vec<I>, Error>;
}

pub trait Authentication {
    async fn login(&self, phone: Phone) -> Result<Verification, Error>;
    async fn verify(
        &self,
        phone: Phone,
        code: String,
        session: Option<Id>,
        device: String,
    ) -> Result<Authorized, Error>;
    async fn setup(&self, phone: Phone, name: String, device: String) -> Result<Registered, Error>;
    async fn refresh(&self, user: Id, session: Id) -> Result<Registered, Error>;
}

pub trait Sessions {
    async fn list(&self, user: Id) -> Result<Vec<Session>, Error>;
    async fn get(&self, id: Id) -> Result<Option<Session>, Error>;
    async fn delete(&self, user: Id, session: Id) -> Result<(), Error>;
}

pub trait Users {
    async fn user(&self, id: Id) -> Result<User, Error>;
    async fn rename(&self, id: Id, name: String) -> Result<User, Error>;
    async fn change_phone(&self, id: Id, phone: Phone) -> Result<Verification, Error>;
    async fn confirm_change_phone(&self, id: Id, phone: Phone, code: String)
    -> Result<User, Error>;
}
