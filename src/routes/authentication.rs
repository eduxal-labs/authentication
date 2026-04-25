use crate::services::{Authentication, Authenticator};
use crate::types::{Authorized, Error, Id, Phone, Refresh, Registered, Token, Verification};
use axum::{
    Json, Router, debug_handler,
    extract::State,
    routing::{get, post},
};
use serde::Deserialize;

type Result<T, E = Error> = std::result::Result<T, E>;

pub fn router(authenticator: Authenticator) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/verify", post(verify))
        .route("/setup", post(setup))
        .route("refresh", get(refresh))
        .with_state(authenticator)
}

#[derive(Clone, Debug, Deserialize)]
struct Login {
    phone: Phone,
}

#[derive(Clone, Debug, Deserialize)]
struct Verify {
    phone: Phone,
    code: String,
    session: Option<Id>,
    device: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Setup {
    name: String,
    device: String,
}

#[debug_handler]
async fn login(
    State(authenticator): State<Authenticator>,
    Json(Login { phone }): Json<Login>,
) -> Result<Json<Verification>> {
    let verification = authenticator.login(phone).await?;
    Ok(Json(verification))
}

#[debug_handler]
async fn verify(
    State(authenticator): State<Authenticator>,
    Json(Verify {
        phone,
        code,
        session,
        device,
    }): Json<Verify>,
) -> Result<Json<Authorized>> {
    let authorized = authenticator.verify(phone, code, session, device).await?;
    Ok(Json(authorized))
}

#[debug_handler]
async fn setup(
    State(authenticator): State<Authenticator>,
    token: Token<crate::types::Setup>,
    Json(Setup { name, device }): Json<Setup>,
) -> Result<Json<Registered>> {
    let phone = token.subject()?;
    let registered = authenticator.setup(phone, name, device).await?;
    Ok(Json(registered))
}

#[debug_handler]
async fn refresh(
    State(authenticator): State<Authenticator>,
    token: Token<Refresh>,
) -> Result<Json<Registered>> {
    let user = token.subject()?;
    let session = token.session()?;
    let registered = authenticator.refresh(user, session).await?;
    Ok(Json(registered))
}
