# syntax=docker/dockerfile:1

# Build the game binary in a pinned Rust toolchain image.
FROM rust:1.93-bookworm AS builder

WORKDIR /app

# Copy the service crate from the monorepo root context.
COPY game_server/Cargo.toml game_server/Cargo.lock ./game_server/
COPY game_server/src ./game_server/src

WORKDIR /app/game_server
RUN cargo build --release --locked

# Run with a slim Debian runtime image and a non-root user.
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --uid 10001 --shell /usr/sbin/nologin appuser

WORKDIR /app

COPY --from=builder /app/game_server/target/release/game_server /usr/local/bin/game_server

EXPOSE 3001

USER appuser

ENTRYPOINT ["/usr/local/bin/game_server"]
