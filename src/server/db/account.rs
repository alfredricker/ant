//! Account settings, avatars and the facts the account page needs.

use sqlx::{PgPool, types::Uuid};

use crate::models::account::Theme;

pub async fn has_password(db: &PgPool, user_id: Uuid) -> sqlx::Result<bool> {
    let row = sqlx::query!(
        r#"SELECT EXISTS (SELECT 1 FROM user_passwords WHERE user_id = $1) AS "exists!""#,
        user_id,
    )
    .fetch_one(db)
    .await?;
    Ok(row.exists)
}

pub async fn has_identity(db: &PgPool, user_id: Uuid, provider: &str) -> sqlx::Result<bool> {
    let row = sqlx::query!(
        r#"SELECT EXISTS (SELECT 1 FROM user_identities WHERE user_id = $1 AND provider = $2) AS "exists!""#,
        user_id,
        provider,
    )
    .fetch_one(db)
    .await?;
    Ok(row.exists)
}

/// The user's theme; the default for users who never picked one.
pub async fn theme(db: &PgPool, user_id: Uuid) -> sqlx::Result<Theme> {
    let theme = sqlx::query_scalar!(
        r#"SELECT theme AS "theme: Theme" FROM user_settings WHERE user_id = $1"#,
        user_id,
    )
    .fetch_optional(db)
    .await?;
    Ok(theme.unwrap_or_default())
}

pub async fn set_theme(db: &PgPool, user_id: Uuid, theme: Theme) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO user_settings (user_id, theme) VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET theme = EXCLUDED.theme",
        user_id,
        theme as Theme,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub struct Avatar {
    pub content_type: String,
    pub data: Vec<u8>,
}

/// Stores an uploaded avatar and points the user at it. The URL carries the
/// upload time so browsers can cache each version forever.
pub async fn save_avatar(db: &PgPool, user_id: Uuid, avatar: &Avatar) -> sqlx::Result<()> {
    let mut tx = db.begin().await?;
    let updated_at = sqlx::query_scalar!(
        "INSERT INTO user_avatars (user_id, content_type, data) VALUES ($1, $2, $3)
         ON CONFLICT (user_id) DO UPDATE
         SET content_type = EXCLUDED.content_type, data = EXCLUDED.data, updated_at = now()
         RETURNING updated_at",
        user_id,
        avatar.content_type,
        avatar.data,
    )
    .fetch_one(&mut *tx)
    .await?;

    let url = format!("/api/users/{user_id}/avatar?v={}", updated_at.timestamp_millis());
    sqlx::query!("UPDATE users SET avatar_url = $2 WHERE id = $1", user_id, url)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

/// Back to the default: drops the upload, if any, and the avatar URL,
/// whether it pointed at the upload or at a Google photo.
pub async fn remove_avatar(db: &PgPool, user_id: Uuid) -> sqlx::Result<()> {
    let mut tx = db.begin().await?;
    sqlx::query!("DELETE FROM user_avatars WHERE user_id = $1", user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("UPDATE users SET avatar_url = NULL WHERE id = $1", user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

pub async fn avatar(db: &PgPool, user_id: Uuid) -> sqlx::Result<Option<Avatar>> {
    sqlx::query_as!(
        Avatar,
        "SELECT content_type, data FROM user_avatars WHERE user_id = $1",
        user_id,
    )
    .fetch_optional(db)
    .await
}
