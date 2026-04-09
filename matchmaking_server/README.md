# Matchmaking Service

## Purpose

The matchmaking service groups players into matches based on simple queue rules
and owns the authoritative ticket lifecycle consumed by the head service.

## Architecture Guidelines

Follow `CLEAN_ARCHITECTURE_GUIDELINES.md` for layering and dependency rules.

## Responsibilities

- Accept matchmaking requests from clients via the head service.
- Group players based on simple queue rules (region-only today).
- Return stable lifecycle state for issued tickets, including waiting,
  matched, and canceled outcomes.

## Queue Flow (Player -> Match -> Head Service)

This flow describes how a player is queued and how the head service is told a
match is ready. It reflects the current simple in-memory implementation, while
leaving room for additional matchmaking rules later.

1. **Request received**: The head service sends `POST /matchmaking/queue` with
   `player_id`, `player_skill`, and `region`.
2. **Request validated**: The matchmaking handler checks required fields before
   enqueueing.
3. **Queue evaluation**: The matchmaker looks for a waiting player in the same
   region. Matching rules are intentionally minimal for now (region only).
4. **Match found**:
    - The waiting player is removed from the active queue.
    - A `match_id` is created for the two players.
    - Both players receive their own stable `ticket_id`.
    - Both tickets resolve to the same stored matched payload.
5. **No match yet**:
    - The player is stored in the in-memory queue as a waiting ticket.
    - A `ticket_id` is issued so the head service can track the request.
    - The service responds with `status: waiting`.
6. **Polling**:
    - The head service can later call
      `GET /matchmaking/queue/{ticket_id}?player_id=<owner_id>`.
    - A queued ticket returns `status: waiting` plus the same `ticket_id`.
    - A matched ticket returns `status: matched` plus the stable
      `ticket_id`, `match_id`, `player_ids`, and `region` payload.
7. **Cancellation**:
    - The head service can call
      `DELETE /matchmaking/queue/{ticket_id}?player_id=<owner_id>`.
    - Waiting tickets transition to `status: canceled`.
    - Matched tickets reject cancellation with `409`.
8. **Head service response**: The head service receives the response and either
   notifies the client immediately (matched) or keeps the client polling until
   later orchestration phases complete the final handoff.

## In-Memory Lifecycle Model

The current implementation keeps queue ordering separate from authoritative
ticket and match state:

- `queue: VecDeque<String>` stores waiting `ticket_id` order only.
- `tickets_by_id: HashMap<String, TicketRecord>` stores the authoritative
  lifecycle state for each ticket.
- `active_ticket_by_player: HashMap<u64, String>` tracks the current active
  ticket for each player so re-enqueue can return the existing waiting or
  matched result.
- `matches_by_id: HashMap<String, MatchRecord>` stores the canonical matched
  payload once per match.

This means matched tickets do not duplicate the full roster payload in their
ticket record. Instead, each matched ticket stores `match_id`, and lookup
resolves the shared `MatchRecord` by that key.

The current ticket lifecycle model is:

- `Waiting { player_id, player_skill, region }`
- `Matched { player_id, match_id }`
- `Canceled { player_id, region }`

The current canonical match record is:

- `MatchRecord { match_id, player_ids, region }`

Match formation updates both ticket records and the shared match record inside
the same in-memory critical section so partially matched state is not exposed.

## External Interfaces

### HTTP API

- `POST /matchmaking/queue`
  - Enqueues a player for matchmaking.
  - Returns a queue ticket or match assignment.
- `GET /matchmaking/queue/{ticket_id}`
  - Looks up the current status of an issued queue ticket.
  - Requires the owning `player_id` as a query parameter for head-scoped
    authorization.
  - Returns the waiting, matched, or canceled state for that ticket.
- `DELETE /matchmaking/queue/{ticket_id}`
  - Cancels a waiting queue ticket.
  - Requires the owning `player_id` as a query parameter for head-scoped
    authorization.
  - Returns the canceled state for that ticket.
- `GET /health`
  - Liveness endpoint for startup and container smoke checks.

## Data Contracts

### Queue Request

- `player_id`: numeric player identifier supplied by the head service.
- `player_skill`: matchmaking rating or tier.
- `region`: preferred region.

### Queue Response

- `status`: `waiting`, `matched`, or `canceled`.
- `ticket_id`: queue ticket identifier for all lifecycle responses.
- `match_id`: match identifier when matched.
- `player_ids`: full matched roster when matched.
- `region`: the region used for matchmaking.

### Ticket Errors

- Unknown `ticket_id` values return `404`.
- Owner mismatches for lookup or cancel return `401`.
- Canceling a matched `ticket_id` returns `409`.

## Security Considerations

- Authenticate requests at the head service before calling matchmaking.
- Head forwards the canonical owning `player_id` on lookup and cancel so the
  service can reject cross-user ticket access.
- Avoid leaking internal server addresses to unauthenticated clients.

## Dependencies

- Head service for client entry and validation.
- In-memory queue storage (current implementation).

## Shared Region Catalog

The repository-level shared region catalog lives at `config/regions.toml`.
Both head and matchmaking load this file at startup via required
`REGION_CONFIG_PATH`.

Matchmaking uses the catalog to validate queue-entry `region` values against
the declared concrete `matchmaking_key` set before storing ticket or match
state. Region matching is exact: the service does not trim, lowercase, alias,
or invent region values.

Startup fails fast if the shared config is missing, unreadable, malformed,
empty, has duplicate `matchmaking_key` values, omits required fields, or
contains invalid game-server URLs.

## Runtime and Configuration

- Required bind host env var: `MATCHMAKING_SERVER_BIND_HOST`
- Bind address: `<MATCHMAKING_SERVER_BIND_HOST>:3003`
- Required shared region config env var: `REGION_CONFIG_PATH`
- Tracing controls: `RUST_LOG`, optional `LOG_FORMAT=json`

Fatal startup exit codes:

- `1`: required startup config is missing.
- `2`: startup config is invalid.
- `3`: startup dependency initialization failed.
- `4`: service failed to bind its listener socket.
- `5`: server runtime failed while serving requests.

## Docker

Build the matchmaking image from the repository root:

```bash
docker build -f matchmaking_server/Dockerfile \
  -t jet-raiders/matchmaking-server:phase2 .
```

Run the matchmaking container:

```bash
docker run --rm \
  --name matchmaking-server \
  -p 3003:3003 \
  -e MATCHMAKING_SERVER_BIND_HOST=0.0.0.0 \
  -e REGION_CONFIG_PATH=/app/config/regions.toml \
  jet-raiders/matchmaking-server:phase2
```

Smoke-check liveness from another terminal:

```bash
curl -sS http://127.0.0.1:3003/health
```

Expected response:

```json
{"status":"ok"}
```

Negative-path check for required bind-host config:

```bash
docker run --rm \
  --name matchmaking-server-missing-bind \
  -e REGION_CONFIG_PATH=/app/config/regions.toml \
  jet-raiders/matchmaking-server:phase2
echo $?
```

Expected outcome:

- Logs include `MATCHMAKING_SERVER_BIND_HOST`.
- Process exits with status code `1`.

## Observability

- Log ticket creation, match transitions, and cancellation with correlation IDs.
