//! Types that cross the wire between the server and the browser. Everything
//! here compiles to wasm, so no sqlx, tokio or axum.
pub mod conversation;
pub mod health;
pub mod post;
pub mod response;
pub mod user;
