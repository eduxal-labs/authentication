use crate::types::error::Error;
use aws_sdk_dynamodb::types::AttributeValue;
use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Hash, Eq)]
pub struct Id([u8; 12]);

impl Id {
    pub fn system() -> Self {
        Self([0u8; 12])
    }

    pub fn bytes(self) -> [u8; 12] {
        self.0
    }
}

impl Default for Id {
    fn default() -> Self {
        Id(ObjectId::new().bytes())
    }
}

impl FromStr for Id {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = ObjectId::parse_str(s)
            .map_err(|_| Error::InvalidId)?
            .bytes();
        Ok(Self(bytes))
    }
}

impl Debug for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ObjectId::from_bytes(self.0).to_hex())
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ObjectId::from_bytes(self.0).to_hex())
    }
}

impl From<Id> for String {
    fn from(value: Id) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for Id {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<[u8; 12]> for Id {
    fn from(bytes: [u8; 12]) -> Self {
        Id(bytes)
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&ObjectId::from_bytes(self.0).to_hex())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex = <&str>::deserialize(deserializer)?;
        let bytes = ObjectId::parse_str(hex)
            .map_err(serde::de::Error::custom)?
            .bytes();
        Ok(Self(bytes))
    }
}

impl TryFrom<AttributeValue> for Id {
    type Error = Error;
    fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
        if let AttributeValue::S(value) = value {
            return value.try_into().map_err(Error::server);
        }
        Err(Error::server("invalid AttributeValue type for type Id"))
    }
}

impl From<Id> for AttributeValue {
    fn from(id: Id) -> Self {
        AttributeValue::S(id.into())
    }
}
