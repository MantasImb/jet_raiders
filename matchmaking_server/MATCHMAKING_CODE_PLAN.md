# Matchmaking Server Code Plan

## Goals

- Implement a clean, game-agnostic matchmaking service that follows the
  repository architecture and clean architecture guidelines.
- Keep the first iteration as small and direct as possible, with only the
  minimum endpoints and logic needed to queue and return assignments.
- Keep queue state and match assignment logic isolated from transport concerns.

## Proposed Crate Layout (Initial)

```text
matchmaking_server/
├── Cargo.toml
└── src/
    ├── main.rs            # Bootstrap, config, wiring, server startup.
    ├── config.rs          # Env/config parsing and types.
    ├── net/               # HTTP adapters and request/response mapping.
    │   ├── mod.rs
    │   └── http.rs         # REST endpoints for queue/status/cancel.
    ├── protocol/          # Wire DTOs for HTTP payloads.
    │   ├── mod.rs
    │   └── http.rs
    ├── application/       # Use cases and orchestration.
    │   ├── mod.rs
    │   ├── queue_service.rs
    │   └── match_service.rs
    ├── domain/            # Core domain logic and entities.
    │   ├── mod.rs
    │   ├── queue.rs
    │   ├── match.rs
    │   └── tickets.rs
    ├── storage/           # Data store abstractions + implementations.
    │   ├── mod.rs
    │   └── memory.rs
    └── clients/           # Outbound clients for auth and game server registry.
        ├── mod.rs
        ├── auth.rs
        └── server_registry.rs
```

## Layering Rules

- **Domain** contains matchmaking rules, queue entities, and ticket logic.
- **Application** owns use cases and orchestration without transport details.
- **Protocol** defines wire DTOs for HTTP and WebSocket messages.
- **Net** owns Axum handlers, request validation, and DTO conversions.
- **Storage** holds repository traits and concrete persistence adapters.
- **Clients** provides integration boundaries for external services.
- **Main** wires dependencies and starts the server.

## Core Data Model

- `QueueEntry`
  - `user_id`, `party_id`, `region`, `skill`, `enqueued_at`.
- `QueueTicket`
  - `ticket_id`, `expires_at`.
- `MatchAssignment`
  - `lobby_id`, `server_addr`, `match_ticket`, `expires_at`.

## Primary Use Cases

### Enqueue

1. Validate auth token with the auth client.
2. Normalize queue request and create a `QueueEntry`.
3. Store entry and return a `QueueTicket`.

### Status

1. Lookup ticket in storage.
2. Return queue position or `MatchAssignment` if available.

### Cancel

1. Validate ticket.
2. Remove from queue and release resources.

## Matching Flow

- A background task or periodic tick evaluates queued entries.
- Candidate groups are formed by region, party size, and skill window.
- A server registry client selects a target game server.
- A match ticket is minted with a short TTL and stored.
- Assignment is published for HTTP polling or WS subscribers.

## Storage Strategy

- Start with in-memory storage for development and testing.
- Define repository traits for queue entries, tickets, and assignments.

## Observability

- Emit structured logs for queue sizes, match assignment, and failures.

## Security Considerations

- All queue operations require validated auth tokens.
- Match tickets are short-lived and single-use.
- Avoid exposing internal server metadata in public responses.

## Incremental Delivery Plan

1. Bootstrap Axum server with health endpoint and config.
2. Implement protocol DTOs for queue/status/cancel.
3. Build in-memory queue repository and use cases.
4. Wire HTTP endpoints to use cases.
5. Add match assignment logic and server registry client.

## Future Feature List

- Optional WebSocket updates for queue status and match assignment.
- Redis storage adapter for shared state and horizontal scaling.
- Metrics for queue wait time by region and allocation success rate.
- Additional queue rules (party size constraints, skill widening windows).
- Dedicated background worker for match evaluation and allocation retries.
