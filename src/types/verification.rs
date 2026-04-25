use crate::types::{Error, Phone};
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::HashMap;

///TTL of a verification code in minutes
const TTL: i64 = 15;

#[derive(Serialize, Clone)]
pub struct Verification {
    pub phone: Phone,
    #[serde(skip)]
    pub code: String,
    pub created: DateTime<Utc>,
    pub ttl: DateTime<Utc>,
}

impl Verification {
    /// Rate Limit Time in seconds until when a user can try to request another Verification code.
    pub const RLTS: Duration = Duration::seconds(60);
    pub fn new(phone: Phone) -> Self {
        let code = rand::random_range::<u32, _>(100000..=999999);
        let code = format!("{:0>6}", code);
        let created = Utc::now();
        let ttl = created + Duration::minutes(TTL);
        Self {
            phone,
            code,
            created,
            ttl,
        }
    }
}

impl From<Verification> for HashMap<String, AttributeValue> {
    fn from(verification: Verification) -> Self {
        let phone = verification.phone.to_string();
        let created = verification.created.timestamp().to_string();
        let ttl = verification.ttl.timestamp().to_string();
        [
            (String::from("phone"), AttributeValue::S(phone)),
            (String::from("code"), AttributeValue::S(verification.code)),
            (String::from("created"), AttributeValue::N(created)),
            (String::from("ttl"), AttributeValue::N(ttl)),
        ]
        .into()
    }
}

impl TryFrom<Option<Verification>> for Verification {
    type Error = Error;
    fn try_from(value: Option<Verification>) -> Result<Self, Self::Error> {
        match value {
            Some(verification) => Ok(verification),
            None => Err(Error::VerificationCodeNotFound),
        }
    }
}

impl TryFrom<HashMap<String, AttributeValue>> for Verification {
    type Error = Error;
    fn try_from(mut map: HashMap<String, AttributeValue>) -> Result<Self, Self::Error> {
        let phone = map.remove("phone").ok_or_else(|| {
            Error::internal("expected field phone for type Verification.", map.clone())
        })?;
        let code = map.remove("code").ok_or_else(|| {
            Error::internal("expected field code for type Verification.", map.clone())
        })?;
        let created = map.remove("created").ok_or_else(|| {
            Error::internal("expected field created for type Verification.", map.clone())
        })?;
        let ttl = map.remove("ttl").ok_or_else(|| {
            Error::internal("expected field ttl for type Verification.", map.clone())
        })?;

        let phone = match phone {
            AttributeValue::S(phone) => phone,
            _ => {
                return Err(Error::internal(
                    "expected the phone field of Verification to be a string",
                    phone,
                ));
            }
        };
        let code = match code {
            AttributeValue::S(code) => code,
            _ => {
                return Err(Error::internal(
                    "expected the code field of Verification to be a string",
                    code,
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
                    "expected the created field of Verification to be a number",
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
                    "expected the ttl field of Verification to be a number",
                    ttl,
                ));
            }
        };
        let created = DateTime::<Utc>::from_timestamp(created, 0)
            .ok_or_else(|| Error::server("error converting seconds to date-time"))?;
        let ttl = DateTime::<Utc>::from_timestamp(ttl, 0)
            .ok_or_else(|| Error::server("error converting seconds to date-time"))?;
        let phone = Phone::new(phone).map_err(Error::server)?;
        Ok(Self {
            phone,
            code,
            created,
            ttl,
        })
    }
}
