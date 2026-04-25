use crate::services::{Authentication, Authenticator, Find, Get, Put, Sessions, Users};
use crate::types::{Authorized, Error, Id, Phone, Registered, Session, Status, User, Verification};
use aws_sdk_dynamodb::Client;
use chrono::Utc;

impl Authentication for Authenticator {
    async fn login(&self, phone: Phone) -> Result<Verification, Error> {
        let db = self.config.db();
        let existing = <Client as Get<Phone, Verification>>::get(db, phone.clone()).await?;
        if let Some(verification) = existing {
            if Utc::now().timestamp() > (verification.created + Verification::RLTS).timestamp() {
                return Err(Error::SlowDown);
            }
        }
        let verification = Verification::new(phone.clone());
        let messenger = self.config.messenger();
        let (receipient, code) = (phone.as_ref(), verification.code.as_str());
        messenger.send(receipient, code).await?;
        <Client as Put<Verification>>::put(db, verification.clone()).await?;
        Ok(verification)
    }

    async fn verify(
        &self,
        phone: Phone,
        code: String,
        session_id: Option<Id>,
        device: String,
    ) -> Result<Authorized, Error> {
        let db = self.config.db();
        if let Some(user) = <Client as Get<Phone, User>>::get(db, phone.clone()).await? {
            if user.status == Status::Blocked {
                return Err(Error::Forbidden);
            }
        }
        let verification = <Client as Find<Phone, Verification>>::find(db, phone.clone()).await?;
        if verification.code != code {
            return Err(Error::InvalidVerificationCode);
        }
        let user = <Client as Get<Phone, User>>::get(db, phone.clone()).await?;
        let user = match user {
            Some(user) => user,
            None => return Authorized::verified(phone),
        };
        if user.status == Status::Blocked {
            return Err(Error::Forbidden);
        }
        let mut session = Session::new(user.id, device);
        if let Some(id) = session_id {
            if let Some(existing) = <Self as Sessions>::get(&self, id).await? {
                if existing.user == user.id {
                    session.id = id;
                }
            }
        }
        let authorized = session.authorized(user)?;
        <Client as Put<Session>>::put(db, session).await?;
        Ok(authorized)
    }

    async fn setup(&self, phone: Phone, name: String, device: String) -> Result<Registered, Error> {
        let db = self.config.db();
        let user = <Client as Get<Phone, User>>::get(db, phone.clone()).await?;
        if let Some(user) = user {
            let user = <Self as Users>::rename(&self, user.id, name).await?;
            if user.status == Status::Blocked {
                return Err(Error::Forbidden);
            }
            let mut session = Session::new(user.id, device);
            let authorized = session.registered(user)?;
            <Client as Put<Session>>::put(db, session).await?;
            return Ok(authorized);
        }
        let user = User::new(phone, name);
        <Client as Put<User>>::put(db, user.clone()).await?;
        let mut session = Session::new(user.id, device);
        let authorized = session.registered(user)?;
        <Client as Put<Session>>::put(db, session).await?;
        Ok(authorized)
    }

    async fn refresh(&self, user: Id, session: Id) -> Result<Registered, Error> {
        let db = self.config.db();
        let user = <Self as Users>::user(&self, user).await?;
        if user.status == Status::Blocked {
            return Err(Error::Forbidden);
        }
        let mut session = <Self as Sessions>::get(&self, session)
            .await?
            .ok_or(Error::Forbidden)?;
        let authorized = session.registered(user)?;
        <Client as Put<Session>>::put(db, session).await?;
        Ok(authorized)
    }
}
