use crate::types::{Error, Id, Phone};
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    id: Id,
    phone: Phone,
    name: String,
    profiled: bool,
    created: DateTime<Utc>,
}

type Map = HashMap<String, AttributeValue>;

impl From<User> for Map {
    fn from(user: User) -> Self {
        let created = user.created.timestamp().to_string();
        let profiled = AttributeValue::Bool(user.profiled);
        [
            (String::from("id"), user.id.into()),
            (String::from("phone"), user.phone.into()),
            (String::from("name"), AttributeValue::S(user.name)),
            (String::from("profiled"), profiled),
            (String::from("created"), AttributeValue::N(created)),
        ]
        .into()
    }
}

impl TryFrom<Option<User>> for User {
    type Error = Error;
    fn try_from(value: Option<User>) -> Result<Self, Self::Error> {
        match value {
            Some(user) => Ok(user),
            None => Err(Error::UserNotFound),
        }
    }
}

impl TryFrom<Map> for User {
    type Error = Error;
    fn try_from(mut map: Map) -> Result<Self, Self::Error> {
        let id = map.remove("id").ok_or(Error::internal(
            "expected field id for type User.",
            map.clone(),
        ))?;
        let phone = map.remove("phone").ok_or(Error::internal(
            "expected field phone for type User.",
            map.clone(),
        ))?;
        let name = map.remove("name").ok_or(Error::internal(
            "expected field name for type User.",
            map.clone(),
        ))?;
        let profiled = map.remove("profiled").ok_or(Error::internal(
            "expected field profiled for type User.",
            map.clone(),
        ))?;
        let created = map.remove("created").ok_or(Error::internal(
            "expected field created for type User.",
            map,
        ))?;

        let id = id.try_into().map_err(Error::server)?;

        let phone = match phone {
            AttributeValue::S(phone) => phone,
            _ => {
                return Err(Error::internal(
                    "expected the phone field of User to be a string",
                    phone,
                ));
            }
        };
        let name = match name {
            AttributeValue::S(name) => name,
            _ => {
                return Err(Error::internal(
                    "expected the name field of User to be a string",
                    name,
                ));
            }
        };
        let profiled = match profiled {
            AttributeValue::Bool(profiled) => profiled,
            _ => {
                return Err(Error::internal(
                    "expected the profiled field of User to be a boolean",
                    profiled,
                ));
            }
        };
        let created = match created {
            AttributeValue::N(created) => created.parse::<i64>().map_err(|_| {
                Error::internal(
                    "error converting created from database string to an int",
                    (),
                )
            })?,
            _ => {
                return Err(Error::internal(
                    "expected the created field of User to be a number",
                    created,
                ));
            }
        };
        let created = DateTime::<Utc>::from_timestamp(created, 0)
            .ok_or(Error::server("error converting seconds to date-time"))?;
        let phone = Phone::new(phone).map_err(Error::server)?;
        Ok(Self {
            id,
            phone,
            name,
            profiled,
            created,
        })
    }
}
