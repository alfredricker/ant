# Agent guidelines

## Containers

- Don't build, rebuild or restart the Docker containers after making changes
  (`./build.sh`, `./start.sh`, `docker compose build` / `up`). The maintainer
  handles container builds: they're slow, and repeated rebuilds eat disk space.
- Verify changes on the host instead: `cargo build`, `cargo clippy`,
  `cargo test`, and for the frontend
  `cargo clippy -p ant-web --target wasm32-unknown-unknown`.
- When a change needs a rebuild or restart to take effect (Dockerfile, compose
  file, entrypoint, env files), say so and leave it to the maintainer.
