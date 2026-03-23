# Plan: Stable Matched Ticket Lifecycle and Idempotent Handoff

> Source plan:
> `plans/head-server-matchmaking-and-regional-lobby-orchestration.md`

## Architectural decisions

Durable decisions that apply across all phases:

- **Ticket lifecycle**: `ticket_id` remains a stable lookup handle after a
  match is formed. It is not consumed on first successful delivery.
- **Match state ownership**: Matchmaking remains the source of truth for
  ticket state and matched results. Head does not persist queue or handoff
  state locally.
- **Handoff trigger**: Head may attempt matched handoff on either enqueue or
  poll. The first request that observes `matched` attempts lobby creation, and
  later matched requests retry the same handoff.
- **Matched payload**: A matched ticket lookup returns an authoritative match
  payload that includes `match_id`, `player_ids`, and `region`.
- **Head matched payload**: The final client-visible matched response from
  head includes `ticket_id`, `match_id`, `lobby_id`, `ws_url`, and `region`.
- **Ticket traceability**: `ticket_id` remains in the final matched head
  response for traceability and debugging only. It does not gain new
  post-handoff behavior.
- **Dual-ticket visibility**: Once two players are paired, both players'
  tickets resolve to the same matched payload.
- **Lobby identifier**: Head derives `lobby_id` directly from `match_id`.
- **Idempotent handoff**: Head treats game-server lobby creation `201 created`
  and `409 already exists` as successful handoff outcomes for the same
  `lobby_id`.
- **Roster verification on duplicate create**: Verification of the existing
  lobby roster is deferred. `409 already exists` is treated as definitive
  success for the requested `lobby_id`, and phase 3 relies on the invariant
  that matchmaking-derived lobby IDs are owned by head and are derived from
  authoritative `match_id` values.
- **Game-server contract**: Head creates game lobbies using the existing
  game-server lobby creation endpoint with `lobby_id` and
  `allowed_player_ids`.
- **Out of scope**: Ticket expiry, durable storage, push notifications, and
  game-server readback APIs are not part of this plan.

---

## Phase 1: Stable Matched Ticket State in Matchmaking

**User stories**: Derived from PRD stories 5, 6, 7, 8, 13

### What to build

Extend matchmaking from a waiting-queue-only model into a ticket lifecycle
model that can answer status lookups after a match is formed. When a second
compatible player arrives, matchmaking should transition both tickets into the
same stored matched result instead of only returning a one-off synchronous
match response to the second enqueue caller.

This slice proves the core invariant that a ticket remains queryable after a
match and that both participants can learn about the same match independently.

### Acceptance criteria

- [ ] Matchmaking stores ticket lifecycle state beyond the initial waiting
      queue entry.
- [ ] When two players are paired, both tickets transition to the same matched
      result.
- [ ] A matched ticket lookup returns `status: matched`, `match_id`,
      `player_ids`, and `region`.
- [ ] Repeated lookups of the same matched ticket return the same stable
      matched payload.
- [ ] Matchmaking tests cover waiting-to-matched transitions for both tickets.

---

## Phase 2: Head Polling Uses Stable Matched Results

**User stories**: Derived from PRD stories 5, 6, 7, 8, 13

### What to build

Update the head polling flow so that it depends on the new authoritative
matched ticket payload from matchmaking. Head should remain stateless about
queue ownership and should construct its next action entirely from the status
lookup result: return `waiting` unchanged or continue to handoff preparation
when `matched` is returned.

This slice proves that head can rely on a stable polling contract rather than
one-time delivery semantics.

### Acceptance criteria

- [ ] Head polling can consume a matched status payload that includes the full
      roster via `player_ids`.
- [ ] Head does not need to store queue or matched state locally to handle
      repeated polls.
- [ ] Repeated polls for the same matched ticket produce the same
      matchmaking-derived match metadata prior to game-server handoff.
- [ ] Head use-case and adapter tests cover repeated matched polls without
      introducing local ticket state.

---

## Phase 3: Idempotent Lobby Handoff from Stable Match Results

**User stories**: Derived from PRD stories 7, 8, 9, 13

### What to build

Complete the handoff path from a stable matched ticket result to a game-ready
lobby assignment. Head resolves the target game server, derives
`lobby_id = match_id`, and creates the lobby using the authoritative
`player_ids` roster from matchmaking. The triggering request may be enqueue or
poll, depending on which request first observes `matched`. If the lobby
already exists, head treats that duplicate create as a successful retry and
returns the same final join payload.

This slice delivers retry-safe end-to-end behavior for matched enqueue and
poll flows without adding head-owned persistence or a game-server readback
contract.

### Acceptance criteria

- [ ] Head derives `lobby_id` directly from `match_id`.
- [ ] Head creates the game-server lobby with the matched `player_ids` as
      `allowed_player_ids`.
- [ ] The final matched response from head includes `ticket_id`, `match_id`,
      `lobby_id`, `ws_url`, and `region`.
- [ ] Game-server `201 created` and `409 already exists` are both treated as
      successful handoff outcomes for the same `lobby_id`.
- [ ] Repeated matched enqueue or poll requests can return the same final join
      payload without producing false failures.
- [ ] Handoff failures return errors and continue retrying on later matched
      requests until the lobby handoff succeeds.
- [ ] Head tests cover first-create and duplicate-create handoff behavior.
- [ ] Bruno/process-compose coverage exists for the retry path from matched
      lifecycle response to successful lobby handoff.

---

## Deferred Follow-Ups

- [ ] Add ticket expiry or cleanup rules for long-lived matched tickets.
- [ ] Add explicit terminal states beyond `waiting` and `matched` if product
      flows require them.
- [ ] Add a game-server readback or idempotent-create contract if duplicate
      handoff verification becomes necessary.
- [ ] Add durable matchmaking storage if in-memory lifecycle state becomes an
      operational limitation.
