# Auth Service Development Plan

## Purpose

Create a barebones auth service that supports the current guest flow and
establishes a clean path to Web3 login (Solana/Web3Auth) in a future iteration.
The guest flow already exists in `game_client` and `game_server`, so the service
should focus on session issuance and verification without forcing client
rewrites.

## Current State

- Guests join using a `guest_id` and `display_name` sent during the game join
  handshake.
- The game services do not yet validate a centralized session token.
- Web3 login is documented but not implemented yet.

## Goals

- Issue short-lived guest session tokens that represent a `guest_id` identity.
- Provide token verification endpoints for other services.
- Keep the API minimal and aligned with the head service integration model.
- Preserve a straightforward upgrade path to Web3 auth.

## Non-Goals

- Full account system or account recovery.
- Persistent player profile storage beyond a minimal guest record.
- Production-grade security hardening before the Web3 phase.

## Phase 1: Barebones Guest Auth

### API Surface

- `POST /auth/guest`
  - Accepts `guest_id`, `display_name`, and optional metadata.
  - Returns a short-lived session token and expiry.
- `POST /auth/verify-token`
  - Validates a session token and returns the associated guest identity.
- `POST /auth/logout`
  - Revokes the current session token.

### Data Model (In-Memory MVP)

- `guest_id`: string identifier from the client.
- `display_name`: latest supplied name.
- `session_id`: unique ID for each session.
- `token_hash`: stored hash of the issued token.
- `expires_at`: token expiration timestamp.

### Service Responsibilities

- Validate the guest payload (length/charset checks).
- Issue and sign tokens (JWT or opaque token + server-side storage).
- Provide verification for head, matchmaking, and game servers.
- Emit structured logs for issuance, verification, and revocation.

### Integration Steps

1. Head service calls `POST /auth/guest` when a guest session is needed.
2. Head service returns the token to the client or uses it server-to-server.
3. Game server verifies token on join via `POST /auth/verify-token`.

## Phase 2: Service Hardening

- Add rate limiting and basic abuse protection.
- Move token and session storage to a durable data store.
- Add tracing correlation IDs for cross-service visibility.
- Add metrics for token issuance and verification success rates.

## Phase 3: Web3 Auth Preparation

### API Additions

- `POST /auth/nonce`
  - Returns a one-time nonce with expiry.
- `POST /auth/verify`
  - Validates a signed login payload and issues a session token.

### Web3 Data Model Additions

- `wallet_address`: verified Solana address.
- `nonce`: one-time login challenge.
- `signature`: signed payload.
- `issued_at`: timestamp for signed payload verification.

### Client Flow Alignment

- Maintain guest flow as a fallback.
- Use the same session token shape for both guest and wallet auth.
- Prefer SIWS-style message formatting for wallet signatures.

## Phase 4: Migration Strategy

- Keep `guest_id` sessions working in parallel with Web3 sessions.
- Allow optional migration of guest profiles to wallet identities.
- Mark guest sessions as limited-trust in downstream services.

## Risks and Mitigations

- **Guest spoofing**: Accept as a known limitation in Phase 1.
- **Token misuse**: Short expiry + server-side revocation in Phase 2.
- **Replay attacks**: Nonce tracking and expiry in Phase 3.

## Milestones

1. Guest auth endpoints implemented and documented.
2. Head and game services use token verification.
3. Durable storage + rate limits in place.
4. Web3 nonce and signature verification added.
5. Guest + Web3 sessions supported side-by-side.
