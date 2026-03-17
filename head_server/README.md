# Head Service

```mermaid
flowchart TD
    F[Frameworks - server.rs] --> R[Interface Adapters - routes.rs]
    R --> H[Interface Adapters - handlers/guest.rs]
    H --> S[Interface Adapters - state.rs]
    H --> U[Use Cases - guest.rs - GuestSessionService]
    U --> P[Use Cases - guest.rs - AuthProvider]
    F --> C[Frameworks - auth_client.rs - AuthClient]
    C --> P
```

## Purpose

The head service is currently the HTTP entry point for guest identity/session
flows used by the game client. It proxies guest auth operations to
`auth_server`.

## Architecture Guidelines

Follow `CLEAN_ARCHITECTURE_GUIDELINES.md` and the service architecture docs
under `head_server/`.

## Current Scope (Implemented)

- Accept guest identity/session requests from clients.
- Orchestrate guest identity/session flows through use cases.
- Call `auth_server` to create guest identities and guest sessions via a port.
- Return head-level response DTOs suitable for client usage.

## HTTP API (Implemented)

### `POST /guest/init`

Creates a first-time guest identity and returns a session token.

Request:

```json
{
  "display_name": "Pilot_42"
}
```

Success response:

```json
{
  "guest_id": "123456789",
  "session_token": "uuid-token",
  "expires_at": "<unix_epoch_seconds>"
}
```

### `POST /guest/login`

Creates or refreshes a guest session for an existing guest ID.

Request:

```json
{
  "guest_id": "123456789",
  "display_name": "Pilot_42"
}
```

Success response:

```json
{
  "session_token": "uuid-token",
  "expires_at": "<unix_epoch_seconds>"
}
```

## Error Behavior

- Invalid `guest_id` format in `/guest/login` returns `400`.
- Upstream 4xx responses from `auth_server` are preserved where possible.
- Upstream transport/failure conditions return `502`.

## Runtime and Configuration

- Bind address: `127.0.0.1:3000`
- Auth base URL env var: `AUTH_SERVICE_URL`
- Default auth base URL: `http://localhost:3002`
- Tracing controls: `RUST_LOG`, optional `LOG_FORMAT=json`

## Dependencies

- `auth_server` for guest identity/session operations.

## Layer Notes

- `interface_adapters/` owns head HTTP DTOs, request validation, and HTTP error mapping.
- `use_cases/` owns guest session orchestration and the `AuthProvider` port.
- `frameworks/` owns the concrete reqwest auth client and runtime wiring.
- `domain/` is reserved for future head-specific business entities and invariants.

## Planned (Not Implemented Yet)

The following items are planned platform responsibilities, but are not
implemented in current routes:

- web app shell endpoints
- profile management endpoints
- party/friends/inventory endpoints
- matchmaking orchestration endpoints
