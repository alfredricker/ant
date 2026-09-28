use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::user::PublicUser;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(
    feature = "server",
    derive(sqlx::Type),
    sqlx(type_name = "post_kind", rename_all = "lowercase")
)]
pub enum PostKind {
    Passion,
    School,
    Research,
    Art,
    Music,
    Game,
    Travel,
    Startup,
    Contract,
    Other,
}

impl PostKind {
    pub const ALL: [PostKind; 10] = [
        PostKind::Passion,
        PostKind::School,
        PostKind::Research,
        PostKind::Art,
        PostKind::Music,
        PostKind::Game,
        PostKind::Travel,
        PostKind::Startup,
        PostKind::Contract,
        PostKind::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PostKind::Passion => "Passion project",
            PostKind::School => "School project",
            PostKind::Research => "Research",
            PostKind::Art => "Art",
            PostKind::Music => "Music",
            PostKind::Game => "Game",
            PostKind::Travel => "Travel",
            PostKind::Startup => "Startup",
            PostKind::Contract => "Contract position",
            PostKind::Other => "Other",
        }
    }

    /// The lowercase name the database and the wire use, which also goes in
    /// URLs (`/?kind=travel`).
    pub fn slug(self) -> &'static str {
        match self {
            PostKind::Passion => "passion",
            PostKind::School => "school",
            PostKind::Research => "research",
            PostKind::Art => "art",
            PostKind::Music => "music",
            PostKind::Game => "game",
            PostKind::Travel => "travel",
            PostKind::Startup => "startup",
            PostKind::Contract => "contract",
            PostKind::Other => "other",
        }
    }

    pub fn from_slug(slug: &str) -> Option<PostKind> {
        PostKind::ALL.into_iter().find(|kind| kind.slug() == slug)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(
    feature = "server",
    derive(sqlx::Type),
    sqlx(type_name = "post_status", rename_all = "lowercase")
)]
pub enum PostStatus {
    /// Looking for collaborators; listed and taking responses.
    Open,
    /// No longer looking. Still readable, but unlisted.
    Closed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub author: PublicUser,
    pub kind: PostKind,
    pub status: PostStatus,
    pub title: String,
    pub body: String,
    pub looking_for: Vec<String>,
    pub funded: bool,
    pub image_url: Option<String>,
    pub like_count: i64,
    pub comment_count: i64,
    /// Whether the user asking has liked it; false when signed out.
    pub liked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What an author writes when creating or editing a post.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PostInput {
    pub kind: PostKind,
    pub title: String,
    pub body: String,
    pub looking_for: Vec<String>,
    pub funded: bool,
    pub image_url: Option<String>,
}

impl PostInput {
    pub const MAX_TITLE: usize = 120;
    pub const MAX_BODY: usize = 10_000;
    pub const MAX_TAGS: usize = 10;
    pub const MAX_TAG: usize = 40;

    /// Trims everything and drops empty or duplicate tags. Call before
    /// `validate`.
    pub fn normalize(&mut self) {
        self.title = self.title.trim().to_owned();
        self.body = self.body.trim().to_owned();
        let mut tags: Vec<String> = Vec::new();
        for tag in &self.looking_for {
            let tag = tag.trim();
            if !tag.is_empty() && !tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                tags.push(tag.to_owned());
            }
        }
        self.looking_for = tags;
        self.image_url = self
            .image_url
            .as_deref()
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(str::to_owned);
    }

    /// The same limits the database enforces, checked here so the browser
    /// can show them before a round trip.
    pub fn validate(&self) -> Result<(), String> {
        check_len(&self.title, Self::MAX_TITLE, "title")?;
        check_len(&self.body, Self::MAX_BODY, "description")?;
        if self.looking_for.len() > Self::MAX_TAGS {
            return Err(format!("at most {} skills", Self::MAX_TAGS));
        }
        if self.looking_for.iter().any(|t| t.chars().count() > Self::MAX_TAG) {
            return Err(format!("skills are at most {} characters", Self::MAX_TAG));
        }
        if let Some(url) = &self.image_url
            && !is_post_image_url(url)
        {
            check_url(url)?;
        }
        Ok(())
    }
}

/// Post pictures: the limit on what the browser sends, not what's stored
/// (the server shrinks it to fit `POST_IMAGE_SIDE`).
pub const POST_IMAGE_MAX_BYTES: usize = 8 * 1024 * 1024;
pub const POST_IMAGE_SIDE: u32 = 1600;
/// Uploads per person per hour; each picked picture is one, posted or not.
pub const POST_IMAGE_HOURLY_LIMIT: i64 = 30;

/// Where an uploaded post picture is served.
pub fn post_image_url(id: Uuid) -> String {
    format!("/api/post-images/{id}")
}

/// Whether `url` is one `post_image_url` made.
pub fn is_post_image_url(url: &str) -> bool {
    url.strip_prefix("/api/post-images/")
        .is_some_and(|id| Uuid::parse_str(id).is_ok())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author: PublicUser,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Comment {
    pub const MAX_BODY: usize = 2000;

    pub fn validate_body(body: &str) -> Result<(), String> {
        check_len(body, Self::MAX_BODY, "comment")
    }
}

/// Errors unless `text` is non-empty and at most `max` characters.
pub(crate) fn check_len(text: &str, max: usize, what: &str) -> Result<(), String> {
    match text.chars().count() {
        0 => Err(format!("{what} can't be empty")),
        n if n > max => Err(format!("{what} is too long (at most {max} characters)")),
        _ => Ok(()),
    }
}

/// Image and attachment links must at least be https.
// TODO: once uploads go through R2, accept only URLs on our CDN.
pub(crate) fn check_url(url: &str) -> Result<(), String> {
    if url.starts_with("https://") && url.len() <= 2048 {
        Ok(())
    } else {
        Err("links must start with https://".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(image_url: &str) -> PostInput {
        PostInput {
            kind: PostKind::Art,
            title: "Mural".into(),
            body: "A big wall.".into(),
            looking_for: vec![],
            funded: false,
            image_url: Some(image_url.into()),
        }
    }

    #[test]
    fn accepts_uploaded_pictures_and_https_links() {
        assert!(input(&post_image_url(Uuid::nil())).validate().is_ok());
        assert!(input("https://example.com/a.jpg").validate().is_ok());
    }

    #[test]
    fn rejects_other_image_urls() {
        for url in ["/api/post-images/nope", "/api/users/x/avatar", "javascript:alert(1)", "http://x.test/a.png"] {
            assert!(input(url).validate().is_err(), "{url}");
        }
    }

    #[test]
    fn normalize_drops_blank_and_duplicate_tags() {
        let mut post = input("https://example.com/a.jpg");
        post.looking_for = vec![" Rust ".into(), "".into(), "rust".into(), "Audio".into()];
        post.normalize();
        assert_eq!(post.looking_for, ["Rust", "Audio"]);
    }
}
