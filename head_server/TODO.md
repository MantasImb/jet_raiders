# Project TODO Checklist

## Platform Foundations

- [ ] Stand up the head service HTTP server and route table.
- [ ] Wire head service to the auth service for guest session creation.
- [ ] Add head service data store access for guest profiles and parties.
- [ ] Implement structured logging and correlation IDs across services.

## Guest and Profile Flow

- [ ] Implement head endpoints for guest profile upsert and retrieval.
- [ ] Add display name normalization in head APIs.
- [ ] Ensure auth returns guest session tokens via head.
- [ ] Update the client flow to call head before connecting to the game server.

## Game Server Cleanup

- [ ] Remove guest profile persistence from the game server join path.
- [ ] Stop normalizing display names inside the game server.
- [ ] Validate join payloads are already authenticated by head + auth.

## Matchmaking and Session Handoff

- [ ] Integrate head with matchmaking for queue entry.
- [ ] Ensure matchmaking returns lobby assignment and server address.
- [ ] Pass validated session identity to the game server on connection.

## Game Loop Parity

- [ ] Verify the game loop runs at the intended tick rate.
- [ ] Confirm world updates still broadcast after join changes.
- [ ] Validate player join/leave lifecycle remains intact.

## Observability and Testing

- [ ] Add basic health checks for head and auth services.
- [ ] Add integration tests for guest onboarding flows.
- [ ] Smoke test end-to-end flow: client -> head -> auth -> matchmaking -> game.
