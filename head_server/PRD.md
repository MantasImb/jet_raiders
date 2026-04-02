# Head Server Matchmaking and Regional Lobby Orchestration

## Problem Statement

The head server is intended to be the entry point for the Jet Raiders platform,
and it currently supports guest initialization, guest login, and matchmaking
queue entry by proxying those flows to the auth and matchmaking services.

The game client still needs a single platform-facing service that can handle
the full pre-game flow: authenticate as a guest, enter matchmaking, wait for a
match, cancel queueing if needed, and receive enough information to connect to
the correct game server lobby when a match is ready.

At the same time, the system is moving toward regional game servers. Even
though only one global game server exists today, the head server needs a
defined model for selecting the correct game server for a match and creating a
lobby there before the client joins.

Without this capability, the client would need to understand too many internal
service boundaries, matchmaking would not have a complete user-facing flow,
and future regional expansion would be harder to add cleanly.

## Solution

Expand the head server from a guest-auth entrypoint into the platform
orchestration layer for the game client's pre-game flow.

The head server will continue to proxy guest identity and guest login through
the auth service. It also exposes the first matchmaking orchestration endpoint
for the game client:

- enter matchmaking
- poll matchmaking status by `ticket_id`
- cancel matchmaking by `ticket_id`

The matchmaking service remains the source of truth for queue state. The head
server does not own matchmaking state; instead, it proxies queue creation,
queue status checks, and queue cancellation to the matchmaking service.

When the matchmaking service reports a completed match, the head server will:

1. resolve the appropriate game server for the match region
2. create a lobby on that game server for the matched players
3. return the final join information to the client

The final matched response from head should include the connection payload the
client needs to stop polling and proceed to gameplay, including:

- `ticket_id`
- `match_id`
- `lobby_id`
- `ws_url`
- `region`

The waiting response should remain minimal and include:

- `status: waiting`
- `ticket_id`
- `region`

The head server will also introduce a game-server routing model through a
registry abstraction. The first implementation will be configuration-backed and
will require explicit concrete region mappings. This keeps regional routing
inside head while allowing the implementation to evolve later into dynamic
service discovery without relying on implicit fallback behavior.

## User Stories

1. As a first-time guest player, I want the game client to initialize my guest
   identity through the head server, so that I can start playing without
   understanding internal backend services.
2. As a returning guest player, I want the game client to log me in through the
   head server, so that I can receive a fresh session token for gameplay.
3. As an authenticated player, I want to enter matchmaking through the head
   server, so that the client has a single platform-facing API for pre-game
   flow.
4. As a queued player, I want to receive a queue ticket when no immediate match
   is available, so that I can poll for progress.
5. As a queued player, I want to poll the head server with my `ticket_id`, so
   that I can learn whether I am still waiting or have been matched.
6. As a queued player, I want the head server to proxy queue status checks to
   the matchmaking service, so that matchmaking remains the source of truth for
   queue state.
7. As a matched player, I want the poll response to contain final connection
   details, so that I can connect to gameplay without extra API calls.
8. As a matched player, I want the response to include `match_id`, `lobby_id`,
   `ws_url`, and `region`, so that the client can stop polling and join the
   correct lobby.
9. As a matched player, I want the lobby to already exist by the time I receive
   the matched response, so that I do not race the backend while connecting.
10. As a player, I want to cancel matchmaking through the head server, so that
    I can leave the queue when I no longer want to wait.
11. As the platform, I want the head server to route match handoff to the
    correct regional game server, so that regional scaling can be introduced
    without changing the client contract.
12. As an operator, I want regional game-server routing to be configuration
    backed at first, so that the system can declare each supported region
    explicitly and still support regional expansion later.
13. As a backend developer, I want clean separation between orchestration,
    transport mapping, and service clients, so that the head server remains
    aligned with repository clean-architecture rules.
14. As a future website client, I want head to remain the platform boundary, so
    that future web-facing flows can be added without exposing internal
    services directly.

## Implementation Decisions

- The head server remains the platform entrypoint and orchestration boundary
  for client-facing pre-game flows.
- Guest init and guest login remain in scope and continue to proxy to the auth
  service.
- Session check is explicitly out of scope for this version because there is no
  current product flow that needs it.
- Matchmaking state ownership remains in the matchmaking service. Head acts as
  a proxy and orchestration layer, not a queue-state store.
- Head currently exposes a client-facing endpoint to enter matchmaking.
- Head will later expose client-facing endpoints to poll matchmaking status
  using only `ticket_id` and to cancel matchmaking using `ticket_id`.
- `ticket_id` is treated as an opaque queue handle and effectively acts as a
  bearer secret for poll and cancel in v1.
- The matchmaking service contract will need to expand beyond queue entry to
  also support:
  - queue status lookup by `ticket_id`
  - queue cancellation by `ticket_id`
- When polling returns `waiting`, head returns a minimal waiting response and
  does not fabricate additional state.
- When enqueue or polling returns `matched`, head must complete match handoff
  before replying successfully to the client.
- Match handoff means:
  - resolve target game server by region
  - create lobby on that game server
  - return final join payload to the client
- The matched response from head includes:
  - `status`
  - `ticket_id`
  - `match_id`
  - `lobby_id`
  - `ws_url`
  - `region`
- `ticket_id` in the matched response is preserved for traceability/debugging
  only and does not carry new post-handoff behavior.
- Handoff may be triggered by either enqueue or poll. The first request that
  observes `matched` attempts lobby creation, and later matched requests retry
  the same idempotent handoff.
- Game-server `409 already exists` is treated as a successful handoff outcome
  for the requested `lobby_id`.
- The waiting response from head includes:
  - `status`
  - `ticket_id`
  - `region`
- Head needs a dedicated game-server routing abstraction, described as a
  `GameServerRegistry` style module, whose responsibility is to resolve the
  correct game server for a region.
- The first game-server registry implementation is config-backed.
- The config-backed registry should support:
  - region key
  - internal base URL for head-to-game-server API calls
  - public WebSocket URL returned to the client
- Unknown regions must fail handoff rather than falling back to an implicit
  default route.
- The initial deployment can point multiple explicit regions to the current
  single game server.
- Head also needs a game-server client module that creates lobbies on the
  selected game server.
- The existing game-server lobby creation flow already supports head-driven
  lobby creation with a lobby identifier and allowed player identifiers. The
  PRD assumes head will use that style of contract.
- Head needs a matchmaking client module separate from the auth client so that
  auth integration and matchmaking integration remain isolated behind
  dedicated ports. That client is implemented for queue entry.
- The head service should introduce head-specific application workflows for:
  - guest auth orchestration
  - matchmaking entry
  - matchmaking polling
  - matchmaking cancellation
  - match handoff and lobby creation
- Domain modeling inside head should remain minimal but can introduce value
  objects or domain entities where they simplify invariants around:
  - queue ticket handling
  - matched connection payload construction
  - regional game-server selection
- Transport DTOs remain in adapter layers and must not become canonical domain
  types.
- The client contract should preserve the current architectural intent that the
  game client talks to head for platform flows and to the game server only for
  real-time gameplay transport.
- Error handling should preserve the current head-service behavior where
  upstream validation-like failures are translated cleanly and upstream
  transport/integration failures surface as gateway-style failures.
- Lobby creation failure after a matchmaking match is a critical handoff error.
  Head must not return a successful matched response unless lobby creation
  succeeded.
- Regional routing is modeled now, but dynamic discovery and fleet management
  are deferred. The first implementation is static configuration with a clean
  replacement path.

## Testing Decisions

- Good tests should verify externally visible behavior and boundary contracts,
  not implementation details or internal call ordering beyond what is required
  by the public behavior.
- Use-case tests in head should mock ports for auth, matchmaking, and
  game-server routing/creation, following the repository rule that use cases
  depend on small interfaces and are tested through those interfaces.
- Adapter tests in head should verify:
  - request validation
  - HTTP status mapping
  - DTO conversion
  - response shape for waiting and matched queue-entry flows
- Framework tests in head should stay lightweight and focus on wiring and
  configuration only.
- The most important head use-case tests should cover:
  - guest init delegates correctly to auth
  - guest login delegates correctly to auth
  - enter matchmaking delegates correctly to matchmaking
  - poll matchmaking returns waiting without attempting lobby creation
  - matched enqueue or poll returns success only after successful lobby
    creation
  - matched head response includes `ticket_id`, `match_id`, `lobby_id`,
    `ws_url`, and `region`
  - cancel matchmaking delegates correctly to matchmaking
  - queue entry response preserves `ticket_id` and `region` when waiting
  - queue entry response surfaces immediate matched outcomes cleanly
  - unknown regional mapping surfaces a handoff error instead of fallback
  - duplicate lobby create (`409`) is treated as successful retry-safe handoff
  - lobby creation failure prevents a successful matched response and retries
    on later matched requests
- Matchmaking-service tests should expand to cover:
  - status lookup by `ticket_id`
  - cancel by `ticket_id`
  - correct lifecycle from waiting to matched or canceled
- Prior art for these tests already exists in the repository:
  - head service use-case tests with mocked ports
  - head service adapter tests that verify HTTP validation and error mapping
  - auth service route and use-case tests for token-oriented flows
  - matchmaking service tests around queue outcomes
- Integration-oriented tests are especially valuable around the head
  matchmaking handoff because that flow crosses multiple service boundaries and
  is the highest-risk orchestration path.

## Out of Scope

- Session check endpoint in head
- Website-specific endpoints or website session flows
- Friends, parties, inventory, or profile-management features
- Wallet or external identity login
- Dynamic service discovery for regional game servers
- Persistent or distributed matchmaking storage
- Push-based match delivery such as WebSocket or SSE notifications
- Game-server gameplay protocol changes beyond the existing lobby-creation
  contract
- Security hardening beyond the v1 `ticket_id` bearer-handle model
- Multi-region fleet operations, autoscaling, or control-plane management

## Further Notes

The current repository already establishes the architectural intent that the
head service is the platform-facing orchestration layer for auth and
matchmaking. This PRD turns that intent into a concrete v1 feature set.

The most important design constraint is to keep responsibilities clean: auth
owns session issuance and verification, matchmaking owns queue state, game
servers own gameplay lobbies, and head owns orchestration plus client-facing
contract stability.

The key v1 simplification is that the client polls head with `ticket_id` until
the result is either still waiting, matched with final connection details, or
canceled/terminated. This keeps the client simple while avoiding premature
real-time queue-notification infrastructure.

The regional routing model should be written so that replacing static
configuration with a real discovery mechanism later does not require changing
the client-facing API.
