//! Posts, their comments and likes.

use chrono::{DateTime, Utc};
use dioxus::{fullstack::FileStream, prelude::*};
use uuid::Uuid;

use crate::models::post::{Comment, Post, PostInput, PostKind, PostStatus};

#[cfg(feature = "server")]
use crate::{
    models::post::{POST_IMAGE_HOURLY_LIMIT, POST_IMAGE_MAX_BYTES, POST_IMAGE_SIDE, post_image_url},
    server::{
        auth::session::{CurrentUser, MaybeUser},
        db::{self, posts::PostFilter},
        error::{OrInternal, bad_request},
        images::{self, Fit, ImageError},
        state::AppState,
    },
};
#[cfg(feature = "server")]
use axum::Extension;

/// Posts per page of `list_posts`.
pub const PAGE_SIZE: i64 = 20;

/// Browse open posts, newest first, or one author's posts (closed included).
/// `funded: Some(true)` keeps only posts with money behind them. For the next
/// page, pass the last post's `created_at` as `before`.
#[get("/api/posts?kind&funded&text&author&before", user: MaybeUser, state: Extension<AppState>)]
pub async fn list_posts(
    kind: Option<PostKind>,
    funded: Option<bool>,
    text: Option<String>,
    author: Option<Uuid>,
    before: Option<DateTime<Utc>>,
) -> Result<Vec<Post>, HttpError> {
    let filter = PostFilter {
        kind,
        funded,
        text: text.filter(|t| !t.trim().is_empty()),
        author,
        before,
    };
    let viewer = user.0.map(|u| u.id);
    db::posts::list(&state.db, viewer, &filter, PAGE_SIZE).await.or_internal()
}

#[get("/api/posts/{id}", user: MaybeUser, state: Extension<AppState>)]
pub async fn get_post(id: Uuid) -> Result<Post, HttpError> {
    let viewer = user.0.map(|u| u.id);
    let post = db::posts::find(&state.db, viewer, id).await.or_internal()?;
    post.or_not_found("no such post")
}

#[post("/api/posts", user: CurrentUser, state: Extension<AppState>)]
pub async fn create_post(mut input: PostInput) -> Result<Post, HttpError> {
    input.normalize();
    input.validate().map_err(bad_request)?;
    let id = db::posts::create(&state.db, user.0.id, &input).await.or_internal()?;
    let post = db::posts::find(&state.db, Some(user.0.id), id).await.or_internal()?;
    post.or_internal_server_error("post vanished after insert")
}

/// Uploads a picture for a post being written, and returns the URL to put in
/// `PostInput::image_url`. The body is the file itself; the server redraws
/// it (see `server::images`), so what's stored is never what was sent.
#[post("/api/post-images", user: CurrentUser, state: Extension<AppState>)]
pub async fn upload_post_image(file: FileStream) -> Result<String, HttpError> {
    let recent = db::post_images::uploaded_last_hour(&state.db, user.0.id).await.or_internal()?;
    if recent >= POST_IMAGE_HOURLY_LIMIT {
        return Err(HttpError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "that's a lot of pictures for one hour; try again later",
        ));
    }

    let upload = read_upload(file, POST_IMAGE_MAX_BYTES).await?;
    if upload.is_empty() {
        return Err(bad_request("pick a picture first".into()));
    }
    let image = images::process(upload, Fit::Within(POST_IMAGE_SIDE)).await.map_err(|err| match err {
        ImageError::NotAnImage => bad_request("that doesn't look like a picture; try a JPEG, PNG, WebP or GIF".into()),
        ImageError::Internal => HttpError::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
    })?;

    let id = db::post_images::create(&state.db, user.0.id, &image).await.or_internal()?;
    tracing::info!(user_id = %user.0.id, image_id = %id, bytes = image.data.len(), "post image uploaded");
    Ok(post_image_url(id))
}

/// The whole upload, or a 413 as soon as it passes `max` bytes. Server
/// functions' body isn't size-limited, so this is what stops a huge one.
#[cfg(feature = "server")]
async fn read_upload(mut file: FileStream, max: usize) -> Result<Vec<u8>, HttpError> {
    use futures_util::StreamExt;

    let too_big = || {
        let mb = max / (1024 * 1024);
        HttpError::new(StatusCode::PAYLOAD_TOO_LARGE, format!("pictures can be up to {mb} MB"))
    };
    // The browser says how big it is up front; no point reading a file
    // that's too big.
    if file.size().is_some_and(|size| size > max as u64) {
        return Err(too_big());
    }
    let mut upload = Vec::new();
    while let Some(chunk) = file.next().await {
        let chunk = chunk.map_err(|_| bad_request("the upload broke off; try again".into()))?;
        if upload.len() + chunk.len() > max {
            return Err(too_big());
        }
        upload.extend_from_slice(&chunk);
    }
    Ok(upload)
}

#[put("/api/posts/{id}", user: CurrentUser, state: Extension<AppState>)]
pub async fn update_post(id: Uuid, mut input: PostInput) -> Result<Post, HttpError> {
    input.normalize();
    input.validate().map_err(bad_request)?;
    let updated = db::posts::update(&state.db, id, user.0.id, &input).await.or_internal()?;
    updated.or_not_found("no such post of yours")?;
    let post = db::posts::find(&state.db, Some(user.0.id), id).await.or_internal()?;
    post.or_not_found("no such post")
}

/// Close a post once it's found its people, or reopen it.
#[post("/api/posts/{id}/status", user: CurrentUser, state: Extension<AppState>)]
pub async fn set_post_status(id: Uuid, status: PostStatus) -> Result<(), HttpError> {
    let updated = db::posts::set_status(&state.db, id, user.0.id, status).await.or_internal()?;
    updated.or_not_found("no such post of yours")
}

/// Deletes the post along with its comments, likes and responses.
#[delete("/api/posts/{id}", user: CurrentUser, state: Extension<AppState>)]
pub async fn delete_post(id: Uuid) -> Result<(), HttpError> {
    let deleted = db::posts::delete(&state.db, id, user.0.id).await.or_internal()?;
    deleted.or_not_found("no such post of yours")
}

#[post("/api/posts/{id}/like", user: CurrentUser, state: Extension<AppState>)]
pub async fn like_post(id: Uuid) -> Result<(), HttpError> {
    // Liking a post that doesn't exist fails its foreign key; check first so
    // that's a 404 rather than a logged 500.
    let exists = db::posts::author_and_status(&state.db, id).await.or_internal()?;
    exists.or_not_found("no such post")?;
    db::posts::like(&state.db, id, user.0.id).await.or_internal()
}

#[delete("/api/posts/{id}/like", user: CurrentUser, state: Extension<AppState>)]
pub async fn unlike_post(id: Uuid) -> Result<(), HttpError> {
    db::posts::unlike(&state.db, id, user.0.id).await.or_internal()
}

#[get("/api/posts/{id}/comments", state: Extension<AppState>)]
pub async fn list_comments(id: Uuid) -> Result<Vec<Comment>, HttpError> {
    db::comments::list(&state.db, id).await.or_internal()
}

#[post("/api/posts/{id}/comments", user: CurrentUser, state: Extension<AppState>)]
pub async fn add_comment(id: Uuid, body: String) -> Result<Comment, HttpError> {
    let body = body.trim();
    Comment::validate_body(body).map_err(bad_request)?;
    let exists = db::posts::author_and_status(&state.db, id).await.or_internal()?;
    exists.or_not_found("no such post")?;
    db::comments::create(&state.db, id, user.0.id, body).await.or_internal()
}

/// Comment authors can delete their comments, and post authors any comment
/// on their posts.
#[delete("/api/comments/{id}", user: CurrentUser, state: Extension<AppState>)]
pub async fn delete_comment(id: Uuid) -> Result<(), HttpError> {
    let deleted = db::comments::delete(&state.db, id, user.0.id).await.or_internal()?;
    deleted.or_not_found("no such comment you can delete")
}
