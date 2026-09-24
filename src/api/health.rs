use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct Health {
    status: &'static str,
    database: &'static str,
    /// Version of the pgvector extension, once the database is reachable.
    pgvector: Option<String>,
}

/// Liveness plus a real round trip to postgres, so the dev stack can tell the
/// difference between "server up" and "server up, database missing".
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<Health>) {
    let pgvector = sqlx::query_scalar::<_, String>(
        "SELECT extversion FROM pg_extension WHERE extname = 'vector'",
    )
    .fetch_optional(&state.db)
    .await;

    match pgvector {
        Ok(version) => (
            StatusCode::OK,
            Json(Health {
                status: "ok",
                database: "connected",
                pgvector: version,
            }),
        ),
        Err(err) => {
            tracing::warn!("health check could not reach the database: {err}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(Health {
                    status: "degraded",
                    database: "unavailable",
                    pgvector: None,
                }),
            )
        }
    }
}
