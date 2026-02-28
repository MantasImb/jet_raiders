# Matchmaking Server Code Plan

## Summary

The matchmaking service is already runnable and currently provides a minimal
in-memory queue endpoint:

- `POST /matchmaking/queue`

This document tracks the current implementation shape and the next incremental
steps.

## Current Crate Layout (Implemented)

```text
matchmaking_server/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── domain/
    │   ├── mod.rs
    │   └── queue.rs
    ├── use_cases/
    │   ├── mod.rs
    │   └── matchmaker.rs
    ├── interface_adapters/
    │   ├── mod.rs
    │   ├── protocol.rs
    │   ├── routes.rs
    │   ├── state.rs
    │   └── handlers/
    │       ├── mod.rs
    │       └── queue.rs
    └── frameworks/
        ├── mod.rs
        └── server.rs
```

## Current Behavior (Implemented)

1. Head service (or another caller) sends `POST /matchmaking/queue` with
   `player_id`, `player_skill`, and `region`.
2. Handler validates required fields.
3. Matchmaker checks for a waiting player in the same region.
4. If found, returns `matched` with `match_id` and `opponent_id`.
5. If not found, stores the player and returns `waiting` with `ticket_id`.
6. Duplicate enqueue by the same player returns conflict.

## Layer Ownership

- `domain/`: queue entities and ID builders.
- `use_cases/`: matchmaking orchestration and outcomes/errors.
- `interface_adapters/`: HTTP DTOs, validation, route wiring, app state.
- `frameworks/`: runtime bootstrap, tracing setup, and server startup.

## Current Constraints

- Queue storage is in-memory only.
- Matching rule is region-only.
- No queue status/cancel endpoints yet.
- No auth validation at the matchmaking boundary yet.

## Incremental Delivery Plan

1. Add auth validation middleware or service call for queue requests.
2. Add status endpoint for ticket polling.
3. Add cancel endpoint for queue withdrawal.
4. Add queue expiry/cleanup for stale waiting entries.
5. Add metrics and richer tracing for queue depth and wait time.
6. Introduce a persistence adapter (for example Redis) behind a trait boundary.

## Acceptance Criteria for Next Iteration

- Unauthorized queue requests are rejected.
- Queued players can query status and cancel.
- Stale tickets are cleaned up deterministically.
- Existing enqueue/match behavior remains compatible for current callers.
