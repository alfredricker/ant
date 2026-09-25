use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
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
