use chrono::{DateTime, Utc};
use sqlx::{PgPool, types::Uuid};

use crate::models::{
    post::{Post, PostInput, PostKind, PostStatus},
    user::PublicUser,
};

/// Which posts `list` returns. The default is the browse page: every open
/// post, newest first.
#[derive(Debug, Default)]
pub struct PostFilter {
    pub kind: Option<PostKind>,
    /// Full-text search over title and description.
    pub text: Option<String>,
    /// One author's posts, closed ones included.
    pub author: Option<Uuid>,
    /// Only posts created before this; the last post of one page is the
    /// cursor for the next.
    pub before: Option<DateTime<Utc>>,
}

struct PostRow {
    id: Uuid,
    author_id: Uuid,
    author_username: Option<String>,
    kind: PostKind,
    status: PostStatus,
    title: String,
    body: String,
    looking_for: Vec<String>,
    funded: bool,
    image_url: Option<String>,
    like_count: i64,
    comment_count: i64,
    liked: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PostRow> for Post {
    fn from(row: PostRow) -> Self {
        Post {
            id: row.id,
            author: PublicUser {
                id: row.author_id,
                username: row.author_username,
            },
            kind: row.kind,
            status: row.status,
            title: row.title,
            body: row.body,
            looking_for: row.looking_for,
            funded: row.funded,
            image_url: row.image_url,
            like_count: row.like_count,
            comment_count: row.comment_count,
            liked: row.liked,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// The one query that reads posts, so `find` and `list` can't disagree on
/// what a post looks like. `viewer` decides `liked`. Closed posts only show
/// up when asked for by id or by author.
async fn select(
    db: &PgPool,
    viewer: Option<Uuid>,
    id: Option<Uuid>,
    filter: &PostFilter,
    limit: i64,
) -> sqlx::Result<Vec<Post>> {
    let rows = sqlx::query_as!(
        PostRow,
        r#"SELECT p.id, p.author_id, u.username AS author_username,
                  p.kind AS "kind: PostKind", p.status AS "status: PostStatus",
                  p.title, p.body, p.looking_for, p.funded, p.image_url,
                  (SELECT count(*) FROM post_likes l WHERE l.post_id = p.id) AS "like_count!",
                  (SELECT count(*) FROM post_comments c WHERE c.post_id = p.id) AS "comment_count!",
                  EXISTS (SELECT 1 FROM post_likes l WHERE l.post_id = p.id AND l.user_id = $1) AS "liked!",
                  p.created_at, p.updated_at
           FROM posts p JOIN users u ON u.id = p.author_id
           WHERE ($2::uuid IS NULL OR p.id = $2)
             AND ($3::uuid IS NULL OR p.author_id = $3)
             AND (p.status = 'open' OR $2 IS NOT NULL OR $3 IS NOT NULL)
             AND ($4::post_kind IS NULL OR p.kind = $4)
             AND ($5::text IS NULL OR p.search @@ websearch_to_tsquery('english', $5))
             AND ($6::timestamptz IS NULL OR p.created_at < $6)
           ORDER BY p.created_at DESC
           LIMIT $7"#,
        viewer,
        id,
        filter.author,
        filter.kind as Option<PostKind>,
        filter.text,
        filter.before,
        limit,
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(Post::from).collect())
}

pub async fn find(db: &PgPool, viewer: Option<Uuid>, id: Uuid) -> sqlx::Result<Option<Post>> {
    Ok(select(db, viewer, Some(id), &PostFilter::default(), 1)
        .await?
        .pop())
}

pub async fn list(
    db: &PgPool,
    viewer: Option<Uuid>,
    filter: &PostFilter,
    limit: i64,
) -> sqlx::Result<Vec<Post>> {
    select(db, viewer, None, filter, limit).await
}

/// Who wrote a post and whether it's open, for permission checks.
pub async fn author_and_status(db: &PgPool, id: Uuid) -> sqlx::Result<Option<(Uuid, PostStatus)>> {
    let row = sqlx::query!(
        r#"SELECT author_id, status AS "status: PostStatus" FROM posts WHERE id = $1"#,
        id
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| (r.author_id, r.status)))
}

pub async fn create(db: &PgPool, author: Uuid, input: &PostInput) -> sqlx::Result<Uuid> {
    sqlx::query_scalar!(
        "INSERT INTO posts (author_id, kind, title, body, looking_for, funded, image_url)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id",
        author,
        input.kind as PostKind,
        input.title,
        input.body,
        &input.looking_for,
        input.funded,
        input.image_url,
    )
    .fetch_one(db)
    .await
}

// Writes below match on the author too, so they're no-ops (false) on someone
// else's post; callers turn that into a 404.

pub async fn update(db: &PgPool, id: Uuid, author: Uuid, input: &PostInput) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        "UPDATE posts SET kind = $3, title = $4, body = $5, looking_for = $6, funded = $7, image_url = $8
         WHERE id = $1 AND author_id = $2",
        id,
        author,
        input.kind as PostKind,
        input.title,
        input.body,
        &input.looking_for,
        input.funded,
        input.image_url,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn set_status(db: &PgPool, id: Uuid, author: Uuid, status: PostStatus) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        "UPDATE posts SET status = $3 WHERE id = $1 AND author_id = $2",
        id,
        author,
        status as PostStatus,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn delete(db: &PgPool, id: Uuid, author: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!("DELETE FROM posts WHERE id = $1 AND author_id = $2", id, author)
        .execute(db)
        .await?;
    Ok(result.rows_affected() == 1)
}

/// Idempotent: liking twice is still one like.
pub async fn like(db: &PgPool, post: Uuid, user: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO post_likes (post_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        post,
        user,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn unlike(db: &PgPool, post: Uuid, user: Uuid) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM post_likes WHERE post_id = $1 AND user_id = $2", post, user)
        .execute(db)
        .await?;
    Ok(())
}
