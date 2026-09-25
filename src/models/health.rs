use serde::{Deserialize, Serialize};

/// Liveness plus a real round trip to postgres: served at `/api/health` for
/// the container healthcheck and by the `status` server function for the UI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Health {
    pub status: String,
    pub database: String,
    /// Version of the pgvector extension, once the database is reachable.
    pub pgvector: Option<String>,
}

impl Health {
    pub fn is_ok(&self) -> bool {
        self.status == "ok"
    }
}
