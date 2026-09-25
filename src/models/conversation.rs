//! Direct messages between two people.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{post::check_len, user::PublicUser};

/// One row of the inbox.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: Uuid,
    /// The other person.
    pub with: PublicUser,
    /// The post it started from, if any and if it still exists.
    pub post_id: Option<Uuid>,
    pub last_message: Option<Message>,
    pub last_message_at: DateTime<Utc>,
    /// Messages from the other person since this user last read it.
    pub unread_count: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub const MAX_BODY: usize = 4000;

    pub fn validate_body(body: &str) -> Result<(), String> {
        check_len(body, Self::MAX_BODY, "message")
    }
}
