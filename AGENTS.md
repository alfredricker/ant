# Agent guidelines

## Containers

- Don't build, rebuild or restart the Docker containers after making changes
  (`./build.sh`, `./start.sh`, `docker compose build` / `up`). The maintainer
  handles container builds: they're slow, and repeated rebuilds eat disk space.
- Verify changes on the host instead: `cargo build`, `cargo clippy`,
  `cargo test` (server + shared code), and for the wasm half
  `cargo clippy --no-default-features --features web --target wasm32-unknown-unknown`.
  With no database reachable, sqlx checks queries against the committed
  `.sqlx/`; after adding or changing a query, regenerate it with the stack up:
  `docker exec ant_dev_app cargo sqlx prepare`.

## Code layout

- One crate, built twice by `dx`: `server` feature -> axum binary, `web` ->
  wasm. `src/models` (wire types), `src/api` (server functions) and `src/ui`
  compile into both; `src/server` is server-only and must stay behind
  `#[cfg(feature = "server")]`.
- When a change needs a rebuild or restart to take effect (Dockerfile, compose
  file, entrypoint, env files), say so and leave it to the maintainer.
