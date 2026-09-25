//! Private responses to a post: someone saying "I'd like to help", seen only
//! by them and the post's author.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    post::{check_len, check_url},
    user::PublicUser,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(
    feature = "server",
    derive(sqlx::Type),
    sqlx(type_name = "response_status", rename_all = "lowercase")
)]
pub enum ResponseStatus {
    Pending,
    /// The author wants to talk; a conversation between the two is open.
    Accepted,
    Declined,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PostResponse {
    pub id: Uuid,
    pub post_id: Uuid,
    pub responder: PublicUser,
    pub message: String,
    pub attachment_url: Option<String>,
    pub status: ResponseStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What a responder writes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponseInput {
    pub message: String,
    pub attachment_url: Option<String>,
}

impl ResponseInput {
    pub const MAX_MESSAGE: usize = 4000;

    pub fn normalize(&mut self) {
        self.message = self.message.trim().to_owned();
        self.attachment_url = self
            .attachment_url
            .as_deref()
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(str::to_owned);
    }

    pub fn validate(&self) -> Result<(), String> {
        check_len(&self.message, Self::MAX_MESSAGE, "message")?;
        if let Some(url) = &self.attachment_url {
            check_url(url)?;
        }
        Ok(())
    }
}

/// How a post's author narrows down its responses. Every field is optional;
/// the default matches everything.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ResponseFilter {
    pub status: Option<ResponseStatus>,
    pub has_attachment: Option<bool>,
    /// Words to look for in the message (postgres full-text search, so
    /// "designing" also finds "design").
    pub text: Option<String>,
}
