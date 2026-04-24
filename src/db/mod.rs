use crate::types::Error;
use aws_sdk_dynamodb::types::AttributeValue;
use std::collections::HashMap;

mod table;
mod tables;

type Map = HashMap<String, AttributeValue>;

pub trait Item:
    TryFrom<Map, Error = Error> + TryFrom<Option<Self>, Error = Error> + Into<Map>
{
    const TABLE: &'static str;
}

pub trait Table<I: Item> {
    async fn get(&self, key: impl Into<Map>) -> Result<Option<I>, Error>;
    async fn find(&self, key: impl Into<Map>) -> Result<I, Error> {
        self.get(key).await?.try_into()
    }
    async fn put(&self, item: I) -> Result<(), Error>;
}
