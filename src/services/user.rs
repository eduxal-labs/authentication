use crate::db::Table;
use crate::services::{Authenticator, Delete, Find, Get, Put, Update, Users};
use crate::types::{Error, Id, Phone, Purpose, User, Verification};
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::Utc;

impl Get<Phone, User> for Client {
    async fn get(&self, phone: Phone) -> Result<Option<User>, Error> {
        let key = [("phone", phone.into())];
        let items = <Self as Table<User>>::list(&self, key, Some("phone-index")).await?;
        Ok(items.into_iter().next())
    }
}

impl Find<Id, User> for Client {
    async fn find(&self, id: Id) -> Result<User, Error> {
        let key = [(String::from("id"), id.into())];
        <Self as Table<User>>::find(&self, key).await
    }
}

impl Update<Id, String, User> for Client {
    async fn update(&self, id: Id, name: String) -> Result<Option<User>, Error> {
        let key = [(String::from("id"), id.into())];
        let update = [(String::from("name"), AttributeValue::S(name))];
        <Self as Table<User>>::update(&self, key, update).await
    }
}

impl Update<Id, Phone, User> for Client {
    async fn update(&self, id: Id, phone: Phone) -> Result<Option<User>, Error> {
        let key = [(String::from("id"), id.into())];
        let update = [(String::from("phone"), phone.into())];
        <Self as Table<User>>::update(&self, key, update).await
    }
}

impl Delete<Id, User> for Client {
    async fn delete(&self, id: Id) -> Result<(), Error> {
        let key = [(String::from("id"), id.into())];
        <Self as Table<User>>::delete(&self, key).await
    }
}

impl Users for Authenticator {
    async fn user(&self, id: Id) -> Result<User, Error> {
        let db = self.config.db();
        <Client as Find<Id, User>>::find(db, id).await
    }

    async fn rename(&self, id: Id, name: String) -> Result<User, Error> {
        let db = self.config.db();
        let user = <Client as Update<Id, String, User>>::update(db, id, name)
            .await?
            .ok_or(Error::UserNotFound)?;
        Ok(user)
    }

    async fn change_phone(&self, id: Id, phone: Phone) -> Result<Verification, Error> {
        let db = self.config.db();
        let existing = <Client as Get<Phone, User>>::get(db, phone.clone()).await?;
        if let Some(_) = existing {
            return Err(Error::UserAlreadyExists);
        }
        let user = <Client as Find<Id, User>>::find(db, id).await?;
        if user.phone == phone {
            return Err(Error::UptoDate);
        }
        let existing = <Client as Get<Phone, Verification>>::get(db, phone.clone()).await?;
        if let Some(verification) = existing {
            if Utc::now().timestamp() < (verification.created + Verification::RLTS).timestamp() {
                return Err(Error::SlowDown);
            }
        }
        let verification = Verification::chnage_phone(phone.clone(), id);
        let messenger = self.config.messenger();
        let (receipient, code) = (phone.as_ref(), verification.code.as_str());
        messenger.send(receipient, code).await?;
        <Client as Put<Verification>>::put(db, verification.clone()).await?;
        Ok(verification)
    }

    async fn confirm_change_phone(
        &self,
        id: Id,
        phone: Phone,
        code: String,
    ) -> Result<User, Error> {
        let db = self.config.db();
        let existing = <Client as Get<Phone, User>>::get(db, phone.clone()).await?;
        if let Some(_) = existing {
            return Err(Error::UserAlreadyExists);
        }
        let verification = <Client as Find<Phone, Verification>>::find(db, phone.clone()).await?;
        if verification.code != code
            || Utc::now().timestamp() >= verification.ttl.timestamp()
            || verification.user != Some(id)
            || verification.purpose != Purpose::ChangePhone
        {
            return Err(Error::InvalidVerificationCode);
        }
        let user = <Client as Update<Id, Phone, User>>::update(db, id, phone.clone())
            .await?
            .ok_or(Error::UserNotFound)?;
        <Client as Delete<Phone, Verification>>::delete(db, phone).await?;
        Ok(user)
    }

    async fn delete(&self, id: Id) -> Result<Verification, Error> {
        let db = self.config.db();
        let user = <Client as Find<Id, User>>::find(db, id).await?;
        let existing = <Client as Get<Phone, Verification>>::get(db, user.phone.clone()).await?;
        if let Some(verification) = existing {
            if Utc::now().timestamp() < (verification.created + Verification::RLTS).timestamp() {
                return Err(Error::SlowDown);
            }
        }
        let verification = Verification::delete_user(user.phone.clone(), user.id);
        let messenger = self.config.messenger();
        let (receipient, code) = (user.phone.as_ref(), verification.code.as_str());
        messenger.send(receipient, code).await?;
        <Client as Put<Verification>>::put(db, verification.clone()).await?;
        Ok(verification)
    }

    async fn confirm_delete(&self, id: Id, phone: Phone, code: String) -> Result<(), Error> {
        let db = self.config.db();
        let verification = <Client as Find<Phone, Verification>>::find(db, phone.clone()).await?;
        match verification.user {
            Some(user) => {
                if user != id {
                    return Err(Error::Forbidden);
                }
            }
            None => return Err(Error::Forbidden),
        }
        if verification.code != code
            || Utc::now().timestamp() >= verification.ttl.timestamp()
            || verification.purpose != Purpose::DeleteUser
        {
            return Err(Error::InvalidVerificationCode);
        }
        <Client as Delete<Id, User>>::delete(db, id).await?;
        Ok(())
    }
}
