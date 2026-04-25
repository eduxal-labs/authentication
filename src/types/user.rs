use crate::types::{Error, Id, Phone};
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Id,
    pub phone: Phone,
    pub name: String,
    pub level: Level,
    pub status: Status,
    pub profiled: bool,
    pub created: DateTime<Utc>,
}

#[derive(Default, Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Level {
    #[default]
    Normal,
    System,
    Super,
}

#[derive(Default, Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Status {
    #[default]
    Active,
    Invited,
    Blocked,
    Deleted,
}

impl User {
    pub fn new(phone: Phone, name: String) -> Self {
        let id = Id::default();
        let level = Level::Normal;
        let status = Status::Active;
        let profiled = false;
        let created = Utc::now();
        Self {
            id,
            phone,
            name,
            level,
            status,
            profiled,
            created,
        }
    }
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
            (String::from("level"), user.level.into()),
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
        let level = map.remove("level").ok_or(Error::internal(
            "expected field level for type User.",
            map.clone(),
        ))?;
        let status = map.remove("status").ok_or(Error::internal(
            "expected field status for type User.",
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
        let level = level.try_into()?;
        let status = status.try_into()?;
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
            level,
            status,
            profiled,
            created,
        })
    }
}

impl From<Level> for AttributeValue {
    fn from(level: Level) -> Self {
        match level {
            Level::Normal => AttributeValue::S(String::from("Normal")),
            Level::System => AttributeValue::S(String::from("System")),
            Level::Super => AttributeValue::S(String::from("Super")),
        }
    }
}

impl TryFrom<AttributeValue> for Level {
    type Error = Error;
    fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
        if let AttributeValue::S(value) = value {
            let value = value.to_lowercase();
            return match value.as_str() {
                "normal" => Ok(Self::Normal),
                "system" => Ok(Self::System),
                "super" => Ok(Self::Super),
                _ => Err(Error::server("invalid value for User Level")),
            };
        }
        Err(Error::server("invalid AttributeValue type for User Level"))
    }
}

impl From<Status> for AttributeValue {
    fn from(status: Status) -> Self {
        match status {
            Status::Active => AttributeValue::S(String::from("Active")),
            Status::Invited => AttributeValue::S(String::from("Invited")),
            Status::Blocked => AttributeValue::S(String::from("Blocked")),
            Status::Deleted => AttributeValue::S(String::from("Deleted")),
        }
    }
}

impl TryFrom<AttributeValue> for Status {
    type Error = Error;
    fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
        if let AttributeValue::S(value) = value {
            let value = value.to_lowercase();
            return match value.as_str() {
                "active" => Ok(Self::Active),
                "invited" => Ok(Self::Invited),
                "blocked" => Ok(Self::Blocked),
                "deleted" => Ok(Self::Deleted),
                _ => Err(Error::server("invalid value for User Status")),
            };
        }
        Err(Error::server("invalid AttributeValue type for User Status"))
    }
}
