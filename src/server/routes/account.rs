//! Avatar upload and serving. Plain axum routes: the upload is a multipart
//! form (works before the wasm loads, and file bytes don't fit server
//! functions' JSON), and serving needs real image responses.

use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use sqlx::types::Uuid;

use crate::{
    models::account::{AVATAR_MAX_BYTES, AvatarNotice},
    server::{
        auth::session::CurrentUser,
        avatar::{self, AvatarError},
        db,
        error::ApiError,
        state::AppState,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/account/avatar",
            // Room for the multipart framing around the largest allowed file.
            post(upload).layer(DefaultBodyLimit::max(AVATAR_MAX_BYTES + 64 * 1024)),
        )
        .route("/api/account/avatar/remove", post(remove))
        .route("/api/users/{id}/avatar", get(serve))
}

fn back_to_account(notice: AvatarNotice) -> Redirect {
    Redirect::to(&format!("/account?notice={}", notice.slug()))
}

async fn upload(
    State(state): State<AppState>,
    user: CurrentUser,
    mut form: Multipart,
) -> Result<Redirect, ApiError> {
    let Some(file) = read_file(&mut form).await else {
        return Ok(back_to_account(AvatarNotice::TooBig));
    };
    if file.is_empty() {
        return Ok(back_to_account(AvatarNotice::Missing));
    }

    let avatar = match avatar::process(file.to_vec()).await {
        Ok(avatar) => avatar,
        Err(AvatarError::NotAnImage) => return Ok(back_to_account(AvatarNotice::NotAnImage)),
        Err(AvatarError::Internal) => return Err(ApiError::internal()),
    };
    db::account::save_avatar(&state.db, user.0.id, &avatar).await?;
    tracing::info!(user_id = %user.0.id, bytes = avatar.data.len(), "avatar uploaded");

    Ok(back_to_account(AvatarNotice::Saved))
}

/// The `avatar` field's bytes: empty when the form had no file, `None` when
/// the body broke off, which in practice means it went over the size limit.
async fn read_file(form: &mut Multipart) -> Option<Bytes> {
    while let Some(field) = form.next_field().await.ok()? {
        if field.name() == Some("avatar") {
            return field.bytes().await.ok();
        }
    }
    Some(Bytes::new())
}

async fn remove(State(state): State<AppState>, user: CurrentUser) -> Result<Redirect, ApiError> {
    db::account::remove_avatar(&state.db, user.0.id).await?;
    Ok(back_to_account(AvatarNotice::Removed))
}

async fn serve(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Response, ApiError> {
    let avatar = db::account::avatar(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no such avatar"))?;

    let headers = [
        // Stored by us, so only ever these two.
        (header::CONTENT_TYPE, HeaderValue::from_static(if avatar.content_type == "image/png" { "image/png" } else { "image/jpeg" })),
        // Each upload gets a new `?v=` in the URL, so any version can be
        // cached for good.
        (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=31536000, immutable")),
        (header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
        (header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'none'; sandbox")),
    ];
    Ok((headers, avatar.data).into_response())
}
