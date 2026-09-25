use axum::{Json, extract::State, http::StatusCode};
use sqlx::PgPool;

use crate::{models::health::Health, server::state::AppState};

/// Liveness plus a real round trip to postgres, so the dev stack can tell the
/// difference between "server up" and "server up, database missing".
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<Health>) {
    let health = check(&state.db).await;
    let status = if health.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(health))
}

pub async fn check(db: &PgPool) -> Health {
    let pgvector = sqlx::query_scalar::<_, String>(
        "SELECT extversion FROM pg_extension WHERE extname = 'vector'",
    )
    .fetch_optional(db)
    .await;

    match pgvector {
        Ok(version) => Health {
            status: "ok".into(),
            database: "connected".into(),
            pgvector: version,
        },
        Err(err) => {
            tracing::warn!("health check could not reach the database: {err}");
            Health {
                status: "degraded".into(),
                database: "unavailable".into(),
                pgvector: None,
            }
        }
    }
}
