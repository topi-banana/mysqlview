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
# Multi-arch:
#   - On Apple Silicon / arm64 Linux hosts, the build runs natively and
#     produces an aarch64 binary.
#   - On amd64 hosts (GitHub Actions ubuntu-latest, etc.) it produces an
#     x86_64 binary.
#   - For cross-arch builds use buildx:
#       docker buildx build --platform linux/amd64,linux/arm64 -t mysqlview .
#
# WARNING: keep the host port mapped to 127.0.0.1 — the backend has no
# authentication and is intended for local development only.

# ---- Stage 1: build the frontend with trunk ----
# Frontend output is WebAssembly + static assets, so it's arch-independent.
# Pinning to BUILDPLATFORM means a multi-arch build only compiles the
# frontend once on the native build host (no qemu).
FROM --platform=$BUILDPLATFORM rust:slim AS frontend-builder

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl pkg-config \
 && rm -rf /var/lib/apt/lists/* \
 && rustup target add wasm32-unknown-unknown \
 && cargo install --locked trunk

WORKDIR /workspace
COPY . .

RUN cd frontend && trunk build --release

# ---- Stage 2: build the backend as a static musl binary ----
# Runs on TARGETPLATFORM (qemu when cross-building) so musl-tools and the
# Rust target match. Debian's musl-gcc is arch-specific: on amd64 it wraps
# x86_64-linux-gnu-gcc, on arm64 it wraps aarch64-linux-gnu-gcc. As long as
# the Rust musl target matches the host arch, cc-rs C deps (ring,
# aws-lc-rs, …) compile cleanly.
FROM rust:slim AS backend-builder

ARG TARGETARCH

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates musl-tools \
 && rm -rf /var/lib/apt/lists/* \
 && case "$TARGETARCH" in \
      amd64) RUST_TARGET=x86_64-unknown-linux-musl ;; \
      arm64) RUST_TARGET=aarch64-unknown-linux-musl ;; \
      *) echo "Unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac \
 && rustup target add "$RUST_TARGET" \
 && echo "$RUST_TARGET" > /tmp/rust_target

WORKDIR /workspace
COPY . .
COPY --from=frontend-builder /workspace/dist /workspace/dist

RUN RUST_TARGET=$(cat /tmp/rust_target) \
 && cargo build --release \
    --target "$RUST_TARGET" \
    --features embedded-frontend \
    -p mysqlview-backend \
 && cp "target/$RUST_TARGET/release/mysqlview-backend" /tmp/mysqlview

# ---- Stage 3: scratch image with just the static binary ----
FROM scratch

# Container default: bind 0.0.0.0 so the host can reach the listener via
# `-p`. The binary still logs a warning about non-loopback binds. Keep
# `-p 127.0.0.1:3000:3000` on the host side to stay local-only.
ENV MYSQLVIEW_BIND=0.0.0.0

COPY --from=backend-builder /tmp/mysqlview /mysqlview

EXPOSE 3000

# Probe /api/health from inside the container. The binary doubles as its
# own HTTP client when invoked with `--healthcheck`, so no extra tools
# (curl, wget) are needed in the scratch image. /api/health pings the
# MySQL pool, so the container is "healthy" only once the DB is reachable.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/mysqlview", "--healthcheck"]

ENTRYPOINT ["/mysqlview"]
