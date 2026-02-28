# Auth Service

## Purpose

The auth service is the source of truth for session identity in the current
backend. Right now it implements a guest-only auth flow and provides token
verification for downstream services.

## Current Scope

Implemented today:

- Create first-time guest identities.
- Issue short-lived guest session tokens.
- Verify guest session tokens.
- Revoke guest session tokens.
- Persist guest profile data to Postgres.

Not implemented yet:

- Wallet nonce/signature login (`/auth/nonce`, `/auth/verify`).
- Durable session storage (sessions are currently in-memory).

## Architecture

Follow repository and service architecture rules:

- `CLEAN_ARCHITECTURE_GUIDELINES.md`
- `auth_server/ARCHITECTURE.md`

Code is split into clean architecture layers:

- `domain/`: entities, errors, and ports.
- `use_cases/`: guest login, token verify, logout.
- `interface_adapters/`: HTTP DTOs, handlers, routes, app state.
- `frameworks/`: server bootstrap and database wiring.

## HTTP API

Base URL (local): `http://127.0.0.1:3002`

### `POST /auth/guest/init`

Creates a new `guest_id`, issues a token, and persists the guest profile.

Request:

```json
{
  "display_name": "Pilot_42",
  "metadata": { "region": "eu" }
}
```

Success response:

```json
{
  "guest_id": 123456789,
  "token": "uuid-token",
  "expires_at": 1700003600
}
```

Validation:

- `display_name` length must be `3..=32` characters.
- Allowed chars: ASCII letters, digits, space, `_`, `-`.
- No leading/trailing whitespace.
- `guest_id` is exposed as an unsigned 64-bit integer (`u64`).

### `POST /auth/guest`

Issues a token for an existing guest identity.

Request:

```json
{
  "guest_id": 123456789,
  "display_name": "Pilot_42",
  "metadata": { "region": "eu" }
}
```

Success response:

```json
{
  "token": "uuid-token",
  "expires_at": 1700003600
}
```

Validation:

- `guest_id` must be non-zero.
- `display_name` uses the same validation rules as `/auth/guest/init`.
- `guest_id` is exposed as an unsigned 64-bit integer (`u64`).

### `POST /auth/verify-token`

Validates a token and returns the identity/session payload.

Request:

```json
{
  "token": "uuid-token"
}
```

Success response:

```json
{
  "user_id": 123456789,
  "display_name": "Pilot_42",
  "metadata": { "region": "eu" },
  "session_id": "uuid-session-id",
  "expires_at": 1700003600
}
```

Failure cases:

- `401 invalid session token`
- `401 session expired`

### `POST /auth/logout`

Revokes a token.

Request:

```json
{
  "token": "uuid-token"
}
```

Success response:

```json
{
  "revoked": true
}
```

`revoked` is `false` when the token is unknown.

## Error Envelope

Domain and validation errors are returned as:

```json
{
  "message": "invalid display_name"
}
```

## Runtime and Configuration

Required environment variable:

- `DATABASE_URL`: Postgres connection string.

Optional environment variables:

- `RUST_LOG`: tracing filter (defaults to `info`).
- `LOG_FORMAT=json`: enables JSON logs.

Server bind:

- `0.0.0.0:3002`

Session details:

- TTL: `3600` seconds (1 hour).
- Session storage: in-memory `HashMap`.

## Database

On startup, the service runs migrations from `auth_server/migrations`.

Current schema stores guest profile snapshots:

- `guest_profiles(guest_id TEXT PRIMARY KEY, display_name TEXT, metadata TEXT)`

Type note:

- API canonical type is numeric `u64`.
- `guest_profiles.guest_id` is stored as `TEXT` using the decimal string form
  of that same numeric ID.

Guest profile persistence is best-effort and does not block token issuance.

## Testing

Route and use-case tests live in the service source files. Run:

```bash
cargo test -p auth_server
```
