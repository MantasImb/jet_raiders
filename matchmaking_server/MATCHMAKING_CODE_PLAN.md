# Matchmaking Server Code Plan

## Summary

The matchmaking service is already runnable and now exposes a ticket-centric
in-memory lifecycle API through head:

- `POST /matchmaking/queue`
- `GET /matchmaking/queue/{ticket_id}`
- `DELETE /matchmaking/queue/{ticket_id}`

This document tracks the implemented shape and the remaining incremental work.

## Current Crate Layout (Implemented)

```text
matchmaking_server/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── domain/
    │   └── mod.rs
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
        ├── id_generator.rs
        ├── mod.rs
        └── server.rs
```

## Current Behavior (Implemented)

1. Head service sends `POST /matchmaking/queue` with `player_id`,
   `player_skill`, and `region`.
2. Handler validates required fields.
3. Matchmaker checks for a waiting player in the same region.
4. If found, it creates a canonical matched record and returns `matched` with
   `ticket_id`, `match_id`, `player_ids`, and `region`.
5. If not found, it stores the player and returns `waiting` with `ticket_id`
   and `region`.
6. Later `GET /matchmaking/queue/{ticket_id}` returns `waiting`, `matched`, or
   `canceled`, and `DELETE /matchmaking/queue/{ticket_id}` transitions a
   waiting ticket to `canceled`.

## Current In-Memory Model (Implemented)

- `queue: VecDeque<String>` stores waiting `ticket_id` order.
- `tickets_by_id: HashMap<String, TicketRecord>` is the authoritative ticket
  store.
- `active_ticket_by_player: HashMap<u64, String>` preserves the current active
  ticket handle for each player.
- `matches_by_id: HashMap<String, MatchRecord>` stores the canonical matched
  roster once per `match_id`.

Matched tickets store `match_id` and resolve their shared payload through
`matches_by_id` during lookup instead of embedding duplicate roster data in
each ticket record.

## Layer Ownership

- `domain/`: reserved for future matchmaking-specific domain entities.
- `use_cases/`: matchmaking orchestration and outcomes/errors.
- `interface_adapters/`: HTTP DTOs, validation, route wiring, app state.
- `frameworks/`: runtime bootstrap, tracing setup, server startup, and opaque
  ID generation.

## Current Constraints

- Queue storage is in-memory only.
- Matching rule is region-only.
- Head owns auth verification before calling matchmaking.
- `match_id` is opaque and does not encode roster data.

## Incremental Delivery Plan

1. Add queue expiry/cleanup for stale waiting entries.
2. Add metrics and richer tracing for queue depth and wait time.
3. Introduce a persistence adapter behind a trait boundary if in-memory state
   becomes an operational limit.

## Acceptance Criteria for Next Iteration

- Stale tickets are cleaned up deterministically.
- Existing enqueue, match, poll, and cancel behavior remains compatible for
  current callers.
