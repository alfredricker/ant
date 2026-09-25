use chrono::{DateTime, Utc};
use sqlx::{PgPool, types::Uuid};

use crate::{
    models::{
        response::{PostResponse, ResponseFilter, ResponseInput, ResponseStatus},
        user::PublicUser,
    },
    server::db::conversations,
};

struct ResponseRow {
    id: Uuid,
    post_id: Uuid,
    responder_id: Uuid,
    responder_username: Option<String>,
    message: String,
    attachment_url: Option<String>,
    status: ResponseStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ResponseRow> for PostResponse {
    fn from(row: ResponseRow) -> Self {
        PostResponse {
            id: row.id,
            post_id: row.post_id,
            responder: PublicUser {
                id: row.responder_id,
                username: row.responder_username,
            },
            message: row.message,
            attachment_url: row.attachment_url,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// The one query that reads responses: by id, a post's (for its author), or
/// a responder's own, narrowed by `filter`. Newest first.
async fn select(
    db: &PgPool,
    id: Option<Uuid>,
    post: Option<Uuid>,
    responder: Option<Uuid>,
    filter: &ResponseFilter,
) -> sqlx::Result<Vec<PostResponse>> {
    let rows = sqlx::query_as!(
        ResponseRow,
        r#"SELECT r.id, r.post_id, r.responder_id, u.username AS responder_username,
                  r.message, r.attachment_url, r.status AS "status: ResponseStatus",
                  r.created_at, r.updated_at
           FROM post_responses r JOIN users u ON u.id = r.responder_id
           WHERE ($1::uuid IS NULL OR r.id = $1)
             AND ($2::uuid IS NULL OR r.post_id = $2)
             AND ($3::uuid IS NULL OR r.responder_id = $3)
             AND ($4::response_status IS NULL OR r.status = $4)
             AND ($5::bool IS NULL OR (r.attachment_url IS NOT NULL) = $5)
             AND ($6::text IS NULL OR r.search @@ websearch_to_tsquery('english', $6))
           ORDER BY r.created_at DESC
           LIMIT 500"#,
        id,
        post,
        responder,
        filter.status as Option<ResponseStatus>,
        filter.has_attachment,
        filter.text,
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(PostResponse::from).collect())
}

pub async fn find(db: &PgPool, id: Uuid) -> sqlx::Result<Option<PostResponse>> {
    Ok(select(db, Some(id), None, None, &ResponseFilter::default())
        .await?
        .pop())
}

pub async fn for_post(db: &PgPool, post: Uuid, filter: &ResponseFilter) -> sqlx::Result<Vec<PostResponse>> {
    select(db, None, Some(post), None, filter).await
}

pub async fn by_responder(db: &PgPool, responder: Uuid) -> sqlx::Result<Vec<PostResponse>> {
    select(db, None, None, Some(responder), &ResponseFilter::default()).await
}

/// None if this user has already responded to the post.
pub async fn create(
    db: &PgPool,
    post: Uuid,
    responder: Uuid,
    input: &ResponseInput,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar!(
        "INSERT INTO post_responses (post_id, responder_id, message, attachment_url)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (post_id, responder_id) DO NOTHING
         RETURNING id",
        post,
        responder,
        input.message,
        input.attachment_url,
    )
    .fetch_optional(db)
    .await
}

/// The post's author and the responder, for permission checks.
pub async fn parties(db: &PgPool, id: Uuid) -> sqlx::Result<Option<(Uuid, Uuid)>> {
    let row = sqlx::query!(
        "SELECT p.author_id, r.responder_id
         FROM post_responses r JOIN posts p ON p.id = r.post_id
         WHERE r.id = $1",
        id,
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| (r.author_id, r.responder_id)))
}

/// Marks a response declined (or back to pending). Accepting goes through
/// `accept`, which also opens the conversation.
pub async fn set_status(db: &PgPool, id: Uuid, status: ResponseStatus) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE post_responses SET status = $2 WHERE id = $1",
        id,
        status as ResponseStatus,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Accepts a response and opens (or reopens) a conversation between the
/// post's author and the responder, returning its id.
pub async fn accept(db: &PgPool, id: Uuid) -> sqlx::Result<Uuid> {
    let mut tx = db.begin().await?;
    let row = sqlx::query!(
        "UPDATE post_responses r SET status = 'accepted'
         FROM posts p
         WHERE r.id = $1 AND p.id = r.post_id
         RETURNING r.post_id, r.responder_id, p.author_id",
        id,
    )
    .fetch_one(&mut *tx)
    .await?;
    let conversation =
        conversations::find_or_create_direct(&mut tx, row.author_id, row.responder_id, Some(row.post_id))
            .await?;
    tx.commit().await?;
    Ok(conversation)
}

/// Withdraws a response; only its responder can. False if it isn't theirs.
pub async fn delete(db: &PgPool, id: Uuid, responder: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM post_responses WHERE id = $1 AND responder_id = $2",
        id,
        responder,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}
