#!/usr/bin/env bash
# Dev entrypoint: `dx serve` builds the server and the wasm bundle, runs the
# server, and on a change hot-reloads rsx/CSS or rebuilds and restarts.
set -euo pipefail

# sqlx::query! checks SQL against the live schema at compile time (this
# container sets DATABASE_URL), so the migrations must be applied before the
# first build; the app's own startup migrate! can't help, since the binary
# won't compile against an empty database. dx doesn't re-run this on change:
# after adding a migration, `docker exec ant_dev_app sqlx migrate run`.
echo "[entrypoint] applying migrations..."
sqlx migrate run

# Listens where the app always has; nginx proxies to it, websockets (hot
# reload) included.
echo "[entrypoint] starting dx serve..."
exec dx serve --web --addr 0.0.0.0 --port 6353 --interactive false --open false
