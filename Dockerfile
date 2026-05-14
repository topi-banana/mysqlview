# syntax=docker/dockerfile:1.7

# Multi-stage build that produces a fully static mysqlview-backend binary
# wrapped in a `FROM scratch` image. End image holds nothing but the binary
# (~9 MB) and listens on port 3000.
#
# Build with:
#   docker build -t mysqlview .
# Run with (host MySQL on the default Docker bridge):
#   docker run --rm -it \
#     -p 127.0.0.1:3000:3000 \
#     -e DATABASE_URI=mysql://root:pass@host.docker.internal:3306 \
#     mysqlview
#
# WARNING: keep the host port mapped to 127.0.0.1 — the backend has no
# authentication and is intended for local development only.

# ---- Stage 1: build the frontend with trunk ----
FROM rust:1.85-slim AS frontend-builder

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl pkg-config \
 && rm -rf /var/lib/apt/lists/* \
 && rustup target add wasm32-unknown-unknown \
 && cargo install --locked trunk

WORKDIR /workspace
COPY . .

RUN cd frontend && trunk build --release

# ---- Stage 2: build the backend as a static musl binary ----
FROM rust:1.85-slim AS backend-builder

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates musl-tools cmake \
 && rm -rf /var/lib/apt/lists/* \
 && rustup target add x86_64-unknown-linux-musl

WORKDIR /workspace
COPY . .
COPY --from=frontend-builder /workspace/frontend/dist /workspace/frontend/dist

RUN cargo build --release \
    --target x86_64-unknown-linux-musl \
    --features embedded-frontend \
    -p mysqlview-backend

# ---- Stage 3: scratch image with just the static binary ----
FROM scratch

# Container default: bind 0.0.0.0 so the host can reach the listener via
# `-p`. The binary still logs a warning about non-loopback binds. Keep
# `-p 127.0.0.1:3000:3000` on the host side to stay local-only.
ENV MYSQLVIEW_BIND=0.0.0.0

COPY --from=backend-builder \
    /workspace/target/x86_64-unknown-linux-musl/release/mysqlview-backend \
    /mysqlview

EXPOSE 3000

# Probe /api/health from inside the container. The binary doubles as its
# own HTTP client when invoked with `--healthcheck`, so no extra tools
# (curl, wget) are needed in the scratch image. /api/health pings the
# MySQL pool, so the container is "healthy" only once the DB is reachable.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/mysqlview", "--healthcheck"]

ENTRYPOINT ["/mysqlview"]
