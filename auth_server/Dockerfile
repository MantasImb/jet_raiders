# syntax=docker/dockerfile:1

# Build the auth binary in a pinned Rust toolchain image.
FROM rust:1.93-bookworm AS builder

WORKDIR /app

# Copy dependency manifests first to improve layer cache reuse.
COPY auth_server/Cargo.toml auth_server/Cargo.lock ./auth_server/
COPY auth_server/src ./auth_server/src
COPY auth_server/migrations ./auth_server/migrations

WORKDIR /app/auth_server
RUN cargo build --release --locked

# Run with a slim Debian runtime image and a non-root user.
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --uid 10001 --shell /usr/sbin/nologin appuser

WORKDIR /app

COPY --from=builder /app/auth_server/target/release/auth_server /usr/local/bin/auth_server

EXPOSE 3002

USER appuser

ENTRYPOINT ["/usr/local/bin/auth_server"]
