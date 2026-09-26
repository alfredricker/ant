use crate::models::user::User;
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
/// or linking the identity to an existing account on first use. `picture` is
/// the provider's profile photo, used for users who don't have an avatar.
///
/// Linking goes by email, so callers must only pass an email the provider has
/// verified; otherwise anyone could claim an account by typing its address.
pub async fn find_or_create_from_identity(
    db: &PgPool,
    provider: &str,
    subject: &str,
    verified_email: &str,
    picture: Option<&str>,
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

    let user = match existing {
        Some(user) => user,
        None => link_identity(&mut tx, provider, subject, verified_email).await?,
    };

    // Fill in the provider's photo for users without an avatar, including
    // ones who removed theirs (it comes back on their next sign-in).
    let user = match picture {
        Some(picture) if user.avatar_url.is_none() => {
            sqlx::query_as!(
                User,
                "UPDATE users SET avatar_url = $2 WHERE id = $1 RETURNING *",
                user.id,
                picture,
            )
            .fetch_one(&mut *tx)
            .await?
        }
        _ => user,
    };

    tx.commit().await?;
    Ok(user)
}

/// First sign-in with this identity: a new user, or an existing one with the
/// same email.
async fn link_identity(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    provider: &str,
    subject: &str,
    verified_email: &str,
) -> sqlx::Result<User> {
    // The no-op update makes RETURNING hand back the existing row on a
    // conflict, so a user who already exists by email gets linked, not duplicated.
    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (email, email_verified) VALUES ($1, true)
         ON CONFLICT ((lower(email))) DO UPDATE SET email = users.email
         RETURNING *",
        verified_email,
    )
    .fetch_one(&mut **tx)
    .await?;

    sqlx::query!(
        "INSERT INTO user_identities (user_id, provider, subject) VALUES ($1, $2, $3)",
        user.id,
        provider,
        subject,
    )
    .execute(&mut **tx)
    .await?;

    if user.email_verified {
        return Ok(user);
    }

    // An existing account whose email nobody verified: a password sign-up,
    // or an account that changed its email. Anyone could have typed this
    // address before its owner showed up with a verified one, so the owner
    // gets the account and every other way into it goes: the password, other
    // linked identities, and open sessions. A real owner who set that
    // password just signs in with the provider from now on.
    sqlx::query!("DELETE FROM user_passwords WHERE user_id = $1", user.id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!(
        "DELETE FROM user_identities WHERE user_id = $1 AND (provider, subject) <> ($2, $3)",
        user.id,
        provider,
        subject,
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user.id)
        .execute(&mut **tx)
        .await?;
    tracing::warn!(user_id = %user.id, provider, "verified an unverified account's email; dropped its other sign-ins and sessions");

    sqlx::query_as!(
        User,
        "UPDATE users SET email_verified = true WHERE id = $1 RETURNING *",
        user.id,
    )
    .fetch_one(&mut **tx)
    .await
}

/// Sets the user's username. `None` if someone else has it, in any case.
pub async fn set_username(db: &PgPool, id: Uuid, username: &str) -> sqlx::Result<Option<User>> {
    let result = sqlx::query_as!(
        User,
        "UPDATE users SET username = $2 WHERE id = $1 RETURNING *",
        id,
        username,
    )
    .fetch_one(db)
    .await;

    match result {
        Err(sqlx::Error::Database(err)) if err.constraint() == Some("users_username_key") => Ok(None),
        other => other.map(Some),
    }
}

/// Changes the email. The new one is unverified unless only its case changed.
/// `None` if another account has it.
pub async fn set_email(db: &PgPool, id: Uuid, email: &str) -> sqlx::Result<Option<User>> {
    let result = sqlx::query_as!(
        User,
        "UPDATE users SET email = $2, email_verified = email_verified AND lower(email) = lower($2)
         WHERE id = $1 RETURNING *",
        id,
        email,
    )
    .fetch_one(db)
    .await;

    match result {
        Err(sqlx::Error::Database(err)) if err.constraint() == Some("users_email_key") => Ok(None),
        other => other.map(Some),
    }
}

pub async fn set_avatar_url(db: &PgPool, id: Uuid, url: Option<&str>) -> sqlx::Result<User> {
    sqlx::query_as!(User, "UPDATE users SET avatar_url = $2 WHERE id = $1 RETURNING *", id, url)
        .fetch_one(db)
        .await
}
