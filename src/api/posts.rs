//! Posts, their comments and likes.

use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use uuid::Uuid;

use crate::models::post::{Comment, Post, PostInput, PostKind, PostStatus};

#[cfg(feature = "server")]
use crate::server::{
    auth::session::{CurrentUser, MaybeUser},
    db::{self, posts::PostFilter},
    error::{OrInternal, bad_request},
    state::AppState,
};
#[cfg(feature = "server")]
use axum::Extension;

/// Posts per page of `list_posts`.
pub const PAGE_SIZE: i64 = 20;

/// Browse open posts, newest first, or one author's posts (closed included).
/// For the next page, pass the last post's `created_at` as `before`.
#[get("/api/posts?kind&text&author&before", user: MaybeUser, state: Extension<AppState>)]
pub async fn list_posts(
    kind: Option<PostKind>,
    text: Option<String>,
    author: Option<Uuid>,
    before: Option<DateTime<Utc>>,
) -> Result<Vec<Post>, HttpError> {
    let filter = PostFilter {
        kind,
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
