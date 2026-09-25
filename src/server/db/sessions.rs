use crate::models::user::User;
use sqlx::{PgPool, types::Uuid};

/// Stores a new session, and prunes expired ones while we're here.
pub async fn create(db: &PgPool, token_hash: &[u8], user_id: Uuid, days: i32) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM sessions WHERE expires_at < now()")
        .execute(db)
        .await?;
    sqlx::query!(
        "INSERT INTO sessions (token_hash, user_id, expires_at)
         VALUES ($1, $2, now() + make_interval(days => $3))",
        token_hash,
        user_id,
        days,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn find_user(db: &PgPool, token_hash: &[u8]) -> sqlx::Result<Option<User>> {
    sqlx::query_as!(
        User,
        "SELECT u.* FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = $1 AND s.expires_at > now()",
        token_hash,
    )
    .fetch_optional(db)
    .await
}

pub async fn delete(db: &PgPool, token_hash: &[u8]) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", token_hash)
        .execute(db)
        .await?;
    Ok(())
}
