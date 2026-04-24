use crate::types::Error;
use aws_sdk_dynamodb::types::AttributeValue;
use phone_number_verifier::verify_phone_number_with_country_code;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[derive(Clone, PartialEq, Eq)]
pub struct Phone(String);

impl Phone {
    pub fn new(phone: String) -> Result<Self, Error> {
        if !verify_phone_number_with_country_code(&phone) {
            return Err(Error::InvalidPhoneNumber);
        }
        Ok(Self(phone))
    }
}

impl Serialize for Phone {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Phone {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let phone = String::deserialize(deserializer)?;

        let phone = Phone::new(phone).map_err(de::Error::custom)?;
        Ok(phone)
    }
}

impl AsRef<str> for Phone {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Phone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for Phone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<AttributeValue> for Phone {
    type Error = Error;
    fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
        if let AttributeValue::S(value) = value {
            return Self::new(value).map_err(Error::server);
        }
        Err(Error::server("invalid AttributeValue type for type Phone"))
    }
}

impl From<Phone> for AttributeValue {
    fn from(value: Phone) -> Self {
        AttributeValue::S(value.0)
    }
}
