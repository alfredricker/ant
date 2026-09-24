//! Database access. Every query goes through sqlx's checked macros
//! (`query!`, `query_as!`), which compare SQL, column types and nullability
//! against the live schema at compile time, so the structs here can't drift
//! from postgres without a build error.
pub mod users;
