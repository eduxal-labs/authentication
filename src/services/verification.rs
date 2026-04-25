use crate::db::Table;
use crate::services::{Find, Get};
use crate::types::{Error, Phone, Verification};
use aws_sdk_dynamodb::Client;

impl Find<Phone, Verification> for Client {
    async fn find(&self, phone: Phone) -> Result<Verification, Error> {
        let key = [(String::from("phone"), phone.into())];
        <Self as Table<Verification>>::find(&self, key).await
    }
}

impl Get<Phone, Verification> for Client {
    async fn get(&self, phone: Phone) -> Result<Option<Verification>, Error> {
        let key = [(String::from("phone"), phone.into())];
        <Self as Table<Verification>>::get(&self, key).await
    }
}
