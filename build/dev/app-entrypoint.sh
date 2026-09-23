#!/usr/bin/env bash
# Dev entrypoint: one container, two watchers.
#   trunk watch  -> rebuilds the yew bundle into /app/dist on frontend changes
#   cargo watch  -> rebuilds and restarts the axum server on backend changes
# The server serves /app/dist, so a frontend change is just a browser refresh.
set -euo pipefail

mkdir -p "${ANT_DIST_DIR:-/app/dist}"

echo "[entrypoint] building the frontend bundle..."
(cd web && trunk build) || echo "[entrypoint] initial frontend build failed; trunk watch will retry"

echo "[entrypoint] starting trunk watch..."
(cd web && exec trunk watch) &
TRUNK_PID=$!

# Don't leave the watcher orphaned when the container stops.
trap 'kill "$TRUNK_PID" 2>/dev/null || true' EXIT INT TERM

echo "[entrypoint] starting the server (cargo watch)..."
exec cargo watch -q -w src -w Cargo.toml -w migrations -x run
