use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("too many requests, Please slow down")]
    SlowDown,
    #[error("verification code not found")]
    VerificationCodeNotFound,
    #[error("invalid verification code")]
    InvalidVerificationCode,
    #[error("invalid phone number. Make sure to include country code")]
    InvalidPhoneNumber,
    #[error("invalid id")]
    InvalidId,
    #[error("user not found")]
    UserNotFound,
    #[error("invalid session")]
    InvalidSession,
    #[error("invalid token")]
    InvalidToken,
    #[error("unauthorized")]
    Unauthorized,
    #[error("permission denied")]
    Forbidden,
    #[error("record already exists")]
    RecordAlreadyExists,
    #[error("nothing new to update")]
    UptoDate,
    #[error("user with this phone already exists")]
    UserAlreadyExists,
    #[error("internal server error")]
    InternalServerError,
}

impl Error {
    pub fn internal<T: std::fmt::Debug>(msg: &'static str, output: T) -> Self {
        error!("message: {:?}, output: {:?}", msg, output);
        Self::InternalServerError
    }

    pub fn server<T: std::fmt::Debug>(err: T) -> Self {
        error!("internal server error: {:?}", err);
        Self::InternalServerError
    }

    pub fn invalid_token<T: std::fmt::Display>(err: T) -> Self {
        error!("invalid token: {}", err);
        Self::InvalidToken
    }
}

#[derive(serde::Serialize)]
struct Body {
    message: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Self::SlowDown => StatusCode::TOO_MANY_REQUESTS,
            Self::VerificationCodeNotFound | Self::UserNotFound => StatusCode::NOT_FOUND,
            Self::InvalidVerificationCode
            | Self::InvalidPhoneNumber
            | Self::InvalidId
            | Self::InvalidSession
            | Self::InvalidToken
            | Self::UptoDate => StatusCode::BAD_REQUEST,
            Self::RecordAlreadyExists | Self::UserAlreadyExists => StatusCode::CONFLICT,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = self.to_string();
        let body = Json::from(Body { message });
        (status, body).into_response()
    }
}
