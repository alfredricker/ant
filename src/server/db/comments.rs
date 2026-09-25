use chrono::{DateTime, Utc};
use sqlx::{PgPool, types::Uuid};

use crate::models::{post::Comment, user::PublicUser};

struct CommentRow {
    id: Uuid,
    post_id: Uuid,
    author_id: Uuid,
    author_username: Option<String>,
    body: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CommentRow> for Comment {
    fn from(row: CommentRow) -> Self {
        Comment {
            id: row.id,
            post_id: row.post_id,
            author: PublicUser {
                id: row.author_id,
                username: row.author_username,
            },
            body: row.body,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Oldest first, like a conversation.
// TODO: paginate once posts collect enough comments to need it.
pub async fn list(db: &PgPool, post: Uuid) -> sqlx::Result<Vec<Comment>> {
    let rows = sqlx::query_as!(
        CommentRow,
        "SELECT c.id, c.post_id, c.author_id, u.username AS author_username,
                c.body, c.created_at, c.updated_at
         FROM post_comments c JOIN users u ON u.id = c.author_id
         WHERE c.post_id = $1
         ORDER BY c.created_at
         LIMIT 500",
        post,
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(Comment::from).collect())
}

pub async fn create(db: &PgPool, post: Uuid, author: Uuid, body: &str) -> sqlx::Result<Comment> {
    let row = sqlx::query_as!(
        CommentRow,
        r#"WITH c AS (
               INSERT INTO post_comments (post_id, author_id, body) VALUES ($1, $2, $3)
               RETURNING *
           )
           SELECT c.id, c.post_id, c.author_id, u.username AS author_username,
                  c.body, c.created_at, c.updated_at
           FROM c JOIN users u ON u.id = c.author_id"#,
        post,
        author,
        body,
    )
    .fetch_one(db)
    .await?;
    Ok(row.into())
}

/// Deletes a comment if `user` wrote it or wrote the post it's on (authors
/// moderate their own threads). False if neither, or if it doesn't exist.
pub async fn delete(db: &PgPool, id: Uuid, user: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM post_comments c USING posts p
         WHERE c.id = $1 AND p.id = c.post_id AND (c.author_id = $2 OR p.author_id = $2)",
        id,
        user,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}
