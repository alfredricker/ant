//! ant — browser app. Mounted into <body> by Trunk's generated bootstrap.

use serde::Deserialize;
mod home;

fn main() {
    yew::Renderer::<home::Home>::new().render();
}

/// Mirrors the `/api/health` payload served by the axum side.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Health {
    pub status: String,
    pub database: String,
    pub pgvector: Option<String>,
}
