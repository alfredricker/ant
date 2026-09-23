# ---------------------------------------------------------------------------
# ant :: dev application container (axum server + yew/wasm frontend)
# Dev-only: the source tree is bind-mounted and rebuilt on change, so this
# image carries the toolchain, not the binary.
# ---------------------------------------------------------------------------
FROM rust:1.90-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        postgresql-client \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Frontend target + bundler, backend live-reload, migrations CLI.
RUN rustup target add wasm32-unknown-unknown
RUN cargo install cargo-watch --locked \
    && cargo install trunk --locked \
    && cargo install sqlx-cli --no-default-features --features rustls,postgres --locked

# Kept off the bind mount so host (macOS) and container (linux) artifacts never
# share a target dir; docker-compose backs this with a named volume.
ENV CARGO_TARGET_DIR=/build-target \
    CARGO_HOME=/usr/local/cargo \
    ANT_DIST_DIR=/app/dist \
    RUST_BACKTRACE=1

WORKDIR /app

# Warm the dependency cache so the first `up` after a rebuild is not a cold build.
COPY Cargo.toml Cargo.lock* ./
COPY web/Cargo.toml ./web/
RUN mkdir -p src web/src \
    && echo 'fn main() {}' > src/main.rs \
    && echo 'fn main() {}' > web/src/main.rs \
    && cargo build 2>/dev/null || true \
    && cargo build -p ant-web --target wasm32-unknown-unknown 2>/dev/null || true \
    && rm -rf src web/src

COPY build/dev/app-entrypoint.sh /usr/local/bin/app-entrypoint.sh
RUN chmod +x /usr/local/bin/app-entrypoint.sh

EXPOSE 6353

ENTRYPOINT ["/usr/local/bin/app-entrypoint.sh"]
