//! Serving post pictures. Uploading them is a server function
//! (`api::posts::upload_post_image`), since the composer sends the file from
//! the browser; serving needs real image responses.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::get,
};
use sqlx::types::Uuid;

use crate::server::{db, error::ApiError, routes::image_response, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/post-images/{id}", get(serve))
}

async fn serve(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Response, ApiError> {
    let image = db::post_images::find(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no such image"))?;
    Ok(image_response(&image.content_type, image.data))
}
