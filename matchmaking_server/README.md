# Matchmaking Service

## Purpose

The matchmaking service groups players into matches, chooses an appropriate
region/game server, and returns lobby assignments to clients.

## Responsibilities

- Accept matchmaking requests from authenticated clients.
- Group players based on queue rules (region, skill, party size).
- Select a target game server and lobby ID.
- Issue a short-lived match ticket for the game server.
- Emit match status updates to clients.

## Queue Flow (Player -> Match -> Head Service)

This flow describes how a player is queued and how the head service is told a
match is ready. It reflects the current simple in-memory implementation, while
leaving room for additional matchmaking rules later.

1. **Request received**: The head service sends `POST /matchmaking/queue` with
   `player_id`, `player_skill`, and `region`.
2. **Request validated**: The matchmaking handler checks required fields and
   normalizes the payload for the queue use case.
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
6. **Head service response**: The head service receives the response and can
   either notify the client immediately (matched) or keep polling until a match
   is returned.

## Current Axum Server Functionality to Extract

- Shift the current direct `/ws` connection flow into a matchmaking-driven
  assignment step so the game server only accepts connections with a lobby
  assignment.
- Move the "join before input" gatekeeping checks into matchmaking so the game
  server receives pre-validated lobby members.

## External Interfaces

### HTTP API

- `POST /matchmaking/queue`
  - Enqueues a player or party for matchmaking.
  - Returns a queue ticket.
- `POST /matchmaking/status`
  - Returns queue status or match assignment.
- `POST /matchmaking/cancel`
  - Cancels an active queue ticket.

### WebSocket (Optional)

- `/matchmaking/ws`
  - Real-time updates for queue position and match assignment.

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

- Validate session tokens with the auth service.
- Keep match tickets short-lived and single-use.
- Avoid leaking internal server addresses to unauthenticated clients.

## Dependencies

- Auth service for token validation.
- Game server registry or allocator.
- Data store for queue state and rules.

## Observability

- Track queue wait times by region.
- Log match assignments with correlation IDs.
- Alert on allocation failures or capacity issues.
