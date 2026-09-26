# syntax=docker/dockerfile:1
# ---------------------------------------------------------------------------
# ant :: dev application container (Dioxus fullstack: axum server + wasm UI)
# Dev-only: the source tree is bind-mounted and rebuilt on change by `dx serve`,
# so this image carries the toolchain, not the binary.
# ---------------------------------------------------------------------------
# trixie, not bookworm: the prebuilt dx below links against glibc 2.39, and
# bookworm ships 2.36.
FROM rust:1.90-trixie

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        postgresql-client \
        curl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

# The Dioxus CLI builds the server and the wasm bundle, serves them and
# hot-reloads. Pinned to the dioxus crate in Cargo.toml; prebuilt release,
# checked against its published sha256.
ARG DX_VERSION=0.7.10
RUN set -eux; \
    asset="dx-$(uname -m)-unknown-linux-gnu"; \
    base="https://github.com/DioxusLabs/dioxus/releases/download/v${DX_VERSION}"; \
    cd /tmp; \
    curl -fsSLO "$base/$asset.tar.gz"; \
    curl -fsSLO "$base/$asset.sha256"; \
    sha256sum -c --ignore-missing "$asset.sha256"; \
    tar -xzf "$asset.tar.gz" -C /usr/local/cargo/bin dx; \
    rm -f "$asset.tar.gz" "$asset.sha256"; \
    dx --version

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
    RUST_BACKTRACE=1

WORKDIR /app

# Warm the dependency cache so the first `up` after a rebuild is not a cold build.
# Goes through `dx build` rather than cargo so the deps land in the same
# profiles (server-dev, wasm-dev) that `dx serve` uses, and dx fetches its
# wasm-bindgen into /root/.local/share/.dx now rather than on first start.
# The stub is a real (empty) Dioxus app: an empty main gives wasm-bindgen
# nothing to bind and fails the client half. It ends up newer than the real
# sources mounted later, so cargo would treat our own crate as fresh; dropping
# its fingerprints keeps only the third-party deps cached.
COPY Cargo.toml Cargo.lock* build.rs ./
RUN mkdir -p src migrations \
    && echo 'use dioxus::prelude::*; fn app() -> Element { rsx! {} } fn main() { dioxus::launch(app) }' > src/main.rs \
    && touch src/lib.rs \
    && (dx build --web 2>/dev/null || true) \
    && rm -rf src migrations \
    && rm -rf /build-target/*/.fingerprint/ant-* /build-target/*/*/.fingerprint/ant-*

COPY build/dev/app-entrypoint.sh /usr/local/bin/app-entrypoint.sh
RUN chmod +x /usr/local/bin/app-entrypoint.sh

EXPOSE 6353

ENTRYPOINT ["/usr/local/bin/app-entrypoint.sh"]
