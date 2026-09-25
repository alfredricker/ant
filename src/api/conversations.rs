//! Direct messages. Conversations open when a post's author accepts a
//! response (`responses::accept_response`); there's no cold DM yet.

use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use uuid::Uuid;

use crate::models::conversation::{ConversationSummary, Message};

#[cfg(feature = "server")]
use crate::server::{
    auth::session::CurrentUser,
    db,
    error::{OrInternal, bad_request},
    state::AppState,
};
#[cfg(feature = "server")]
use axum::Extension;

/// Messages per page of `list_messages`.
pub const PAGE_SIZE: i64 = 50;

/// Your inbox, most recently active first.
#[get("/api/conversations", user: CurrentUser, state: Extension<AppState>)]
pub async fn list_conversations() -> Result<Vec<ConversationSummary>, HttpError> {
    db::conversations::list(&state.db, user.0.id).await.or_internal()
}

#[get("/api/conversations/{id}", user: CurrentUser, state: Extension<AppState>)]
pub async fn get_conversation(id: Uuid) -> Result<ConversationSummary, HttpError> {
    let conversation = db::conversations::find(&state.db, user.0.id, id).await.or_internal()?;
    conversation.or_not_found("no such conversation")
}

/// Newest first. For older messages, pass the oldest one's `created_at` as
/// `before`.
#[get("/api/conversations/{id}/messages?before", user: CurrentUser, state: Extension<AppState>)]
pub async fn list_messages(id: Uuid, before: Option<DateTime<Utc>>) -> Result<Vec<Message>, HttpError> {
    ensure_member(&state, id, user.0.id).await?;
    db::conversations::messages(&state.db, id, before, PAGE_SIZE)
        .await
        .or_internal()
}

#[post("/api/conversations/{id}/messages", user: CurrentUser, state: Extension<AppState>)]
pub async fn send_message(id: Uuid, body: String) -> Result<Message, HttpError> {
    let body = body.trim();
    Message::validate_body(body).map_err(bad_request)?;
    ensure_member(&state, id, user.0.id).await?;
    db::conversations::send(&state.db, id, user.0.id, body).await.or_internal()
}

/// Marks everything in the conversation read, for the unread counts.
#[post("/api/conversations/{id}/read", user: CurrentUser, state: Extension<AppState>)]
pub async fn mark_read(id: Uuid) -> Result<(), HttpError> {
    db::conversations::mark_read(&state.db, id, user.0.id).await.or_internal()
}

/// 404 unless `user` is in the conversation; other people's look missing.
#[cfg(feature = "server")]
async fn ensure_member(state: &AppState, conversation: Uuid, user: Uuid) -> Result<(), HttpError> {
    let member = db::conversations::is_member(&state.db, conversation, user)
        .await
        .or_internal()?;
    member.or_not_found("no such conversation")
}
