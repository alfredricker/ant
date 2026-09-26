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
    /// Set for emails Google vouched for; cleared when the email changes.
    pub email_verified: bool,
    /// The Google photo or an uploaded one; `None` shows a default insect.
    pub avatar_url: Option<String>,
}

impl User {
    pub const MIN_USERNAME: usize = 3;
    pub const MAX_USERNAME: usize = 30;

    /// The username if one is set, otherwise the email.
    pub fn display_name(&self) -> &str {
        self.username.as_deref().unwrap_or(&self.email)
    }

    /// ASCII letters, digits, `_` and `-`, so names are easy to type, can go
    /// in a URL as-is, and can't impersonate each other with lookalike
    /// characters. Case is kept for display; uniqueness ignores it (see the
    /// `users_username_key` index).
    pub fn validate_username(name: &str) -> Result<(), String> {
        let len = name.chars().count();
        if !(Self::MIN_USERNAME..=Self::MAX_USERNAME).contains(&len) {
            return Err(format!(
                "usernames are {} to {} characters",
                Self::MIN_USERNAME,
                Self::MAX_USERNAME,
            ));
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err("usernames can only use letters, numbers, _ and -".into());
        }
        if RESERVED_USERNAMES.contains(&name.to_ascii_lowercase().as_str()) {
            return Err("that username is reserved".into());
        }
        Ok(())
    }
}

/// Names that would pass for the site or a staff account, or clash with our
/// own paths if profiles get `/<username>` URLs. Compared lowercased.
const RESERVED_USERNAMES: &[&str] = &[
    "account", "admin", "administrator", "api", "help", "me", "mod", "moderator", "post", "root",
    "sandhouse", "settings", "signin", "signout", "someone", "staff", "support", "system",
];

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames() {
        for ok in ["maya", "Maya_2", "a-b", &"x".repeat(30)] {
            assert_eq!(User::validate_username(ok), Ok(()), "{ok:?}");
        }
        for bad in ["", "ab", &"x".repeat(31), "has space", "émile", "a.b", "Admin", "someone"] {
            assert!(User::validate_username(bad).is_err(), "{bad:?}");
        }
    }
}
