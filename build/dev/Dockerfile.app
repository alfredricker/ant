# syntax=docker/dockerfile:1
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
# cargo-binstall pulls prebuilt release binaries instead of compiling from source.
RUN curl -fsSL "https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-$(uname -m)-unknown-linux-musl.tgz" \
        | tar -xz -C /usr/local/cargo/bin
RUN cargo binstall -y --locked cargo-watch trunk

# sqlx-cli has no prebuilt binary for these features, so compile it, but keep
# the registry and build dir in BuildKit caches so a rebuild reuses them.
# Pinned to match the sqlx crate in Cargo.toml (0.9 needs a newer rustc).
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/tmp/cargo-install-target \
    CARGO_TARGET_DIR=/tmp/cargo-install-target \
    cargo install sqlx-cli@0.8.6 --no-default-features --features rustls,postgres --locked

# Kept off the bind mount so host (macOS) and container (linux) artifacts never
# share a target dir; docker-compose backs this with a named volume.
ENV CARGO_TARGET_DIR=/build-target \
    CARGO_HOME=/usr/local/cargo \
    ANT_DIST_DIR=/app/dist \
    RUST_BACKTRACE=1

WORKDIR /app

# Warm the dependency cache so the first `up` after a rebuild is not a cold build.
# The stubs end up newer than the real sources mounted later, so cargo would
# treat our own crates as fresh; dropping their fingerprints keeps only the
# third-party deps cached.
COPY Cargo.toml Cargo.lock* build.rs ./
COPY web/Cargo.toml ./web/
COPY common/Cargo.toml ./common/
RUN mkdir -p src web/src common/src \
    && echo 'fn main() {}' > src/main.rs \
    && echo 'fn main() {}' > web/src/main.rs \
    && touch common/src/lib.rs \
    && cargo build 2>/dev/null || true \
    && cargo build -p ant-web --target wasm32-unknown-unknown 2>/dev/null || true \
    && rm -rf src web/src common/src \
    && rm -rf /build-target/*/.fingerprint/ant-* /build-target/*/*/.fingerprint/ant-*

COPY build/dev/app-entrypoint.sh /usr/local/bin/app-entrypoint.sh
RUN chmod +x /usr/local/bin/app-entrypoint.sh

EXPOSE 6353

ENTRYPOINT ["/usr/local/bin/app-entrypoint.sh"]
