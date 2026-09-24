use ant_common::models::user::User;
use sqlx::{
    PgPool,
    types::{
        Uuid,
        chrono::{DateTime, Utc},
    },
};

/// One `users` row, column for column. Fetched with `SELECT *`, so adding or
/// dropping a column in a migration without updating this struct (or the
/// reverse) fails to compile. Holds `password_hash`, so it never leaves the
/// server; convert to `User` before responding.
pub struct UserRow {
    pub id: Uuid,
    pub username: Option<String>,
    pub email: String,
    pub password_hash: String,
    pub is_superuser: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Exhaustive on purpose: a new field on the shared `User` is a compile error
// here until it's mapped from the row.
impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        User {
            id: row.id,
            username: row.username,
            email: row.email,
            is_superuser: row.is_superuser,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

pub async fn find_by_id(db: &PgPool, id: Uuid) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as!(UserRow, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(db)
        .await
}
