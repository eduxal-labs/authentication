use crate::types::{Authorized, Error, Id, Registered, Token, User};
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: Id,
    pub user: Id,
    pub refresh: Id,
    pub device: String,
    pub created: DateTime<Utc>,
    pub ttl: DateTime<Utc>,
}

impl Session {
    const TTL: Duration = Duration::days(100);
    pub fn new(user: Id, device: String) -> Self {
        let id = Id::default();
        let refresh = id;
        let created = Utc::now();
        let ttl = created + Self::TTL;
        Self {
            id,
            user,
            refresh,
            device,
            created,
            ttl,
        }
    }
    pub fn authorized(&mut self, user: User) -> Result<Authorized, Error> {
        let session = self.id;
        let access = &Token::access(user.id, session);
        let refresh = &Token::refresh(user.id, session);
        self.refresh = refresh.id;
        Authorized::registered(session, access, refresh, user)
    }

    pub fn registered(&mut self, user: User) -> Result<Registered, Error> {
        let session = self.id;
        let access = &Token::access(user.id, session);
        let refresh = &Token::refresh(user.id, session);
        self.refresh = refresh.id;
        Ok(Registered::new(session, access, refresh, user)?)
    }
}

type Map = HashMap<String, AttributeValue>;

impl From<Session> for Map {
    fn from(session: Session) -> Self {
        let created = session.created.timestamp().to_string();
        let ttl = session.ttl.timestamp().to_string();
        [
            (String::from("id"), session.id.into()),
            (String::from("user"), session.user.into()),
            (String::from("refresh"), session.refresh.into()),
            (String::from("device"), AttributeValue::S(session.device)),
            (String::from("created"), AttributeValue::N(created)),
            (String::from("ttl"), AttributeValue::N(ttl)),
        ]
        .into()
    }
}

impl TryFrom<Option<Session>> for Session {
    type Error = Error;
    fn try_from(value: Option<Session>) -> Result<Self, Self::Error> {
        match value {
            Some(session) => Ok(session),
            None => Err(Error::InvalidSession),
        }
    }
}

impl TryFrom<Map> for Session {
    type Error = Error;
    fn try_from(mut map: Map) -> Result<Self, Self::Error> {
        let id = map
            .remove("id")
            .ok_or_else(|| Error::internal("expected field id for type Session.", map.clone()))?;
        let user = map
            .remove("user")
            .ok_or_else(|| Error::internal("expected field user for type Session.", map.clone()))?;
        let refresh = map.remove("refresh").ok_or_else(|| {
            Error::internal("expected field refresh for type Session.", map.clone())
        })?;
        let device = map.remove("device").ok_or_else(|| {
            Error::internal("expected field device for type Session.", map.clone())
        })?;
        let created = map.remove("created").ok_or_else(|| {
            Error::internal("expected field created for type Session.", map.clone())
        })?;
        let ttl = map
            .remove("ttl")
            .ok_or_else(|| Error::internal("expected field ttl for type Session.", map))?;

        let id = id.try_into().map_err(Error::server)?;
        let user = user.try_into().map_err(Error::server)?;
        let refresh = refresh.try_into().map_err(Error::server)?;

        let device = match device {
            AttributeValue::S(device) => device,
            _ => {
                return Err(Error::internal(
                    "expected the device field of Session to be a string",
                    device,
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
                    "expected the created field of Session to be a number",
                    created,
                ));
            }
        };
        let ttl = match ttl {
            AttributeValue::N(ttl) => ttl.parse::<i64>().map_err(|_| {
                Error::internal("error converting ttl from database string to an int", ())
            })?,
            _ => {
                return Err(Error::internal(
                    "expected the ttl field of Session to be a number",
                    ttl,
                ));
            }
        };
        let created = DateTime::<Utc>::from_timestamp(created, 0)
            .ok_or_else(|| Error::server("error converting seconds to date-time"))?;
        let ttl = DateTime::<Utc>::from_timestamp(ttl, 0)
            .ok_or_else(|| Error::server("error converting seconds to date-time"))?;

        Ok(Self {
            id,
            user,
            refresh,
            device,
            created,
            ttl,
        })
    }
}
