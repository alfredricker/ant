use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: Option<String>,
    pub email: String,
    pub is_superuser: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// The username if one is set, otherwise the email.
    pub fn display_name(&self) -> &str {
        self.username.as_deref().unwrap_or(&self.email)
    }
}

/// What anyone can see about a user: authors of posts and comments, the other
/// side of a conversation. No email, which `User` carries for its owner only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PublicUser {
    pub id: Uuid,
    pub username: Option<String>,
}

impl PublicUser {
    /// The username, or a placeholder for users who haven't picked one; the
    /// email fallback `User::display_name` uses would leak it.
    pub fn display_name(&self) -> &str {
        self.username.as_deref().unwrap_or("someone")
    }
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self { id: user.id, username: user.username }
    }
}
