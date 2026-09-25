use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dioxus::prelude::HttpError;
use serde_json::json;

/// An error a handler can return with `?`. Answers `{"error": message}`;
/// internal details go to the log, never the response.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: &'static str,
}

impl ApiError {
    pub fn new(status: StatusCode, message: &'static str) -> Self {
        Self { status, message }
    }

    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "not signed in")
    }

    pub fn internal() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("database error: {err}");
        Self::internal()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

/// For server functions: a database error becomes a bare 500 for the browser
/// and the details go to the log, as with `ApiError`. (Returned as-is, a
/// sqlx error's message would reach the browser.)
pub trait OrInternal<T> {
    fn or_internal(self) -> Result<T, HttpError>;
}

impl<T> OrInternal<T> for sqlx::Result<T> {
    fn or_internal(self) -> Result<T, HttpError> {
        self.map_err(|err| {
            tracing::error!("database error: {err}");
            HttpError::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
        })
    }
}

/// A 400 carrying a validation message (`PostInput::validate` and friends),
/// which is written for the user and safe to show.
pub fn bad_request(message: String) -> HttpError {
    HttpError::new(StatusCode::BAD_REQUEST, message)
}
