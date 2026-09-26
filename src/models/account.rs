//! What the account page reads and writes, beyond the `User` itself.

use serde::{Deserialize, Serialize};

use super::user::User;

/// Color theme. `System` follows the OS; the others pin it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(sqlx::Type),
    sqlx(type_name = "theme", rename_all = "lowercase")
)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::System, Theme::Light, Theme::Dark];

    /// The `data-theme` attribute value the stylesheets switch on.
    pub fn slug(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "Match system",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

/// Everything the account page shows about the signed-in user.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountDetails {
    pub user: User,
    /// Whether email + password sign-in works; changing the email or the
    /// password then asks for the current one.
    pub has_password: bool,
    pub google_linked: bool,
    pub theme: Theme,
}

/// Uploaded avatars: the limit on what the browser sends, not what's stored
/// (the server shrinks it to `AVATAR_SIZE` square).
pub const AVATAR_MAX_BYTES: usize = 8 * 1024 * 1024;
pub const AVATAR_SIZE: u32 = 256;

/// The avatar form is a plain multipart post; the server redirects back to
/// `/account?notice=<slug>` with one of these. Only these fixed messages are
/// shown, never text from the query string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AvatarNotice {
    Saved,
    Removed,
    TooBig,
    NotAnImage,
    Missing,
}

impl AvatarNotice {
    const ALL: [AvatarNotice; 5] = [
        AvatarNotice::Saved,
        AvatarNotice::Removed,
        AvatarNotice::TooBig,
        AvatarNotice::NotAnImage,
        AvatarNotice::Missing,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Saved => "avatar-saved",
            Self::Removed => "avatar-removed",
            Self::TooBig => "avatar-too-big",
            Self::NotAnImage => "avatar-not-image",
            Self::Missing => "avatar-missing",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|notice| notice.slug() == slug)
    }

    pub fn is_error(self) -> bool {
        !matches!(self, Self::Saved | Self::Removed)
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::Saved => "New photo saved.",
            Self::Removed => "Photo removed.",
            Self::TooBig => "That file is too big; photos can be up to 8 MB.",
            Self::NotAnImage => "That doesn't look like a photo. Try a JPEG, PNG, WebP or GIF.",
            Self::Missing => "Pick a photo first.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notice_slugs_round_trip() {
        for notice in AvatarNotice::ALL {
            assert_eq!(AvatarNotice::from_slug(notice.slug()), Some(notice));
        }
        assert_eq!(AvatarNotice::from_slug("<b>hi</b>"), None);
    }
}
