# Plan: Head Server Phase 3 Default Game-Server Handoff Implementation

> Source artifacts:
> `head_server/PRD.md`
> `plans/head-server-matchmaking-and-regional-lobby-orchestration.md`
> `plans/stable-matched-ticket-lifecycle-and-idempotent-handoff.md`
> `head_server/src/use_cases/matchmaking.rs`
> `head_server/src/interface_adapters/handlers/matchmaking.rs`
> `head_server/src/frameworks/game_server_client.rs`
> `head_server/src/frameworks/game_server_directory.rs`
> `game_server/src/interface_adapters/net/internal.rs`

## Objective

Complete head-server phase 3 so that a stable matched result from matchmaking
is turned into a game-ready lobby assignment on the default game server before
head returns a successful matched response to the client.

This plan is focused on the head-side implementation slice that consumes the
already-settled matchmaking lifecycle contract and turns it into a safe,
retryable lobby handoff flow.

## Scope

In scope for this phase:

- Preserve the current head behavior where handoff may be triggered by either
  enqueue or polling.
- Create or confirm a lobby when head observes `status: matched`.
- Return the final matched client payload only after handoff succeeds.
- Include `ticket_id` in the final matched head response for
  traceability/debugging.
- Treat game-server `201 created` and `409 already exists` as successful
  handoff outcomes.
- Keep retry behavior stateless in head: failed handoff returns an error and
  later matched requests retry the same handoff.
- Add or update Rust unit tests around the use case, adapters, and framework
  client behavior.
- Add or update Bruno/process-compose scenarios for the real multi-service
  flow.
- Update documentation to reflect the final matched head contract.

Out of scope for this phase:

- Changing matchmaking ownership of queue or ticket state.
- Durable storage for handoff state.
- Game-server readback or roster verification for duplicate creates.
- Ticket expiry or cleanup.
- Dynamic game-server discovery.
- New client-visible lifecycle states beyond `waiting`, `matched`, and
  `canceled`.

## Settled Decisions

- **Source of truth**: Matchmaking remains authoritative for ticket state and
  matched roster data.
- **Matched handoff trigger**: Head may attempt handoff on either enqueue or
  poll. The first request that observes `matched` attempts lobby creation.
- **Retry model**: Later matched requests retry the same handoff rather than
  relying on head-owned persistence.
- **Lobby identifier**: `lobby_id = match_id`.
- **Duplicate create behavior**: Game-server `409 already exists` is
  definitive success for the requested `lobby_id`.
- **Final matched payload**: Head returns `status`, `ticket_id`, `match_id`,
  `lobby_id`, `ws_url`, and `region`.
- **Ticket meaning after handoff**: `ticket_id` remains only for traceability
  and debugging. It does not gain new post-handoff behavior.
- **Failure model**: If lobby creation fails, head returns an error and
  retries on later matched requests until a `201` or `409` is observed.
- **Integration coverage**: Rust tests protect local behavior; Bruno plus
  process-compose protect the real multi-service flow.

## Current Codebase State

The core phase-3 seams already exist:

- `MatchmakingService` in
  `head_server/src/use_cases/matchmaking.rs` already maps matched lifecycle
  results into `complete_handoff(...)`.
- The same use case path is already used for both enqueue and polling.
- `GameServerClient` in
  `head_server/src/frameworks/game_server_client.rs` already maps `201` to
  `Created` and `409` to `AlreadyExists`.
- `StaticGameServerDirectory` in
  `head_server/src/frameworks/game_server_directory.rs` already resolves the
  target server for the current region model.
- `game_server` already exposes `POST /lobbies` with `lobby_id` and
  `allowed_player_ids`, returning `201` on create and `409` on duplicate.

The main remaining implementation gap is contract completion: head currently
drops `ticket_id` from the final matched response and the tests/docs still
need to be aligned with the settled phase-3 behavior.

## Implementation Workstreams

### Workstream 1: Finalize the head use-case matched contract

Goal:
Carry the matched ticket handle through head handoff so the final matched
result preserves the caller's `ticket_id` alongside the shared lobby
assignment.

Primary files:

- `head_server/src/use_cases/matchmaking.rs`

Implementation tasks:

- Extend `HeadMatchmakingResult::Matched` to carry `ticket_id` in addition to
  `match_id`, `lobby_id`, `ws_url`, and `region`.
- Preserve the matched ticket handle from
  `MatchmakingLifecycleState::Matched { ticket_id, ... }` when calling the
  handoff path.
- Update `complete_handoff(...)` so it accepts the caller's `ticket_id` and
  returns it in the final matched result.
- Keep the existing stateless retry model intact; no head-owned handoff cache
  should be introduced.

Acceptance checks:

- A matched enqueue response includes the enqueue caller's `ticket_id`.
- A matched poll response includes the polled ticket's `ticket_id`.
- Two tickets for the same match return identical `match_id`, `lobby_id`,
  `ws_url`, and `region`, but preserve their own distinct `ticket_id`.

### Workstream 2: Finalize the head HTTP response shape

Goal:
Expose the settled matched payload at the adapter boundary without changing
the waiting and canceled response semantics.

Primary files:

- `head_server/src/interface_adapters/handlers/matchmaking.rs`
- Head protocol DTO file(s) under `head_server/src/interface_adapters/`

Implementation tasks:

- Update the matched response mapping so `ticket_id` is present for
  `HeadMatchmakingResult::Matched`.
- Keep the waiting and canceled payloads unchanged.
- Ensure the response DTO schema and serialization behavior match the updated
  PRD and plan docs.

Acceptance checks:

- Matched JSON responses include `ticket_id`.
- Waiting and canceled responses remain unchanged.
- No transport-layer branching is introduced that treats enqueue and poll
  differently once the use case returns a matched result.

### Workstream 3: Protect the retry-safe handoff contract with unit tests

Goal:
Pin the phase-3 behavior in Rust tests so the contract is difficult to
regress during later routing and cancellation work.

Primary files:

- `head_server/src/use_cases/matchmaking.rs`
- `head_server/src/interface_adapters/handlers/matchmaking.rs`
- `head_server/src/frameworks/game_server_client.rs`

Implementation tasks:

- Update use-case tests so matched enqueue and matched poll assertions include
  `ticket_id`.
- Add or refine tests that prove:
  - matched enqueue succeeds only after lobby creation succeeds
  - matched poll succeeds only after lobby creation succeeds
  - duplicate lobby create (`409`) is treated as success
  - failed lobby create returns an error and does not fabricate a matched
    success
  - repeated matched requests remain retry-safe without local state
- Add or update adapter tests to verify matched JSON payload shape now
  includes `ticket_id`.
- Keep framework tests for `GameServerClient` confirming `201` and `409`
  behavior.

Acceptance checks:

- Head use-case tests cover first-create and duplicate-create handoff paths.
- Adapter tests cover the final matched response shape.
- Framework tests continue to verify `201` and `409` mapping.

### Workstream 4: Add real multi-service handoff coverage in Bruno

Goal:
Exercise the actual phase-3 flow through head, matchmaking, and game server
using process-compose rather than mocking service boundaries.

Primary artifacts:

- `bruno/` collection files for head matchmaking flow
- `process-compose.yaml`
- Existing test/run docs if they need command references

Implementation tasks:

- Add a happy-path scenario where:
  - player A enters queue and receives `waiting`
  - player B enters queue and receives final `matched` with `ticket_id`,
    `match_id`, `lobby_id`, `ws_url`, and `region`
  - player A polls and receives the same final lobby assignment with A's own
    `ticket_id`
- Add a retry-safe scenario where a matched poll succeeds after the lobby
  already exists.
- Document any required process-compose startup order or test data needed to
  run the collection reliably.

Suggested Bruno paths:

- `bruno/head-matchmaking/09-immediate-match-creates-lobby-and-returns-ticket.bru`
- `bruno/head-matchmaking/10-matched-poll-reuses-existing-lobby.bru`

Acceptance checks:

- The phase-3 handoff is demoable end-to-end through head.
- Bruno scenarios validate the final matched payload shape and retry-safe
  duplicate-create behavior.

### Workstream 5: Align docs with the implemented contract

Goal:
Keep the repository's planning and service docs synchronized with the actual
phase-3 behavior so later phases inherit the correct contract.

Primary files:

- `head_server/README.md`
- `head_server/PRD.md`
- `plans/head-server-matchmaking-and-regional-lobby-orchestration.md`
- `plans/stable-matched-ticket-lifecycle-and-idempotent-handoff.md`

Implementation tasks:

- Update `head_server/README.md` example matched responses so they include
  `ticket_id`.
- Verify all phase-3-facing docs consistently say handoff may occur on enqueue
  or poll.
- Verify docs consistently say `409 already exists` is successful handoff.
- Verify docs consistently describe `ticket_id` in matched responses as
  traceability/debugging only.

Acceptance checks:

- README, PRD, and plans all describe the same matched payload and handoff
  semantics.

## Suggested Implementation Order

1. Update the head use-case matched result shape to include `ticket_id`.
2. Update the head adapter/protocol mapping and response serialization.
3. Update or add Rust unit tests until the final matched contract is pinned.
4. Update `head_server/README.md` and confirm all phase-3 docs remain aligned.
5. Add Bruno/process-compose coverage for the real multi-service flow.

This order keeps the critical contract change small and testable before adding
end-to-end coverage.

## Detailed Acceptance Criteria

Phase 3 is complete when all of the following are true:

- Head returns `matched` only after game-server lobby creation succeeds or the
  target lobby is confirmed via `409 already exists`.
- Head may perform the same handoff from either enqueue or poll.
- The final matched head response includes:
  - `status`
  - `ticket_id`
  - `match_id`
  - `lobby_id`
  - `ws_url`
  - `region`
- The matched response uses the caller's own `ticket_id`.
- Head uses matchmaking `player_ids` unchanged as `allowed_player_ids` for the
  game-server create request.
- Failed lobby creation returns an error and later matched requests retry the
  same handoff.
- Rust unit tests pass for the use case, adapters, and game-server client.
- Bruno/process-compose scenarios cover the real handoff flow.
- Service and planning docs reflect the implemented contract.

## Validation Checklist

- Run head-server unit tests covering matchmaking orchestration.
- Run any relevant framework/client tests around game-server lobby creation.
- Run Bruno scenarios against services started with process-compose.
- Verify matched responses from both immediate-match enqueue and later poll
  include the expected `ticket_id`.
- Verify the same `match_id` maps to the same `lobby_id`.
- Verify duplicate create does not fail the client-visible handoff.

## Risks and Watchpoints

- **DTO drift**: It is easy to update the use case and forget the HTTP DTO.
  Keep response-shape tests close to the adapter boundary.
- **Asymmetric behavior**: Do not accidentally preserve `ticket_id` for poll
  but not enqueue, or vice versa.
- **Over-scoping**: This phase should not introduce new lifecycle states,
  persistence, or game-server readback.
- **Doc drift**: Several plan artifacts now describe phase 3. Keep them
  synchronized as the implementation lands.

## Deferred Follow-Ups

- Regional routing expansion beyond the default/fallback slice belongs to
  phase 5.
- Durable or inspectable handoff state remains deferred.
- Readback verification for duplicate create remains deferred.
- Ticket cleanup/expiry remains deferred.
