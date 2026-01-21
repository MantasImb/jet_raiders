# Head Service Development Plan

## Goals

- Move guest profile persistence and display name normalization out of the game
  server and into the head service.
- Connect the head service to the auth service for guest validation and session
  creation.
- Keep the game server focused on real-time gameplay only.

## Scope

### In Scope

- Head service HTTP endpoints for profile and guest flows.
- Auth service integration for guest session validation and issuance.
- Game server cleanup to remove guest profile persistence and validation.
- Auth service ownership of guest session storage.

### Out of Scope

- Matchmaking flow changes beyond token validation.
- Real-time gameplay protocol changes unrelated to identity.

## Workstreams

### 1. Requirements Alignment

- Confirm the head service remains the client entry point.
- Confirm the auth service owns guest session validation and issuance.
- Confirm the auth service stores guest session records.
- Capture payload contracts for guest identity data between services.

### 2. Data Model and Storage

- Define the guest profile schema in the head service data store.
- Plan data migration from the game server guest profile table.
- Document a rollback plan if migration fails.

### 3. Head Service API Design

- Add endpoints for guest profile upsert and retrieval.
- Normalize display name rules in the head service.
- Return validated guest identity data to the client.

### 4. Auth Service Integration

- Define the request flow: head service calls auth to create or validate guest
  sessions.
- Ensure auth tokens are returned to the head service for client use.
- Add retry and error handling policies for auth failures.

### 5. Game Server Cleanup

- Remove guest profile upsert from the WebSocket join path.
- Remove display name normalization in the game server join handler.
- Update the join payload handling to rely on validated identity data.

### 6. Contract and Protocol Updates

- Document the join payload expected after head + auth validation.
- Update client integration notes to call head first, then connect to the game
  server.
- Ensure server logs and metrics reflect the new flow.

### 7. Rollout Plan

- Phase 1: Implement head service APIs and auth integration behind a feature
  flag.
- Phase 2: Update client flow to use head service before connecting to the game
  server.
- Phase 3: Remove legacy guest persistence in the game server.

## Dependencies

- Auth service endpoints for guest session validation and issuance.
- Head service data store for profiles.
- Migration scripts for guest profile data.

## Risks and Mitigations

- **Risk:** Auth service unavailability.
  - **Mitigation:** Add retries and clear client error messaging.
- **Risk:** Guest profile data mismatch after migration.
  - **Mitigation:** Validate migrated data and keep a rollback plan.

## Acceptance Criteria

- Game server no longer writes guest profiles.
- Head service owns display name normalization.
- Auth service validates or issues guest sessions used by clients.
- Client can complete onboarding without calling the game server first.
