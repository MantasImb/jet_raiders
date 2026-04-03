# syntax=docker/dockerfile:1

# Build the auth binary in a pinned Rust toolchain image.
FROM rust:1.93-bookworm AS builder

WORKDIR /app

# Copy dependency manifests first to improve layer cache reuse.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release

# Run with a slim Debian runtime image and a non-root user.
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --uid 10001 --shell /usr/sbin/nologin appuser

WORKDIR /app

COPY --from=builder /app/target/release/auth_server /usr/local/bin/auth_server

EXPOSE 3002

USER appuser

ENTRYPOINT ["/usr/local/bin/auth_server"]
