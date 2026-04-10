# Auth Service Architecture

## Purpose

Describe the implemented architecture for `auth_server` and keep boundaries
aligned with `CLEAN_ARCHITECTURE_GUIDELINES.md`.

Status snapshot: **February 28, 2026**.

## Architecture Overview

The service follows four clean-architecture layers:

- **Domain (`src/domain`)**: core session entity, domain errors, and ports.
- **Use Cases (`src/use_cases`)**: guest login, token verify, and logout
  orchestration.
- **Interface Adapters (`src/interface_adapters`)**: HTTP DTOs, handlers,
  routes, and adapter state.
- **Frameworks (`src/frameworks`)**: server bootstrap and Postgres wiring.

`src/main.rs` is a thin entrypoint and delegates startup to
`frameworks::server::run`. On startup failure, `main` exits with an explicit
non-zero exit code exposed by `frameworks::server::StartupFailure`.

## Directory Layout (Current)

```text
auth_server/
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md
├── migrations/
│   └── 0001_create_guest_profiles.sql
└── src/
    ├── main.rs
    ├── domain/
    │   ├── entities.rs
    │   ├── errors.rs
    │   ├── mod.rs
    │   └── ports.rs
    ├── use_cases/
    │   ├── guest_login.rs
    │   ├── logout.rs
    │   ├── mod.rs
    │   ├── test_support.rs
    │   └── verify_token.rs
    ├── interface_adapters/
    │   ├── handlers.rs
    │   ├── mod.rs
    │   ├── protocol.rs
    │   ├── routes.rs
    │   └── state.rs
    └── frameworks/
        ├── db.rs
        ├── mod.rs
        └── server.rs
```

## Layer Responsibilities

### Domain

- `entities.rs` defines canonical `Session` state.
- `errors.rs` defines `AuthError`.
- `ports.rs` defines `SessionStore` and `Clock`.

### Use Cases

- `guest_login.rs` validates identity inputs, creates sessions, and persists
  sessions through `SessionStore`.
- `verify_token.rs` resolves session identity from token and enforces expiry.
- `logout.rs` revokes session tokens through `SessionStore`.

### Interface Adapters

- `protocol.rs` defines wire request/response DTOs.
- `handlers.rs` maps HTTP requests to use cases and maps errors to HTTP
  responses.
- `routes.rs` binds HTTP routes.
- `state.rs` provides adapter implementations:
  `InMemorySessionStore`, `SystemClock`, and `PostgresGuestProfileStore`.

### Frameworks

- `frameworks/server.rs` loads env, initializes tracing, connects to Postgres,
  runs migrations, builds `AppState`, and starts Axum.
- `frameworks/db.rs` contains DB connection and migration helpers.

## Runtime Data Ownership

- Sessions are authoritative in in-memory `HashMap<String, Session>`.
- Guest profiles are best-effort persisted to Postgres (`guest_profiles`) for
  downstream service lookup.
- Session TTL is currently fixed to `3600` seconds in handlers.

## HTTP Flows (Current)

### `POST /auth/guest/init`

1. Handler validates `display_name` via `GuestLoginUseCase`.
2. Handler generates a new numeric `guest_id`.
3. Use case creates and stores a session.
4. Handler best-effort upserts `guest_profiles` in Postgres.
5. Response returns `guest_id`, `token`, and `expires_at`.

### `POST /auth/guest`

1. Handler receives existing `guest_id` identity payload.
2. Use case validates inputs and stores a new session token.
3. Handler best-effort upserts profile to Postgres.
4. Response returns `token` and `expires_at`.

### `POST /auth/verify-token`

1. Use case looks up token in session store.
2. Missing token returns `invalid session token` (`401`).
3. Expired token is cleaned up and returns `session expired` (`401`).
4. Valid token returns identity/session payload.

### `POST /auth/logout`

1. Use case removes the token from session store.
2. Response returns `{ "revoked": true|false }`.

### `GET /health`

1. Route returns liveness payload from adapter handler.
2. Response returns `{ "status": "ok" }` with `200`.

## Startup Failure Codes (Current)

- `1`: required startup config is missing (`DATABASE_URL` or
  `AUTH_SERVER_BIND_HOST`).
- `2`: startup config is invalid.
- `3`: database connection failed.
- `4`: startup migrations failed.
- `5`: listener bind failed.
- `6`: server runtime failed while serving requests.

## Dependency Rule Status

Dependency direction follows the repository-wide clean-architecture rule:
use-cases no longer accept adapter DTOs directly, and transport conversion
happens in interface adapters before calling the use-case layer.

## Web3 Extension Path

Not implemented yet. Planned additions:

- Nonce issuance endpoint.
- Wallet-signature verification endpoint.
- A nonce storage port in `domain/ports.rs` with an adapter implementation.
- Reuse existing session issuance and verification flow once identity is proven.
