use ant_common::models::user::User;
use sqlx::{PgPool, types::Uuid};

// Every users column is safe to expose, so rows are read straight into the
// shared User. `SELECT *` keeps the check two-way: a column without a field,
// or a field without a column, fails to compile.

pub async fn find_by_id(db: &PgPool, id: Uuid) -> sqlx::Result<Option<User>> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(db)
        .await
}

/// Resolves a sign-in from an external provider to a user, creating the user
/// or linking the identity to an existing account on first use.
///
/// Linking goes by email, so callers must only pass an email the provider has
/// verified; otherwise anyone could claim an account by typing its address.
pub async fn find_or_create_from_identity(
    db: &PgPool,
    provider: &str,
    subject: &str,
    verified_email: &str,
) -> sqlx::Result<User> {
    let mut tx = db.begin().await?;

    let existing = sqlx::query_as!(
        User,
        "SELECT u.* FROM user_identities i JOIN users u ON u.id = i.user_id
         WHERE i.provider = $1 AND i.subject = $2",
        provider,
        subject,
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(user) = existing {
        return Ok(user);
    }

    // The no-op update makes RETURNING hand back the existing row on a
    // conflict, so a user who already exists by email gets linked, not duplicated.
    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (email) VALUES ($1)
         ON CONFLICT ((lower(email))) DO UPDATE SET email = users.email
         RETURNING *",
        verified_email,
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO user_identities (user_id, provider, subject) VALUES ($1, $2, $3)
         ON CONFLICT DO NOTHING",
        user.id,
        provider,
        subject,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(user)
}
