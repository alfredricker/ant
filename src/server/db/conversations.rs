use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool, types::Uuid};

use crate::models::{
    conversation::{ConversationSummary, Message},
    user::PublicUser,
};

/// Finds the two-person conversation between `a` and `b`, creating it if
/// needed. Takes a connection so it can run inside a caller's transaction.
/// `post` is recorded only if the conversation doesn't have one already.
pub async fn find_or_create_direct(
    conn: &mut PgConnection,
    a: Uuid,
    b: Uuid,
    post: Option<Uuid>,
) -> sqlx::Result<Uuid> {
    let key = if a < b { format!("{a}:{b}") } else { format!("{b}:{a}") };
    let id = sqlx::query_scalar!(
        "INSERT INTO conversations (direct_key, post_id) VALUES ($1, $2)
         ON CONFLICT (direct_key) DO UPDATE SET post_id = COALESCE(conversations.post_id, EXCLUDED.post_id)
         RETURNING id",
        key,
        post,
    )
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query!(
        "INSERT INTO conversation_members (conversation_id, user_id) VALUES ($1, $2), ($1, $3)
         ON CONFLICT DO NOTHING",
        id,
        a,
        b,
    )
    .execute(&mut *conn)
    .await?;
    Ok(id)
}

struct SummaryRow {
    id: Uuid,
    post_id: Option<Uuid>,
    last_message_at: DateTime<Utc>,
    other_id: Uuid,
    other_username: Option<String>,
    last_id: Option<Uuid>,
    last_sender_id: Option<Uuid>,
    last_body: Option<String>,
    last_created_at: Option<DateTime<Utc>>,
    unread_count: i64,
}

impl From<SummaryRow> for ConversationSummary {
    fn from(row: SummaryRow) -> Self {
        let last_message = match (row.last_id, row.last_sender_id, row.last_body, row.last_created_at) {
            (Some(id), Some(sender_id), Some(body), Some(created_at)) => Some(Message {
                id,
                conversation_id: row.id,
                sender_id,
                body,
                created_at,
            }),
            _ => None,
        };
        ConversationSummary {
            id: row.id,
            with: PublicUser {
                id: row.other_id,
                username: row.other_username,
            },
            post_id: row.post_id,
            last_message,
            last_message_at: row.last_message_at,
            unread_count: row.unread_count,
        }
    }
}

/// `user`'s conversations, most recently active first; just the one if `id`
/// is given (and `user` is in it).
async fn summaries(db: &PgPool, user: Uuid, id: Option<Uuid>) -> sqlx::Result<Vec<ConversationSummary>> {
    let rows = sqlx::query_as!(
        SummaryRow,
        r#"SELECT c.id, c.post_id, c.last_message_at,
                  o.user_id AS other_id, ou.username AS other_username,
                  lm.id AS "last_id?", lm.sender_id AS "last_sender_id?",
                  lm.body AS "last_body?", lm.created_at AS "last_created_at?",
                  (SELECT count(*) FROM messages m
                   WHERE m.conversation_id = c.id AND m.sender_id <> me.user_id
                     AND (me.last_read_at IS NULL OR m.created_at > me.last_read_at)) AS "unread_count!"
           FROM conversation_members me
           JOIN conversations c ON c.id = me.conversation_id
           JOIN conversation_members o ON o.conversation_id = c.id AND o.user_id <> me.user_id
           JOIN users ou ON ou.id = o.user_id
           LEFT JOIN LATERAL (
               SELECT id, sender_id, body, created_at FROM messages
               WHERE conversation_id = c.id ORDER BY created_at DESC LIMIT 1
           ) lm ON true
           WHERE me.user_id = $1 AND ($2::uuid IS NULL OR c.id = $2)
           ORDER BY c.last_message_at DESC
           LIMIT 200"#,
        user,
        id,
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(ConversationSummary::from).collect())
}

pub async fn list(db: &PgPool, user: Uuid) -> sqlx::Result<Vec<ConversationSummary>> {
    summaries(db, user, None).await
}

/// None if it doesn't exist or `user` isn't in it.
pub async fn find(db: &PgPool, user: Uuid, id: Uuid) -> sqlx::Result<Option<ConversationSummary>> {
    Ok(summaries(db, user, Some(id)).await?.pop())
}

pub async fn is_member(db: &PgPool, conversation: Uuid, user: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM conversation_members WHERE conversation_id = $1 AND user_id = $2
           ) AS "exists!""#,
        conversation,
        user,
    )
    .fetch_one(db)
    .await
}

/// Newest first, `limit` at a time; pass the oldest one's `created_at` as
/// `before` for the page above it. Callers check membership first.
pub async fn messages(
    db: &PgPool,
    conversation: Uuid,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<Message>> {
    sqlx::query_as!(
        Message,
        "SELECT id, conversation_id, sender_id, body, created_at FROM messages
         WHERE conversation_id = $1 AND ($2::timestamptz IS NULL OR created_at < $2)
         ORDER BY created_at DESC
         LIMIT $3",
        conversation,
        before,
        limit,
    )
    .fetch_all(db)
    .await
}

/// Sends a message, bumping the conversation in both inboxes and marking it
/// read for the sender. Callers check membership first.
pub async fn send(db: &PgPool, conversation: Uuid, sender: Uuid, body: &str) -> sqlx::Result<Message> {
    let mut tx = db.begin().await?;
    let message = sqlx::query_as!(
        Message,
        "INSERT INTO messages (conversation_id, sender_id, body) VALUES ($1, $2, $3)
         RETURNING id, conversation_id, sender_id, body, created_at",
        conversation,
        sender,
        body,
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE conversations SET last_message_at = $2 WHERE id = $1",
        conversation,
        message.created_at,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE conversation_members SET last_read_at = $3 WHERE conversation_id = $1 AND user_id = $2",
        conversation,
        sender,
        message.created_at,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(message)
}

pub async fn mark_read(db: &PgPool, conversation: Uuid, user: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE conversation_members SET last_read_at = now() WHERE conversation_id = $1 AND user_id = $2",
        conversation,
        user,
    )
    .execute(db)
    .await?;
    Ok(())
}
