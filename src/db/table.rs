use super::{Item, Map, Table};
use crate::types::Error;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::types::{AttributeValue, ReturnValue};

impl<I: Item> Table<I> for Client {
    async fn list<const L: usize>(
        &self,
        keys: [(&str, AttributeValue); L],
        index: Option<&str>,
    ) -> Result<Vec<I>, Error> {
        let mut expr = String::new();
        let mut names = std::collections::HashMap::new();
        for (key, _) in &keys {
            let placeholder = format!("#{}", key);
            expr.push_str(&placeholder);
            expr.push_str("=:");
            expr.push_str(*key);
            expr.push(',');
            names.insert(placeholder, key.to_string());
        }
        let expr = expr.trim_end_matches(',');
        let operator = |(key, value): (&str, AttributeValue)| {
            let key = String::from(":") + key;
            (key, value)
        };
        let values = Some(keys.into_iter().map(operator).collect::<Map>());
        let mut query = self.query().table_name(I::TABLE);
        if let Some(index) = index {
            query = query.index_name(index);
        }
        let output = query
            .key_condition_expression(expr)
            .set_expression_attribute_values(values)
            .set_expression_attribute_names(Some(names))
            .send()
            .await
            .map_err(Error::server)?;
        let items = match output.items {
            Some(items) => items,
            None => return Ok(Vec::new()),
        };
        items.into_iter().map(TryFrom::try_from).collect()
    }
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

    async fn update(
        &self,
        key: impl Into<Map>,
        update: impl Into<Map>,
    ) -> Result<Option<I>, Error> {
        let key = Some(key.into());
        let update = Into::<Map>::into(update);
        if update.len() == 0 {
            return Ok(None);
        }
        let mut expression = String::from("SET ");
        let mut names = std::collections::HashMap::new();
        for key in update.keys() {
            let placeholder = format!("#{}", key);
            expression.push_str(&placeholder);
            expression.push_str("=:");
            expression.push_str(key);
            expression.push(',');
            names.insert(placeholder, key.to_string());
        }
        let expression = expression.trim_end_matches(",");
        let update = update
            .into_iter()
            .map(|(mut key, value)| {
                key.insert(0, ':');
                (key, value)
            })
            .collect::<Map>();
        let output = self
            .update_item()
            .table_name(I::TABLE)
            .set_key(key)
            .update_expression(expression)
            .set_expression_attribute_values(Some(update))
            .set_expression_attribute_names(Some(names))
            .return_values(ReturnValue::AllNew)
            .send()
            .await
            .map_err(Error::server)?;
        let item = match output.attributes {
            Some(item) => item.try_into(),
            None => return Ok(None),
        }?;
        Ok(Some(item))
    }

    async fn delete(&self, key: impl Into<Map>) -> Result<(), Error> {
        let input = Some(key.into());
        self.delete_item()
            .table_name(I::TABLE)
            .set_key(input)
            .send()
            .await
            .map_err(Error::server)?;
        Ok(())
    }
}
