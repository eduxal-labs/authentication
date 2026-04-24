use super::{Item, Map, Table};
use crate::types::Error;
use aws_sdk_dynamodb::Client;

impl<I: Item> Table<I> for Client {
    async fn get(&self, key: impl Into<Map>) -> Result<Option<I>, Error> {
        let key = Some(key.into());
        let output = self
            .get_item()
            .table_name(I::TABLE)
            .set_key(key)
            .send()
            .await
            .map_err(Error::server)?;
        let item = match output.item {
            Some(map) => I::try_from(map)?,
            None => return Ok(None),
        };
        Ok(Some(item))
    }

    async fn put(&self, item: I) -> Result<(), Error> {
        let input = Some(item.into());
        self.put_item()
            .table_name(I::TABLE)
            .set_item(input)
            .send()
            .await
            .map_err(Error::server)?;
        Ok(())
    }
}
