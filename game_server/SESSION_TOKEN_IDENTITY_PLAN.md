# Session Token Identity Plan

## Goal

Make `game_server` identity-agnostic by trusting only `session_token` at join.
Resolve `user_id` and `display_name` through `auth_server` so guest and
authenticated users follow the same logic.

## Context From This Conversation

- `game_server` should not depend on `guest_id` semantics.
- Keep auth route style as `/auth/...` for the auth microservice.
- `guest_id` moved to numeric `u64` in auth/head in current WIP changes.
- `display_name` is now validated server-side in auth:
  - trim whitespace
  - length `3..=32`
  - allowed charset `[A-Za-z0-9 _-]`
- `game_server` currently accepts identity fields in join payload and does not
  verify session tokens yet.

## Target Runtime Flow

1. Client sends `Join { session_token }`.
2. `game_server` calls `POST /auth/verify-token`.
3. `auth_server` returns canonical identity payload:
   - `user_id: u64`
   - `display_name: String`
   - `session_id: String`
   - `expires_at: u64`
4. `game_server` binds connection ownership to verified `user_id`.
5. `game_server` ignores client-supplied identity values.

## Design Rules

- `user_id` is canonical and comes only from auth verification.
- Gameplay loop remains keyed by numeric runtime IDs for performance.
- Any profile data from client is non-authoritative.
- Never log raw session tokens.
- Verify token once on join, not on every input packet.

## Implementation Phases

## Phase 1: Auth Contract Normalization

### Changes

- Update verify-token response to return canonical `user_id`.
- Keep optional temporary compatibility fields only if migration requires them.
- Ensure guest and authenticated sessions both map to `user_id: u64`.

### Files

- `auth_server/src/interface_adapters/protocol.rs`
- `auth_server/src/use_cases/verify_token.rs`
- `auth_server/src/interface_adapters/handlers.rs`
- `auth_server/src/domain/entities.rs`
- `auth_server/src/use_cases/guest_login.rs`

## Phase 2: Game Server Auth Client

### Changes

- Add a dedicated auth client in outer layers.
- Add config for auth base URL and verification timeout.
- Inject auth client into websocket join path.
- Add typed failure handling:
  - invalid token
  - expired token
  - upstream unavailable

### Files

- `game_server/src/frameworks/config.rs`
- `game_server/src/interface_adapters/state.rs`
- `game_server/src/interface_adapters/net/client.rs`
- `game_server/src/interface_adapters/clients/auth.rs` (new)

## Phase 3: Join Protocol Migration

### Changes

- Replace join payload identity fields with `session_token`.
- Parse and validate join payload.
- Verify token before spawning player or accepting inputs.
- Close socket on verification failure with clear reason.

### Files

- `game_server/src/interface_adapters/protocol.rs`
- `game_server/src/interface_adapters/net/client.rs`
- `game_client/Scripts/NetworkManager.gd`
- `game_client/Scripts/UserManager.gd`

## Phase 4: Verified Identity Ownership

### Changes

- Bind connection identity to verified `user_id`.
- Ensure lobby allow-lists compare against verified IDs.
- Keep `display_name` metadata sourced from auth response.

### Files

- `game_server/src/interface_adapters/net/client.rs`
- `game_server/src/use_cases/lobby.rs`
- `game_server/src/interface_adapters/net/internal.rs`

## Phase 5: Caching and Resilience

### Changes

- Add short-lived in-memory verify cache:
  - key: token hash
  - value: verified identity and expiry
- Cache TTL = min(configured cache TTL, token remaining lifetime).
- Add configurable join behavior for auth downtime:
  - production: fail closed
  - local dev: optional fail-open flag

### Files

- `game_server/src/interface_adapters/state.rs`
- `game_server/src/frameworks/config.rs`
- `game_server/src/interface_adapters/net/client.rs`

## Phase 6: Observability

### Changes

- Emit structured logs on join:
  - `user_id`
  - `session_id`
  - `lobby_id`
  - auth verification latency
- Add metrics:
  - verify success/failure count
  - verify latency histogram
  - cache hit ratio

### Files

- `game_server/src/interface_adapters/net/client.rs`
- `game_server/src/frameworks/server.rs` (metrics wiring if needed)

## Phase 7: Migration Rollout

1. Add token-based join path while keeping temporary compatibility for old join
   payloads.
2. Update game client to always send `session_token` on join.
3. Enable strict mode in `game_server` to require `session_token`.
4. Remove guest-specific fields from `game_server` protocol and connection
   context.

## Compatibility and Security Notes

- SQL injection risk is already mitigated by parameterized queries in auth
  persistence.
- Name quality is now enforced by auth; game server should trust verified
  profile fields from auth and avoid divergent validation rules.
- Do not pass `user_id` from client to game server as source of truth.

## Open Decisions

- Whether `player_id` in simulation should equal verified `user_id` or remain a
  separate runtime ID mapped to `user_id`.
- Recommended default:
  - keep runtime `player_id` internal for simulation performance and lifecycle
    control
  - store `user_id` as canonical identity in connection/session metadata

## Test Plan

### Auth

- Valid token returns canonical `user_id`.
- Expired token is rejected.
- Invalid token is rejected.

### Game Server

- Join with valid token succeeds.
- Join with invalid token fails before spawn.
- Join with expired token fails before spawn.
- Duplicate join for same verified `user_id` follows replacement policy.
- Lobby allow-list checks verified `user_id`.

### End-to-End

- Head login -> token issuance -> websocket join succeeds.
- Guest and authenticated flows both produce identical game-server join logic.
