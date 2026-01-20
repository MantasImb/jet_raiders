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

- `user_id`: authenticated user identifier.
- `party_id`: optional party identifier.
- `region`: preferred region.
- `skill`: matchmaking rating or tier.

### Match Assignment

- `lobby_id`: assigned lobby identifier.
- `server_addr`: game server address.
- `match_ticket`: short-lived token for the game server.
- `expires_at`: match ticket expiration time.

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
