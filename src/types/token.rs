use crate::types::{Error, Id, Phone};
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use chrono::{DateTime, Duration, Utc};
use rand::RngExt;
use rusty_paseto::core::{Key, Local, Paseto, PasetoNonce, PasetoSymmetricKey, Payload, V4};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::str::FromStr;

macros::key!("PASETO_PASSWORD");

/// TTL for access tokens in days.
const ACCESS_TTL: i64 = 2;
/// TTL for refresh tokens in days.
const REFRESH_TTL: i64 = 52;
/// TTL for setup tokens in minutes.
const SETUP_TTL: i64 = 15;

// ── Purpose trait ─────────────────────────────────────────────────────────────

/// Marker trait implemented by the three purpose types.
///
/// By making [`Token`] generic over `P: Purpose`, the compiler enforces at
/// the call-site that only the correct token kind is accepted by a handler:
///
/// ```
/// async fn refresh(token: Token<Refresh>) { ... }  // only refresh tokens
/// async fn dashboard(token: Token<Access>) { ... }  // only access tokens
/// ```
pub trait Purpose: Send + Sync + 'static {
    /// How long tokens of this purpose remain valid, in days.
    const TTL: Duration;
    /// The discriminant stored inside the encrypted payload (`1` = Access, `2` = Refresh, `3` = Setup).
    const KIND: u8;
}

// ── Purpose marker structs ────────────────────────────────────────────────────

/// Marker for short-lived API access tokens.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Access;

/// Marker for long-lived refresh tokens.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Refresh;

/// Marker for single-use phone-verification / account-setup tokens.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Setup;

impl Purpose for Access {
    const TTL: Duration = Duration::days(ACCESS_TTL);
    const KIND: u8 = 1;
}

impl Purpose for Refresh {
    const TTL: Duration = Duration::days(REFRESH_TTL);
    const KIND: u8 = 2;
}

impl Purpose for Setup {
    const TTL: Duration = Duration::minutes(SETUP_TTL);
    const KIND: u8 = 3;
}

// ── Subject ───────────────────────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum Subject {
    /// A real, registered user.
    Id(Id),
    /// A verified phone number (pre-registration).
    Phone(Phone),
}

// ── Internal wire format ──────────────────────────────────────────────────────

/// The JSON that actually travels inside the PASETO envelope.
/// Kept private; the public API uses [`Token<P>`].
#[derive(Serialize, Deserialize)]
struct RawToken {
    id: Id,
    subject: Subject,
    session: Option<Id>,
    purpose: u8,
    created: DateTime<Utc>,
    expires: DateTime<Utc>,
}

// ── Token<P> ──────────────────────────────────────────────────────────────────

/// A validated, purpose-typed token.
///
/// The generic parameter `P` is one of [`Access`], [`Refresh`], or [`Setup`].
/// Use it as an Axum extractor — [`FromRequestParts`] will decrypt the bearer
/// token, verify it has not expired, and confirm its purpose matches `P`
/// before handing it to the handler.
#[derive(Clone, Debug, PartialEq)]
pub struct Token<P: Purpose> {
    pub id: Id,
    pub subject: Subject,
    pub session: Option<Id>,
    pub created: DateTime<Utc>,
    pub expires: DateTime<Utc>,
    _purpose: PhantomData<P>,
}

impl<P: Purpose> Token<P> {
    fn new(subject: Subject, session: Option<Id>) -> Self {
        let id = Id::default();
        let created = Utc::now();
        let expires = created + P::TTL;
        Self {
            id,
            subject,
            session,
            created,
            expires,
            _purpose: PhantomData,
        }
    }

    /// Encrypt this token into a PASETO v4-local string.
    pub fn tokenize(&self) -> Result<String, Error> {
        let raw = RawToken {
            id: self.id,
            subject: self.subject.clone(),
            session: self.session,
            purpose: P::KIND,
            created: self.created,
            expires: self.expires,
        };
        let key = Key::from(KEY);
        let key = PasetoSymmetricKey::<V4, Local>::from(key);
        let json = serde_json::to_string(&raw).map_err(Error::server)?;
        let payload = Payload::from(json.as_str());
        let mut nonce = [0u8; 32];
        rand::rng().fill(&mut nonce);
        let nonce = Key::from(nonce);
        let nonce = &PasetoNonce::from(&nonce);
        Paseto::<V4, Local>::builder()
            .set_payload(payload)
            .try_encrypt(&key, nonce)
            .map_err(Error::server)
    }
}

// ── Specialised constructors ──────────────────────────────────────────────────

impl Token<Access> {
    pub fn access(user: Id, session: Id) -> Self {
        Self::new(Subject::Id(user), Some(session))
    }

    pub fn subject(&self) -> Result<Id, Error> {
        match self.subject {
            Subject::Id(id) => Ok(id),
            Subject::Phone(_) => Err(Error::server(
                "Access token with phone as subject instead of the user id",
            )),
        }
    }
}

impl Token<Refresh> {
    pub fn refresh(user: Id, session: Id) -> Self {
        Self::new(Subject::Id(user), Some(session))
    }

    pub fn subject(&self) -> Result<Id, Error> {
        match self.subject {
            Subject::Id(id) => Ok(id),
            Subject::Phone(_) => Err(Error::server(
                "Refresh token with phone as subject instead of the user id",
            )),
        }
    }

    pub fn session(&self) -> Result<Id, Error> {
        match self.session {
            Some(id) => Ok(id),
            None => Err(Error::server(
                "Valid Refresh with token with a Null session.",
            )),
        }
    }
}

impl Token<Setup> {
    pub fn setup(phone: Phone) -> Self {
        Self::new(Subject::Phone(phone), None)
    }

    pub fn subject(&self) -> Result<Phone, Error> {
        match &self.subject {
            Subject::Phone(phone) => Ok(phone.clone()),
            Subject::Id(_) => Err(Error::server(
                "Setup token with id as subject instead of the user's phone",
            )),
        }
    }
}

// ── FromStr ───────────────────────────────────────────────────────────────────

impl<P: Purpose> FromStr for Token<P> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key = Key::from(KEY);
        let key = &PasetoSymmetricKey::from(key);
        let json =
            Paseto::<V4, Local>::try_decrypt(s, key, None, None).map_err(Error::invalid_token)?;
        let raw = serde_json::from_str::<RawToken>(&json).map_err(Error::invalid_token)?;

        if raw.expires.timestamp() < Utc::now().timestamp() {
            return Err(Error::Unauthorized);
        }
        if raw.purpose != P::KIND {
            return Err(Error::Forbidden);
        }

        Ok(Token {
            id: raw.id,
            subject: raw.subject,
            session: raw.session,
            created: raw.created,
            expires: raw.expires,
            _purpose: PhantomData,
        })
    }
}

// ── Axum extractor ────────────────────────────────────────────────────────────

impl<S, P> FromRequestParts<S> for Token<P>
where
    S: Send + Sync,
    P: Purpose,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(Error::Unauthorized)?;

        let token_str = auth_header
            .strip_prefix("Bearer ")
            .ok_or(Error::Unauthorized)?;

        Token::<P>::from_str(token_str)
    }
}
