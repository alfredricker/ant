use crate::models::user::User;
use sqlx::{PgPool, types::Uuid};

pub struct Credentials {
    pub user_id: Uuid,
    pub hash: String,
}

/// Signs up a new user with a password. `None` if the email is taken, by a
/// password account or a Google one.
pub async fn create_user(db: &PgPool, email: &str, hash: &str) -> sqlx::Result<Option<User>> {
    let mut tx = db.begin().await?;

    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (email) VALUES ($1)
         ON CONFLICT ((lower(email))) DO NOTHING
         RETURNING *",
        email,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(user) = user else {
        return Ok(None);
    };

    sqlx::query!(
        "INSERT INTO user_passwords (user_id, hash) VALUES ($1, $2)",
        user.id,
        hash,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Some(user))
}

/// The stored hash for an email, matched case-insensitively like the unique
/// index. `None` for unknown emails and for accounts without a password.
pub async fn find_by_email(db: &PgPool, email: &str) -> sqlx::Result<Option<Credentials>> {
    sqlx::query_as!(
        Credentials,
        "SELECT p.user_id, p.hash FROM users u JOIN user_passwords p ON p.user_id = u.id
         WHERE lower(u.email) = lower($1)",
        email,
    )
    .fetch_optional(db)
    .await
}
