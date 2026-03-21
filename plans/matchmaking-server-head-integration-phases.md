# Plan: Matchmaking Server Head Integration Phases

> Source artifacts:
> `head_server/PRD.md`
> `plans/head-server-matchmaking-and-regional-lobby-orchestration.md`
> `plans/stable-matched-ticket-lifecycle-and-idempotent-handoff.md`

## Architectural decisions

Durable decisions that apply across all phases:

- **Service role**: Matchmaking remains the source of truth for queue and
  ticket state. Head consumes matchmaking contracts and does not mirror queue
  ownership locally.
- **Caller model**: Matchmaking is a head-facing backend service, not a
  direct client-facing API.
- **Queue handle**: `ticket_id` is the v1 lookup and cancellation handle for a
  queued player.
- **Ticket creation**: Every enqueue creates a ticket record. Enqueue returns
  `ticket_id` even when the caller is matched immediately.
- **Lifecycle model**: A ticket can move from `waiting` to `matched` or from
  `waiting` to `canceled`.
- **Terminal state visibility**: `matched` remains queryable for the process
  lifetime in v1. `canceled` remains queryable until the same player enqueues
  again, at which point old canceled tickets may be discarded and return
  `404`.
- **Re-enqueue semantics**: Re-enqueue while `waiting` returns the existing
  waiting ticket. Re-enqueue while `matched` returns the existing matched
  result. Re-enqueue after `canceled` creates a fresh waiting ticket and may
  purge old canceled tickets for that player.
- **Matched payload**: A matched status response must include the
  authoritative `ticket_id`, `match_id`, full `player_ids` roster, and
  `region`.
- **Dual-ticket resolution**: Once two players are paired, both tickets must
  resolve to the same matched result.
- **Response model**: Enqueue and status lookup share the same unified success
  response shape, populated according to `status`.
- **Identifier model**: `player_id` is numeric `u64` internally.
  `ticket_id` remains an opaque string externally, but should use a stricter
  internal value type in application code.
- **Current implementation direction**: The existing `VecDeque + HashMap`
  in-memory model should be evolved incrementally rather than replaced
  wholesale.
- **Authoritative in-memory model**: The waiting queue stores only `TicketId`
  ordering. Authoritative state is held in maps:
  `tickets_by_id`, `active_ticket_by_player`, and `matches_by_id`.
- **Canonical matched state**: Shared matched data should be stored once in a
  canonical match record keyed by `match_id`. Matched tickets refer to that
  record rather than duplicating the full matched payload.
- **Ticket state shape**: The target application model is:
  `Waiting { player_id, player_skill, region }`,
  `Matched { player_id, match_id }`,
  `Canceled { player_id, region }`.
- **Match state shape**: The target match record is:
  `MatchRecord { match_id, player_ids, region }`.
- **Atomicity**: Match formation must update both tickets and the shared match
  record atomically within the in-memory critical section. Partial matched
  state must not be externally visible.
- **Observability**: Log ticket creation, ticket matched transitions, and
  ticket cancellation. Do not log normal polling traffic.
- **Storage model**: In-memory state is acceptable for this slice as long as
  the lifecycle model supports queue entry, lookup, match transition, and
  cancellation consistently.
- **End-to-end test ownership**: Rust test suites should cover matchmaking
  use-case and adapter behavior. Cross-service end-to-end coverage through
  head is deferred to a Bruno collection maintained separately from the Rust
  test harness.
- **Out of scope**: Durable storage, auth middleware, queue expiry, and push
  notifications are not part of this plan.

## Proposed Bruno test paths

Suggested paths for user-authored end-to-end coverage through head:

- `bruno/head-matchmaking/01-enqueue-waiting.bru`
- `bruno/head-matchmaking/02-poll-waiting.bru`
- `bruno/head-matchmaking/03-immediate-match-returns-ticket-and-match.bru`
- `bruno/head-matchmaking/04-waiting-ticket-transitions-to-matched.bru`
- `bruno/head-matchmaking/05-reenqueue-while-matched-returns-matched.bru`
- `bruno/head-matchmaking/06-cancel-waiting-ticket.bru`
- `bruno/head-matchmaking/07-cancel-matched-ticket-rejected.bru`
- `bruno/head-matchmaking/08-unknown-ticket-returns-404.bru`

These paths are a proposed layout only. The main intent is that the Bruno
suite exercises the real head-facing flow while Rust tests continue to protect
matchmaking's internal lifecycle and HTTP contract.

---

## Phase 1: Ticket-Centric Queue State

**Supports head capabilities**: queue entry, later polling by `ticket_id`

### What to build

Refactor matchmaking from a waiting-player queue that only answers enqueue
requests into a ticket-centric lifecycle model. Enqueue should still attempt
immediate matching, but the service must now persist enough state to answer
future status lookups by `ticket_id`.

This phase creates the core internal model that every later head-facing
contract depends on.

### Acceptance criteria

- [ ] Matchmaking issues a `ticket_id` for waiting players and stores ticket
      state explicitly.
- [ ] Every enqueue creates a ticket record, even when the caller is matched
      immediately.
- [ ] The internal model distinguishes waiting queue order from authoritative
      ticket state.
- [ ] The waiting queue stores `TicketId` order while authoritative ticket
      data lives in maps.
- [ ] Re-enqueue while already `waiting` returns the existing waiting ticket as
      a normal success-shaped response.
- [ ] Matchmaking tests cover ticket creation and waiting-state re-enqueue
      recovery behavior.

---

## Phase 2: Ticket Status Lookup API

**Supports head capabilities**: polling waiting state through head

### What to build

Add a head-consumable status lookup endpoint keyed by `ticket_id`. This route
must expose the current ticket lifecycle state without requiring head to infer
or reconstruct queue ownership.

This phase completes the first polling slice needed by head: a queued player
can enter matchmaking and later check whether the ticket is still waiting.

### Acceptance criteria

- [ ] Matchmaking exposes a status lookup contract keyed by `ticket_id`.
- [ ] Waiting tickets return `status: waiting`, `ticket_id`, and `region`.
- [ ] Lookup and enqueue share the same unified response shape.
- [ ] Unknown tickets return a consistent `404` not-found outcome.
- [ ] Adapter tests cover request validation and error mapping for ticket
      lookups.
- [ ] Use-case tests cover successful waiting lookups and unknown-ticket
      behavior.

---

## Phase 3: Stable Matched Ticket Results

**Supports head capabilities**: polling until a match is ready

### What to build

Extend match formation so that when two players are paired, matchmaking stores
an authoritative matched result rather than returning a one-off response only
to the second enqueue caller. Both tickets should transition into the same
stable matched state that can be queried repeatedly.

This phase gives head the durable match metadata it needs before attempting
lobby handoff.

### Acceptance criteria

- [ ] When a compatible second player arrives, matchmaking creates one
      canonical matched record for both tickets.
- [ ] The immediate-match caller still receives a `ticket_id` in the enqueue
      response.
- [ ] Both tickets resolve to the same `status: matched` response.
- [ ] The matched response includes `ticket_id`, `match_id`, `player_ids`, and
      `region`.
- [ ] Repeated lookups of the same matched ticket return the same stable
      payload.
- [ ] Re-enqueue while already matched returns a normal `matched` response with
      the existing match data.
- [ ] Matchmaking tests cover the transition from waiting to matched for both
      participants and the active-match recovery path.

---

## Phase 4: Ticket Cancellation

**Supports head capabilities**: cancel queueing through head

### What to build

Add queue cancellation by `ticket_id` for tickets that are still waiting.
Cancellation should remove the ticket from active queue consideration and make
later lookups reflect that the ticket is no longer active.

This phase closes the negative-control path for head without changing queue
ownership.

### Acceptance criteria

- [ ] Matchmaking exposes cancellation by `ticket_id`.
- [ ] Canceling a waiting ticket removes it from future match consideration.
- [ ] Successful cancel transitions the ticket to explicit `status: canceled`.
- [ ] Canceling an unknown ticket returns a consistent not-found outcome.
- [ ] Canceling a matched ticket is rejected because the player already has an
      active assignment.
- [ ] Fresh enqueue after cancel creates a new waiting ticket and may discard
      all old canceled tickets for that player.
- [ ] Matchmaking tests cover successful cancel, cancel-after-match rejection,
      re-enqueue-after-cancel replacement, and unknown ticket cases.

---

## Phase 5: Contract Hardening for Head Handoff

**Supports head capabilities**: reliable handoff preparation in head

### What to build

Harden the matchmaking contract so the matched status shape is reliable for
head orchestration. This phase is still matchmaking-only work: it focuses on
response stability, invariants, and test coverage that make head-side lobby
handoff safe to implement.

This phase prepares matchmaking for the head phase that creates lobbies using
`match_id` and `player_ids`.

### Acceptance criteria

- [ ] The matched payload shape is documented and stable for head consumers.
- [ ] The service guarantees that both matched tickets expose identical
      `match_id`, `player_ids`, and `region` values.
- [ ] Tests cover repeated matched lookups as a retry-safe contract for head.
- [ ] Tests cover the invariant that a canceled waiting ticket cannot later
      become matched.
- [ ] Tests cover the canonical lifecycle sequence:
      waiting enqueue, immediate match for the second player, matched poll for
      the first player, matched re-enqueue recovery, and cancel rejection after
      match.
- [ ] Plan-adjacent docs call out the intended in-memory model:
      `waiting_queue`, `tickets_by_id`, `active_ticket_by_player`, and
      `matches_by_id`.
- [ ] README or plan-adjacent docs describe the head-facing matchmaking
      lifecycle contract.
