# Plan: Head Server Matchmaking and Regional Lobby Orchestration

> Source PRD: `head_server/PRD.md`
>
> Upstream dependency for later phases:
> `plans/matchmaking-server-head-integration-phases.md`
>
> Implementation note: phases 3 and 4 in this plan assume the matchmaking
> lifecycle and payload contract defined in the matchmaking integration plan
> has been implemented first.

## Architectural decisions

Durable decisions that apply across all phases:

- **Service boundary**: The head server remains the only client-facing backend
  for pre-game flows. The client talks to head for guest auth and matchmaking,
  then talks directly to the game server for realtime gameplay.
- **Queue ownership**: Matchmaking remains the source of truth for queue state.
  Head proxies queue creation, queue status lookup, and queue cancellation
  rather than storing queue state locally.
- **Ticket lifecycle**: Every enqueue creates or returns a `ticket_id`.
  `ticket_id` remains the v1 handle for polling and cancellation.
- **Matchmaking status model**: Matchmaking exposes `waiting`, `matched`, and
  `canceled` as explicit ticket lifecycle states. `404` is reserved for
  unknown or later-discarded tickets.
- **Head-facing matched payload**: The head-facing matched result returned from
  matchmaking includes `ticket_id`, `match_id`, `player_ids`, and `region`.
- **Match handoff contract**: A client-visible `matched` response is only valid
  after head has successfully created or confirmed the target lobby on a game
  server.
- **Handoff trigger**: Head may perform matched handoff on either queue entry
  or polling. The first head request that observes `matched` attempts lobby
  creation; later matched requests retry the same idempotent handoff.
- **Matched payload**: The client-ready matched response includes `ticket_id`,
  `match_id`, `lobby_id`, `ws_url`, and `region`.
- **Waiting payload**: The waiting response includes `status: waiting`,
  `ticket_id`, and `region`.
- **Canceled payload**: The client-visible canceled response includes
  `status: canceled`, `ticket_id`, and `region`.
- **Immediate-match behavior**: Matchmaking may form a match during enqueue,
  but head still owns the client-visible handoff contract. Head should only
  expose a final matched response once lobby creation has succeeded.
- **Re-enqueue behavior**: Matchmaking may return an existing `waiting` or
  `matched` result on re-enqueue for the same player. Head should treat those
  responses as normal success-shaped lifecycle results rather than transport
  errors.
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
head-side orchestration path and the upstream matchmaking integration
boundary. The currently implemented behavior surfaces the upstream enqueue
result through head; later phases tighten the long-term ticket-centric
contract that this flow will eventually align to.

### Acceptance criteria

- [x] The head server exposes a client-facing endpoint to enter matchmaking.
- [x] Head uses a dedicated matchmaking port/client rather than embedding HTTP
      concerns in the use-case layer.
- [x] A request that does not immediately match returns `waiting` with
      `ticket_id` and `region`.
- [x] The current enqueue result from matchmaking is surfaced cleanly through
      head.
- [x] Request validation and upstream error mapping follow the same quality bar
      as the existing guest auth handlers.
- [x] Use-case and adapter tests cover the new enqueue path.

---

## Phase 2: Ticket Polling Through Head

**User stories**: 5, 6, 13

**Status**: Implemented in `head_server` and `matchmaking_server`.

### What to build

Extend matchmaking with ticket-based status lookup and add the corresponding
head endpoint that proxies client polls to the matchmaking service.

This is the first complete waiting-state slice. It proves that queue ownership
stays in matchmaking while the client only interacts with head. It also
clarifies the state transitions that every later phase will depend on:
waiting, matched, canceled, and not found outcomes defined by the service
contracts. The fully aligned target contract includes explicit lifecycle
states and a unified response shape; phase 2's current implementation is the
first concrete polling slice toward that model.

### Acceptance criteria

- [x] The matchmaking service exposes a status lookup contract keyed by
      `ticket_id`.
- [x] The head server exposes a client-facing polling endpoint that proxies to
      matchmaking.
- [x] A waiting ticket returns a waiting response without any lobby-creation
      attempt.
- [x] Head preserves explicit terminal states from matchmaking rather than
      fabricating local queue state.
- [x] The poll flow is fully testable through head with mocked upstream
      responses.
- [x] Matchmaking tests cover ticket lookup behavior and expected status
      transitions.

---

## Phase 3: Matched Handoff to the Default Game Server

**User stories**: 7, 8, 9, 13

### What to build

When head observes that a match is ready, it completes the full match handoff
before responding successfully to the client. The triggering request may be
the enqueue call that first receives `matched` or a later poll that is the
first head request to observe the stable matched state. Head resolves the
default target game server, creates the lobby there, and returns the final
join payload with `ticket_id`, `match_id`, `lobby_id`, `ws_url`, and `region`.

This is the first true end-to-end tracer bullet for the PRD because it covers
the entire client journey from queue wait to a real game-ready match. It is
also the highest-risk orchestration path because it crosses head,
matchmaking, and game-server boundaries. This phase assumes the upstream
matched payload is stable and ticket-centric: `ticket_id`, `match_id`,
`player_ids`, and `region`.

### Acceptance criteria

- [x] Head can create a lobby on the current default game server when enqueue
      or poll returns a matched lifecycle result.
- [x] Head returns a matched response only after lobby creation succeeds.
- [x] The matched response includes `ticket_id`, `match_id`, `lobby_id`,
      `ws_url`, and `region`.
- [x] Head uses the upstream `player_ids` roster as the authoritative
      `allowed_player_ids` input when creating the game-server lobby.
- [x] Head derives `lobby_id` from `match_id` and treats duplicate lobby create
      outcomes as retry-safe handoff success for the same match.
- [x] Game-server `409 already exists` is treated as a successful handoff for
      the requested `lobby_id`.
- [x] Lobby-creation failure is surfaced as a failed handoff rather than a
      false-success matched response, and repeated matched requests continue to
      retry until handoff succeeds.
- [x] Rust unit tests cover first-create and duplicate-create handoff behavior.
- [x] Bruno/process-compose coverage exists for the real head-to-game-server
      lobby creation path.
- [x] Two-player happy-path behavior is demoable end-to-end through head.

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

- [x] The matchmaking service supports cancellation by `ticket_id`.
- [x] The head server exposes a client-facing cancellation endpoint.
- [x] Canceling an active waiting ticket returns explicit `canceled` state.
- [x] A canceled ticket no longer transitions to matched on later polls.
- [x] A matched ticket cannot be canceled.
- [x] Re-enqueue after cancel is delegated to matchmaking and results in a new
      waiting ticket rather than head-owned state repair.
- [x] Head and matchmaking tests cover successful cancel, duplicate cancel, and
      unknown-ticket behavior.

---

## Phase 5: Regional Game-Server Routing

**User stories**: 11, 12, 13, 14

> Detailed implementation plan:
> `plans/strict-regional-routing-shared-config.md`

### What to build

Introduce a shared region catalog plus a game-server registry abstraction in
head that resolves the target game server for a region. The first
implementation is configuration-backed through a repo-level shared
`regions.toml` artifact that defines the canonical allowed region set and the
per-region game-server routing data used by head.

This phase keeps the client contract stable while making regional expansion a
real part of the backend design instead of an undocumented assumption. The
same matchmaking and handoff flow continues to work, but region policy is now
explicit and validated rather than relying on a default fallback path.

### Acceptance criteria

- [ ] Head resolves game-server destination by region through a dedicated
      abstraction rather than hard-coded branching.
- [ ] The first implementation uses a repo-level shared region config artifact
      rather than head-local ad hoc environment mappings.
- [ ] The matched handoff flow uses the resolved server’s internal base URL for
      lobby creation and returns that server’s public `ws_url` to the client.
- [ ] Region matching is exact and does not normalize or alias region strings.
- [ ] Any shared-region-config misconfiguration fails startup rather than
      degrading to a default route.
- [ ] Every valid region is declared explicitly; `global` is not modeled as a
      region.
- [ ] Tests cover exact region resolution and startup-time config validation
      failures.
- [ ] The system remains demoable even when all configured regions point to the
      same current global game server.
