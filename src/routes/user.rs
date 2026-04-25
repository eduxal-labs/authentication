use crate::services::{Authenticator, Users};
use crate::types::{Access, Error, Phone, Token, User, Verification};
use axum::debug_handler;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, patch, post},
};
use serde::Deserialize;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, Deserialize)]
struct Rename {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ChangePhone {
    phone: Phone,
}

#[derive(Debug, Clone, Deserialize)]
struct Verify {
    phone: Phone,
    code: String,
}

pub fn router(authenticator: Authenticator) -> Router {
    Router::new()
        .route("/", get(user))
        .route("rename", patch(rename))
        .route("chnage-phone", post(change_phone))
        .route("confirm-change-phone", patch(confirm_change_phone))
        .with_state(authenticator)
}

#[debug_handler]
async fn user(
    State(authenticator): State<Authenticator>,
    token: Token<Access>,
) -> Result<Json<User>> {
    let id = token.subject()?;
    let user = authenticator.user(id).await?;
    Ok(Json(user))
}

#[debug_handler]
async fn rename(
    State(authenticator): State<Authenticator>,
    token: Token<Access>,
    Json(Rename { name }): Json<Rename>,
) -> Result<Json<User>> {
    let id = token.subject()?;
    let user = authenticator.rename(id, name).await?;
    Ok(Json(user))
}

#[debug_handler]
async fn change_phone(
    State(authenticator): State<Authenticator>,
    token: Token<Access>,
    Json(ChangePhone { phone }): Json<ChangePhone>,
) -> Result<Json<Verification>> {
    let id = token.subject()?;
    let verification = authenticator.change_phone(id, phone).await?;
    Ok(Json(verification))
}

#[debug_handler]
async fn confirm_change_phone(
    State(authenticator): State<Authenticator>,
    token: Token<Access>,
    Json(Verify { phone, code }): Json<Verify>,
) -> Result<Json<User>> {
    let id = token.subject()?;
    let user = authenticator.confirm_change_phone(id, phone, code).await?;
    Ok(Json(user))
}
