use crate::db::Table;
use crate::services::{Authenticator, Delete, Get, List, Sessions};
use crate::types::{Error, Id, Session};
use aws_sdk_dynamodb::Client;

impl List<Id, Session> for Client {
    async fn list(&self, user: Id) -> Result<Vec<Session>, Error> {
        let key = [("user", user.into())];
        <Self as Table<Session>>::list(&self, key).await
    }
}

impl Get<Id, Session> for Client {
    async fn get(&self, id: Id) -> Result<Option<Session>, Error> {
        let key = [(String::from("id"), id.into())];
        <Self as Table<Session>>::get(&self, key).await
    }
}

impl Delete<Id, Session> for Client {
    async fn delete(&self, id: Id) -> Result<(), Error> {
        let key = [(String::from("id"), id.into())];
        <Self as Table<Session>>::delete(&self, key).await
    }
}

impl Sessions for Authenticator {
    async fn list(&self, user: Id) -> Result<Vec<Session>, Error> {
        let db = self.config.db();
        <Client as List<Id, Session>>::list(db, user).await
    }

    async fn get(&self, id: Id) -> Result<Option<Session>, Error> {
        let db = self.config.db();
        <Client as Get<Id, Session>>::get(db, id).await
    }

    async fn delete(&self, user: Id, session: Id) -> Result<(), Error> {
        let db = self.config.db();
        let session = <Self as Sessions>::get(&self, session).await?;
        let session = match session {
            None => return Ok(()),
            Some(session) => {
                if session.user != user {
                    return Err(Error::Forbidden);
                }
                Ok(session.id)
            }
        }?;
        <Client as Delete<Id, Session>>::delete(db, session).await
    }
}
