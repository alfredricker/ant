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