# Matchmaking and Regional Orchestration Implementation Summary

## Source plans consolidated

This summary consolidates the implementation outcome of:

- `plans/head-server-matchmaking-and-regional-lobby-orchestration.md`
- `plans/stable-matched-ticket-lifecycle-and-idempotent-handoff.md`
- `plans/strict-regional-routing-shared-config.md`

The `plans/` directory was intentionally cleared for new feature planning.

## Final architecture decisions in force

- Head remains the only client-facing backend for pre-game flows.
- Matchmaking owns queue and ticket lifecycle state.
- `ticket_id` remains stable and reusable after matching.
- Matchmaking lifecycle states are `waiting`, `matched`, and `canceled`.
- Matched ticket payload from matchmaking includes:
  `ticket_id`, `match_id`, `player_ids`, `region`.
- Head only returns client `matched` after lobby handoff succeeds.
- Handoff can be triggered from enqueue or poll.
- Head derives `lobby_id = match_id`.
- Game-server lobby create `201` and `409` are both successful handoff outcomes.
- Region routing is exact match with no normalization and no implicit fallback.
- Shared region catalog is loaded from `config/regions.toml` by both head and
  matchmaking.

## Implementation status by phase

### Head and matchmaking orchestration

- Phase 1 (head queue entry): implemented.
- Phase 2 (ticket polling): implemented.
- Phase 3 (matched handoff with idempotent create): implemented.
- Phase 4 (ticket cancellation): implemented.
- Phase 5 (regional routing via shared config): implemented.

### Stable matched lifecycle and idempotent handoff refinements

- Stable matched ticket lifecycle: implemented.
- Dual-ticket visibility to shared matched payload: implemented.
- Head retry-safe handoff behavior on repeated matched requests: implemented.

### Strict regional routing and shared config

- Shared config artifact with concrete regions: implemented.
- Head strict startup validation: implemented.
- Head exact region resolution and unknown-region error path: implemented.
- Matchmaking shared catalog load and queue-entry region validation:
  implemented.

## Key implementation files

- Shared config: `config/regions.toml`
- Head config loading: `head_server/src/frameworks/config.rs`
- Head startup wiring: `head_server/src/frameworks/server.rs`
- Head region directory: `head_server/src/frameworks/game_server_directory.rs`
- Head matchmaking orchestration:
  `head_server/src/use_cases/matchmaking.rs`
- Matchmaking lifecycle core:
  `matchmaking_server/src/use_cases/matchmaker.rs`
- Matchmaking region validation handler:
  `matchmaking_server/src/interface_adapters/handlers/queue.rs`
- Matchmaking startup/catalog wiring:
  `matchmaking_server/src/frameworks/config.rs`
  `matchmaking_server/src/frameworks/server.rs`

## E2E and test coverage state

### Rust tests

- `head_server`: passing (`cargo test`)
- `matchmaking_server`: passing (`cargo test`)
- `game_server`: passing (`cargo test`)

### Bruno head E2E coverage

Expanded and passing with `bru run head -r --tests-only --sandbox developer`.

Covered flows include:

- Standard waiting/poll/match/re-enqueue/cancel/unknown-ticket flow.
- `us-east` matched handoff flow:
  - enqueue waiting (`player C`)
  - immediate match (`player D`)
  - poll returns same matched handoff result
- Exact region validation (`400`) for:
  - uppercase alias (`EU-WEST`)
  - trailing whitespace (`eu-west `)
  - unknown region (`ap-south`)

## Operational notes

- `PROCESS_COMPOSE.md` now matches `process-compose.yaml`:
  `matchmaking-server` is listed as active.
- `website` remains the only inactive placeholder in process-compose config.

## Remaining deferred follow-ups

These are intentionally not part of the completed scope:

- Ticket expiry/cleanup policy for long-lived matched tickets.
- Additional terminal lifecycle states if product requires them.
- Durable matchmaking persistence beyond in-memory state.
- Stronger duplicate-handoff verification/readback contract if required later.
- Dynamic game-server discovery and runtime config reload.

## Current stop condition

This implementation step is complete for the consolidated plans:

- Stable ticket lifecycle is enforced.
- Idempotent lobby handoff is enforced.
- Shared strict regional routing is enforced end-to-end.
- Client-visible head contract is stable and tested.
