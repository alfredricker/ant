//! Types that cross the wire between the server and the browser. Everything
//! here compiles to wasm, so no sqlx, tokio or axum.
pub mod health;
pub mod user;
