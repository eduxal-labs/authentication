use crate::services::{Authenticator, Sessions};
use crate::types::{Access, Error, Id, Session, Token};
use axum::debug_handler;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{self, get},
};

type Result<T, E = Error> = std::result::Result<T, E>;

pub fn router(authenticator: Authenticator) -> Router {
    Router::new()
        .route("/", get(list))
        .route("/{session}", routing::delete(delete))
        .with_state(authenticator)
}

#[debug_handler]
async fn list(
    State(authenticator): State<Authenticator>,
    token: Token<Access>,
) -> Result<Json<Vec<Session>>> {
    let user = token.subject()?;
    let sessions = authenticator.list(user).await?;
    Ok(Json(sessions))
}

#[debug_handler]
async fn delete(
    State(authenticator): State<Authenticator>,
    token: Token<Access>,
    Path(session): Path<Id>,
) -> Result<Json<()>> {
    let user = token.subject()?;
    authenticator.delete(user, session).await?;
    Ok(Json(()))
}
