// sqlx::migrate! embeds ./migrations at compile time, but cargo doesn't know
// to rebuild when a new .sql file appears. This tells it to.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}