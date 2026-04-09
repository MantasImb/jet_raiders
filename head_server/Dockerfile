# syntax=docker/dockerfile:1

# Build the head binary in a pinned Rust toolchain image.
FROM rust:1.93-bookworm AS builder

WORKDIR /app

# Copy the service crate and shared config from the monorepo root context.
COPY head_server/Cargo.toml head_server/Cargo.lock ./head_server/
COPY head_server/src ./head_server/src
COPY config ./config

WORKDIR /app/head_server
RUN cargo build --release --locked

# Run with a slim Debian runtime image and a non-root user.
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --uid 10001 --shell /usr/sbin/nologin appuser

WORKDIR /app

COPY --from=builder /app/head_server/target/release/head_server /usr/local/bin/head_server
COPY --from=builder /app/config/regions.toml /app/config/regions.toml
COPY --from=builder /app/config/backend_ports.toml /app/config/backend_ports.toml

EXPOSE 3000

USER appuser

ENTRYPOINT ["/usr/local/bin/head_server"]
