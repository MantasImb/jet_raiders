# Plan: Head Server Matchmaking and Regional Lobby Orchestration

> Source PRD: `head_server/PRD.md`

## Architectural decisions

Durable decisions that apply across all phases:

- **Service boundary**: The head server remains the only client-facing backend
  for pre-game flows. The client talks to head for guest auth and matchmaking,
  then talks directly to the game server for realtime gameplay.
- **Queue ownership**: Matchmaking remains the source of truth for queue state.
  Head proxies queue creation, queue status lookup, and queue cancellation
  rather than storing queue state locally.
- **Match handoff contract**: A client-visible `matched` response is only valid
  after head has successfully created the target lobby on a game server.
- **Matched payload**: The client-ready matched response includes `match_id`,
  `lobby_id`, `ws_url`, and `region`.
- **Waiting payload**: The waiting response includes `status: waiting`,
  `ticket_id`, and `region`.
- **Ticket model**: `ticket_id` is the v1 handle for polling and cancellation.
- **Regional routing**: Head resolves region to a game server through a
  config-backed registry with a default global fallback.
- **External services**: Head integrates with auth for guest identity/session,
  with matchmaking for queue lifecycle, and with game servers for lobby
  creation.
- **Out of scope**: Session check, website-specific flows, dynamic game-server
  discovery, and push-based matchmaking notifications are not part of this
  plan.

---

## Phase 1: Head Queue Entry

**User stories**: 3, 4, 13

**Status**: Implemented in `head_server`.

### What to build

Add a client-facing matchmaking entry flow to head that follows the same
architectural shape as the existing guest auth flow. The client submits a queue
request to head, head validates and maps the request into an application
command, then delegates to a matchmaking client/port that calls the
matchmaking service.

This phase intentionally stops at the first queue boundary. It proves the new
head-side orchestration path, the upstream matchmaking integration boundary,
and the client contract for receiving either a waiting ticket or an immediate
match result from the existing matchmaking behavior.

### Acceptance criteria

- [x] The head server exposes a client-facing endpoint to enter matchmaking.
- [x] Head uses a dedicated matchmaking port/client rather than embedding HTTP
      concerns in the use-case layer.
- [x] A request that does not immediately match returns `waiting` with
      `ticket_id` and `region`.
- [x] An immediate upstream match result is surfaced cleanly through head.
- [x] Request validation and upstream error mapping follow the same quality bar
      as the existing guest auth handlers.
- [x] Use-case and adapter tests cover the new enqueue path.

---

## Phase 2: Ticket Polling Through Head

**User stories**: 5, 6, 13

### What to build

Extend matchmaking with ticket-based status lookup and add the corresponding
head endpoint that proxies client polls to the matchmaking service.

This is the first complete waiting-state slice. It proves that queue ownership
stays in matchmaking while the client only interacts with head. It also
clarifies the state transitions that every later phase will depend on:
waiting, matched, not found, and other terminal/error outcomes defined by the
service contracts.

### Acceptance criteria

- [ ] The matchmaking service exposes a status lookup contract keyed by
      `ticket_id`.
- [ ] The head server exposes a client-facing polling endpoint that proxies to
      matchmaking.
- [ ] A waiting ticket returns a waiting response without any lobby-creation
      attempt.
- [ ] The poll flow is fully testable through head with mocked upstream
      responses.
- [ ] Matchmaking tests cover ticket lookup behavior and expected status
      transitions.

---

## Phase 3: Matched Handoff to the Default Game Server

**User stories**: 7, 8, 9, 13

### What to build

When polling indicates a match is ready, head completes the full match handoff
before responding successfully to the client. Head resolves the default target
game server, creates the lobby there, and returns the final join payload with
`match_id`, `lobby_id`, `ws_url`, and `region`.

This is the first true end-to-end tracer bullet for the PRD because it covers
the entire client journey from queue wait to a real game-ready match. It is
also the highest-risk orchestration path because it crosses head,
matchmaking, and game-server boundaries.

### Acceptance criteria

- [ ] Head can create a lobby on the current default game server after a
      matched poll result.
- [ ] Head returns a matched response only after lobby creation succeeds.
- [ ] The matched response includes `match_id`, `lobby_id`, `ws_url`, and
      `region`.
- [ ] Lobby-creation failure is surfaced as a failed handoff rather than a
      false-success matched response.
- [ ] Integration-style coverage exists for the head-to-game-server lobby
      creation path.
- [ ] Two-player happy-path behavior is demoable end-to-end through head.

---

## Phase 4: Queue Cancellation

**User stories**: 10, 13

### What to build

Add ticket-based queue cancellation in matchmaking and expose it through head
as a client-facing cancel endpoint.

This phase focuses on the negative control path for the waiting lifecycle. It
gives the user a way to leave the queue cleanly and closes an important
usability gap without needing any client-visible expansion beyond the existing
head orchestration pattern.

### Acceptance criteria

- [ ] The matchmaking service supports cancellation by `ticket_id`.
- [ ] The head server exposes a client-facing cancellation endpoint.
- [ ] Canceling an active waiting ticket removes it from matchmaking state.
- [ ] A canceled ticket no longer transitions to matched on later polls.
- [ ] Head and matchmaking tests cover successful cancel, duplicate cancel, and
      unknown-ticket behavior.

---

## Phase 5: Regional Game-Server Routing

**User stories**: 11, 12, 13, 14

### What to build

Introduce a game-server registry abstraction in head that resolves the target
game server for a region. The first implementation is configuration-backed and
stores the internal base URL used for lobby creation plus the public `ws_url`
returned to clients, with a default global fallback when no specific regional
mapping exists.

This phase keeps the client contract stable while making regional expansion a
real part of the backend design instead of an undocumented assumption. The
same matchmaking and handoff flow continues to work, but head now owns the
selection logic explicitly.

### Acceptance criteria

- [ ] Head resolves game-server destination by region through a dedicated
      abstraction rather than hard-coded branching.
- [ ] The first implementation is configuration-backed and supports a default
      fallback server.
- [ ] The matched handoff flow uses the resolved server’s internal base URL for
      lobby creation and returns that server’s public `ws_url` to the client.
- [ ] Missing region-specific mappings fall back to the default global server.
- [ ] Tests cover exact region resolution and fallback behavior.
- [ ] The system remains demoable even when all configured regions point to the
      same current global game server.
