//! Private responses to posts. A responder sees their own; a post's author
//! sees and sorts through everything sent to that post.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::models::response::{PostResponse, ResponseInput, ResponseStatus};

#[cfg(feature = "server")]
use crate::{
    models::{post::PostStatus, response::ResponseFilter},
    server::{
        auth::session::CurrentUser,
        db,
        error::{OrInternal, bad_request},
        state::AppState,
    },
};
#[cfg(feature = "server")]
use axum::Extension;

#[post("/api/posts/{id}/responses", user: CurrentUser, state: Extension<AppState>)]
pub async fn respond(id: Uuid, mut input: ResponseInput) -> Result<PostResponse, HttpError> {
    input.normalize();
    input.validate().map_err(bad_request)?;

    let post = db::posts::author_and_status(&state.db, id).await.or_internal()?;
    let (author, status) = post.or_not_found("no such post")?;
    (author != user.0.id).or_bad_request("you can't respond to your own post")?;
    (status == PostStatus::Open).or_bad_request("this post isn't taking responses")?;

    let created = db::responses::create(&state.db, id, user.0.id, &input).await.or_internal()?;
    let response_id = created.or_http_error(StatusCode::CONFLICT, "you've already responded to this post")?;
    let response = db::responses::find(&state.db, response_id).await.or_internal()?;
    response.or_internal_server_error("response vanished after insert")
}

/// Everything sent to one of your posts, newest first, narrowed by status,
/// attachment and words in the message.
#[get("/api/posts/{id}/responses?status&has_attachment&text", user: CurrentUser, state: Extension<AppState>)]
pub async fn list_responses(
    id: Uuid,
    status: Option<ResponseStatus>,
    has_attachment: Option<bool>,
    text: Option<String>,
) -> Result<Vec<PostResponse>, HttpError> {
    let post = db::posts::author_and_status(&state.db, id).await.or_internal()?;
    // Someone else's post looks the same as a missing one.
    post.filter(|(author, _)| *author == user.0.id)
        .or_not_found("no such post of yours")?;

    let filter = ResponseFilter {
        status,
        has_attachment,
        text: text.filter(|t| !t.trim().is_empty()),
    };
    db::responses::for_post(&state.db, id, &filter).await.or_internal()
}

/// Responses you've sent, and where they stand.
#[get("/api/responses/mine", user: CurrentUser, state: Extension<AppState>)]
pub async fn my_responses() -> Result<Vec<PostResponse>, HttpError> {
    db::responses::by_responder(&state.db, user.0.id).await.or_internal()
}

/// Accept a response to your post: returns the id of the conversation it
/// opens with the responder.
#[post("/api/responses/{id}/accept", user: CurrentUser, state: Extension<AppState>)]
pub async fn accept_response(id: Uuid) -> Result<Uuid, HttpError> {
    ensure_post_author(&state, id, user.0.id).await?;
    db::responses::accept(&state.db, id).await.or_internal()
}

/// Decline a response to your post. The responder sees it as declined.
#[post("/api/responses/{id}/decline", user: CurrentUser, state: Extension<AppState>)]
pub async fn decline_response(id: Uuid) -> Result<(), HttpError> {
    ensure_post_author(&state, id, user.0.id).await?;
    db::responses::set_status(&state.db, id, ResponseStatus::Declined)
        .await
        .or_internal()
}

/// Take back a response you sent.
#[delete("/api/responses/{id}", user: CurrentUser, state: Extension<AppState>)]
pub async fn withdraw_response(id: Uuid) -> Result<(), HttpError> {
    let deleted = db::responses::delete(&state.db, id, user.0.id).await.or_internal()?;
    deleted.or_not_found("no such response of yours")
}

/// 404 unless `user` wrote the post the response is to.
#[cfg(feature = "server")]
async fn ensure_post_author(state: &AppState, response: Uuid, user: Uuid) -> Result<(), HttpError> {
    let parties = db::responses::parties(&state.db, response).await.or_internal()?;
    parties
        .filter(|(author, _)| *author == user)
        .or_not_found("no such response to your posts")?;
    Ok(())
}
