//! Pictures attached to posts, stored already re-encoded.

use sqlx::{PgPool, types::Uuid};

use crate::server::images::Encoded;

pub struct PostImage {
    pub content_type: String,
    pub data: Vec<u8>,
}

pub async fn create(db: &PgPool, uploader: Uuid, image: &Encoded) -> sqlx::Result<Uuid> {
    sqlx::query_scalar!(
        "INSERT INTO post_images (uploader_id, content_type, data) VALUES ($1, $2, $3) RETURNING id",
        uploader,
        image.content_type,
        image.data,
    )
    .fetch_one(db)
    .await
}

/// How many pictures someone has uploaded in the past hour.
pub async fn uploaded_last_hour(db: &PgPool, uploader: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM post_images
           WHERE uploader_id = $1 AND created_at > now() - interval '1 hour'"#,
        uploader,
    )
    .fetch_one(db)
    .await
}

pub async fn find(db: &PgPool, id: Uuid) -> sqlx::Result<Option<PostImage>> {
    sqlx::query_as!(
        PostImage,
        "SELECT content_type, data FROM post_images WHERE id = $1",
        id,
    )
    .fetch_optional(db)
    .await
}
