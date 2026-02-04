# Matchmaking Service

## Purpose

The matchmaking service groups players into matches based on simple queue rules
and returns match results to the head service.

## Architecture Guidelines

Follow `CLEAN_ARCHITECTURE_GUIDELINES.md` for layering and dependency rules.

## Responsibilities

- Accept matchmaking requests from clients via the head service.
- Group players based on simple queue rules (region-only today).
- Return an immediate response indicating whether a match was found.

## Queue Flow (Player -> Match -> Head Service)

This flow describes how a player is queued and how the head service is told a
match is ready. It reflects the current simple in-memory implementation, while
leaving room for additional matchmaking rules later.

1. **Request received**: The head service sends `POST /matchmaking/queue` with
   `player_id`, `player_skill`, and `region`.
2. **Request validated**: The matchmaking handler checks required fields before
   enqueueing.
3. **Queue evaluation**: The matchmaker looks for a waiting player in the same
   region. Matching rules are intentionally minimal for now (region only).
4. **Match found**:
    - The waiting player is removed from the queue.
    - A `match_id` is created for the two players.
    - The service responds immediately with `status: matched` plus the opponent
      and match identifiers.
5. **No match yet**:
    - The player is stored in the in-memory queue.
    - A `ticket_id` is issued so the head service can track the request.
    - The service responds with `status: waiting`.
6. **Head service response**: The head service receives the response and either
   notifies the client immediately (matched) or waits to re-attempt once the
   queued entry is cleared (a duplicate request returns a conflict error).

## External Interfaces

### HTTP API

- `POST /matchmaking/queue`
  - Enqueues a player for matchmaking.
  - Returns a queue ticket or match assignment.

## Data Contracts

### Queue Request

- `player_id`: player identifier supplied by the head service.
- `player_skill`: matchmaking rating or tier.
- `region`: preferred region.

### Queue Response

- `status`: `waiting` or `matched`.
- `ticket_id`: queue ticket identifier when waiting.
- `match_id`: match identifier when matched.
- `opponent_id`: opponent player identifier when matched.
- `region`: the region used for matchmaking.

## Security Considerations

- Authenticate requests at the head service before calling matchmaking.
- Avoid leaking internal server addresses to unauthenticated clients.

## Dependencies

- Head service for client entry and validation.
- In-memory queue storage (current implementation).

## Observability

- Log queue activity and match assignments with correlation IDs.
