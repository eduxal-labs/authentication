use crate::db::Item;
use crate::types::{Session, User, Verification};

impl Item for Verification {
    const TABLE: &'static str = "eduxal-verifications";
}

impl Item for User {
    const TABLE: &'static str = "eduxal-users";
}

impl Item for Session {
    const TABLE: &'static str = "eduxal-sessions";
}
