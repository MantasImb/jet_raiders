# Score and Match Time Sync Plan (Game Server)

## Summary

This plan defines how the game server should provide score and match time data
to clients without sending those values every tick.

Chosen approach:

- Keep per-tick `WorldUpdate` focused on high-frequency simulation state
  (`entities`, `projectiles`, `tick`).
- Send score/time via `GameState` events.
- Add periodic authoritative resync (every 1 second) while the match is
  running to correct client drift.

This keeps bandwidth lower while preserving correctness.

## Scope

In scope:

- `game_server` code and protocol updates.
- Event model for score and timer synchronization.
- Server-side tests for protocol mapping and world task behavior.

Out of scope (for this phase):

- Implementing `game_client` UI logic for rendering score/timer.
- Any changes outside `game_server`.

## Design Goals

- Avoid sending score and timer in every `WorldUpdate`.
- Keep client countdown smooth via local simulation.
- Preserve server authority with correction events.
- Follow clean architecture boundaries in
  `/Users/mantas/repos/jet-raiders/game_server/CLEAN_ARCHITECTURE_GUIDELINES.md`.

## Public Contract Changes

Update `ServerState` / `GameState` payload semantics to include metadata for
score and timer synchronization.

### New server state payload shape

Use structured `ServerState` variants with metadata:

1. `MatchStarting`
   - `in_seconds: u32`
   - `match_duration_ms: Option<u64>`
2. `MatchRunning`
   - `started_at_unix_ms: u64`
   - `match_duration_ms: Option<u64>`
   - `scores: Vec<PlayerScoreDto>`
   - `reason: StateSyncReasonDto` (`Start`, `ScoreChanged`, `PeriodicResync`)
3. `MatchEnded`
   - `ended_at_unix_ms: u64`
   - `final_scores: Vec<PlayerScoreDto>`
   - `reason: MatchEndReasonDto` (`TimeLimit`, `Manual`, `Other`)

`PlayerScoreDto`:

- `id: String`
- `score: u32`

Notes:

- `match_duration_ms = None` means unlimited match (for the pinned test lobby).
- Keep `Lobby` as-is.

## Domain and Use Case Changes

### 1. Add authoritative score storage

File:
`/Users/mantas/repos/jet-raiders/game_server/src/domain/state.rs`

- Add `score: u32` to `SimEntity`.
- Initialize to `0` on join.
- Keep score through death/respawn.
- Remove naturally when entity is removed on leave.

### 2. Emit kill outcomes from projectile system

File:
`/Users/mantas/repos/jet-raiders/game_server/src/domain/systems/projectiles.rs`

- Extend projectile tick result to expose kill outcomes, e.g.:
  `Vec<KillEvent { killer_id: u64, victim_id: u64 }>`
- A kill is only when HP transitions from positive to zero.
- Do not mutate scoreboard inside protocol/adapter layer.

### 3. Apply scoring + trigger state sync in world task

File:
`/Users/mantas/repos/jet-raiders/game_server/src/use_cases/game.rs`

- On each kill event:
  - increment killer score with saturating add.
  - emit `ServerState::MatchRunning` with `reason = ScoreChanged`.
- On match start:
  - capture authoritative `started_at_unix_ms`.
  - emit running state once with full timer anchor and scores.
- Every 1 second during running match:
  - emit `ServerState::MatchRunning` with `reason = PeriodicResync`.
- On match end:
  - emit `ServerState::MatchEnded` with final scores and end reason.

Important:

- Keep `WorldUpdate` unchanged except existing simulation fields.
- Do not add score/time to `WorldUpdate`.

## Protocol Adapter Changes

File:
`/Users/mantas/repos/jet-raiders/game_server/src/interface_adapters/protocol.rs`

- Extend `ServerStateDto` and conversion logic to support the richer payload.
- Add `PlayerScoreDto`, `StateSyncReasonDto`, and `MatchEndReasonDto`.
- Keep serialization strictly in adapter layer.

File:
`/Users/mantas/repos/jet-raiders/game_server/src/interface_adapters/net/client.rs`

- Keep forwarding mechanism unchanged.
- Existing `forward_server_state` path will send richer `GameState` payloads.

## Client Simulation Model (Contract Expectations)

The client should:

- start local countdown from `started_at_unix_ms + match_duration_ms`.
- update score display from `scores`.
- on each periodic resync, correct drift.

Drift correction policy (for client implementation later):

- If absolute drift <= 150 ms: smooth adjustment.
- If absolute drift > 150 ms: snap to server-authoritative value.

## Edge Cases

- Unlimited match time (`match_duration_ms = None`):
  - no countdown required.
  - periodic score resync still applies.
- Late joiners:
  - receive latest `GameState` immediately on connection.
- Packet loss / lag:
  - periodic resync events restore consistency.
- Rapid multi-kill tick:
  - apply all kill increments before emitting consolidated score update for that
    tick.

## Test Plan

### Unit tests

File:
`/Users/mantas/repos/jet-raiders/game_server/src/domain/systems/projectiles.rs`

1. Kill event emitted exactly once when lethal hit occurs.
2. No kill event for non-lethal hit.
3. No self-kill events due to owner immunity.

### Use-case tests

File:
`/Users/mantas/repos/jet-raiders/game_server/src/use_cases/game.rs` (or a new
test module)

1. `MatchRunning` start payload includes timer anchor and initial scores.
2. Score increments produce `ScoreChanged` state update.
3. Periodic resync emits every ~1 second while running.
4. `MatchEnded` includes final scores and reason when time limit reached.

### Protocol mapping tests

File:
`/Users/mantas/repos/jet-raiders/game_server/src/interface_adapters/protocol.rs`

1. `ServerState` variants map to expected serialized DTO shapes.
2. `Option` timer semantics serialize correctly for limited vs unlimited lobbies.

### Integration tests

File:
`/Users/mantas/repos/jet-raiders/game_server/tests/ws_join.rs`

1. Join WebSocket and assert receipt of `GameState` payload with running anchor.
2. Assert periodic `GameState` resync messages arrive.
3. Validate score payload shape in `GameState`.

## Acceptance Criteria

1. Score and timer are not sent in per-tick `WorldUpdate`.
2. Server emits authoritative score/time anchors in `GameState`.
3. Server emits periodic (1 second) `GameState` resync while running.
4. Score updates are triggered by authoritative kill events only.
5. Existing entity/projectile replication remains functional.

## Assumptions and Defaults

- Score rule: `+1` per kill.
- Score type: `u32` with saturating increment.
- Periodic resync interval: `1 second`.
- Time reference: Unix milliseconds from server wall clock.
- Unlimited match is represented as `match_duration_ms = None`.
