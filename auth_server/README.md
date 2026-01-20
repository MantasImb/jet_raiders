# Auth Service

## Purpose

The auth service is the source of truth for player identity. It validates login
proofs (wallet signatures or other identity proofs), issues short-lived session
tokens, and exposes verification endpoints for other services.

## Responsibilities

- Issue nonces for login challenges.
- Validate signed login payloads.
- Create and rotate short-lived session tokens.
- Provide token verification for matchmaking and game servers.
- Support logout and token revocation.
- Enforce rate limits and audit logging for auth events.

## Client Access Pattern

The auth service is intended to be consumed by the head service. The game
client should normally call the head service for login and session workflows,
while the head service calls the auth service to issue or verify tokens. Direct
client-to-auth access can remain optional for dedicated launcher flows, but the
default posture is to keep auth behind the head service for consistency and to
avoid exposing identity endpoints directly to the game client.

## Current Axum Server Functionality to Extract

- Replace the current random player ID assignment during WebSocket bootstrap
  with an auth-derived identity binding so player IDs map to verified sessions.
- Own the session validation that the server currently leaves as a TODO in the
  WebSocket handshake path.
- Issue guest session tokens as a bridge from the current guest flow so game
  servers can transition from `guest_id` usage to verified session identities.

## External Interfaces

### HTTP API

- `POST /auth/nonce`
  - Returns a one-time nonce and expiry.
- `POST /auth/verify`
  - Validates signature + nonce.
  - Returns a session token (cookie or JWT).
- `POST /auth/logout`
  - Invalidates the current session token.
- `POST /auth/verify-token`
  - Validates a token and returns the user identity payload.

## Data Contracts

### Session Token (JWT or opaque token)

- `sub`: user ID or wallet address.
- `iat`: issued-at timestamp.
- `exp`: expiration timestamp.
- `aud`: intended audience (matchmaking, game-server).
- `jti`: unique token ID for revocation.

### Auth Verification Response

- `user_id`: canonical user ID.
- `wallet_address`: optional wallet address.
- `session_id`: server-side session ID.
- `expires_at`: token expiry timestamp.

## Security Considerations

- Enforce nonce expiry and one-time usage.
- Require TLS for all auth endpoints.
- Keep tokens short-lived and rotate on refresh.
- Store only token hashes for revocation checks.
- Include correlation IDs for tracing across services.

## Dependencies

- Identity verification library (wallet signature validation or OAuth).
- Secure random generator for nonces.
- Data store for nonce/session persistence.

## Observability

- Log auth events with correlation IDs.
- Track invalid signature attempts and throttle.
- Emit metrics for login success/failure rates.
