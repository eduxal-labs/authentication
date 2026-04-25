use crate::types::{Access, Error, Id, Phone, Refresh, Setup, Token, User};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authorized {
    Registered(Registered),
    Verified(Verified),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Verified {
    token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Registered {
    session: Id,
    access_token: String,
    refresh_token: String,
    user: User,
}

impl Authorized {
    pub fn registered(
        session: Id,
        access: &Token<Access>,
        refresh: &Token<Refresh>,
        user: User,
    ) -> Result<Self, Error> {
        let registered = Registered::new(session, access, refresh, user)?;
        Ok(Self::Registered(registered))
    }

    pub fn verified(phone: Phone) -> Result<Self, Error> {
        let token = &Token::setup(phone);
        Ok(Self::Verified(Verified::new(token)?))
    }
}

impl Registered {
    pub fn new(
        session: Id,
        access: &Token<Access>,
        refresh: &Token<Refresh>,
        user: User,
    ) -> Result<Self, Error> {
        let (access_token, refresh_token) = (access.tokenize()?, refresh.tokenize()?);
        Ok(Self {
            session,
            access_token,
            refresh_token,
            user,
        })
    }
}

impl Verified {
    pub fn new(token: &Token<Setup>) -> Result<Self, Error> {
        let token = token.tokenize()?;
        Ok(Self { token })
    }
}
