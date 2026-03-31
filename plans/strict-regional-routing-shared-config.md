# Plan: Strict Regional Routing and Shared Region Config

> Source plan:
> `plans/head-server-matchmaking-and-regional-lobby-orchestration.md`

## Objective

Make regional routing explicit, shared, and validation-driven so head can
handoff matches only to intentionally configured regional game servers and the
same region catalog can later be reused by other services and Docker-based
orchestration.

## Scope

This plan covers:

- A repo-level shared region config artifact.
- Head startup/config changes required to load that artifact and resolve game
  servers by exact region.
- Strict validation rules for malformed or incomplete routing config.
- A clean reuse path for matchmaking to consume the same region catalog later.

This plan does not cover:

- Dynamic game-server discovery.
- Runtime config reload.
- Per-region capacity management or fleet operations.
- Docker wiring itself.

## Architectural decisions

- **Canonical region catalog**: The allowed region set is defined once in a
  repo-level shared config file, likely `config/regions.toml`.
- **Concrete regions only**: Region values must be concrete names such as
  `eu-west` or `us-east`. `global` is not a valid region.
- **Strict matching**: Region strings are matched exactly. Head does not trim,
  lowercase, or otherwise normalize region values.
- **Strict startup validation**: Any configuration defect is fatal at startup.
  Head must not continue with partial or fallback routing.
- **No implicit fallback**: A region that is valid in the catalog must have an
  explicit mapping. Silent fallback to a default global route is not allowed.
- **Shared-file ownership**: The shared file is a repository-level artifact,
  not a file owned by a single service directory.
- **Layer ownership**: Parsing and loading the shared config belongs in each
  service's frameworks/config startup layer. Use cases continue to depend on
  abstractions rather than raw config parsing.
- **Single-server bootstrap**: The system may remain single-server in
  deployment, but each concrete region must still be declared explicitly and
  may temporarily point to the same game-server endpoints.

## Proposed shared config shape

Suggested file: `config/regions.toml`

```toml
[regions.eu_west]
matchmaking_key = "eu-west"
game_server_base_url = "http://localhost:3001"
game_server_ws_url = "ws://localhost:3001/ws"

[regions.us_east]
matchmaking_key = "us-east"
game_server_base_url = "http://localhost:3001"
game_server_ws_url = "ws://localhost:3001/ws"
```

Notes:

- The allowed region set is the set of declared region entries.
- `matchmaking_key` is the exact value expected from matchmaking payloads.
- Head consumes the game-server routing fields.
- Matchmaking can later consume the same file for allowed-region validation
  without needing the routing fields itself.

## Validation rules

Head startup should fail if any of the following are true:

- The config file path is missing or unreadable.
- TOML parsing fails.
- No regions are declared.
- A region entry omits `matchmaking_key`, `game_server_base_url`, or
  `game_server_ws_url`.
- Any required field is empty.
- Two entries declare the same `matchmaking_key`.
- Two routing entries use malformed URLs if URL validation is performed during
  startup.

Head routing should also fail if matchmaking later returns a region that is
not present in the loaded catalog. That should be treated as an invariant
break between services, not as a signal to use a fallback route.

## Implementation slices

### Slice 1: Shared config artifact and schema

Add the repo-level config file and define the schema that both services can
read later.

Acceptance criteria:

- [ ] A shared file exists at a repo-level path such as `config/regions.toml`.
- [ ] The file documents at least two concrete regions for local/demo use.
- [ ] The schema distinguishes the external matchmaking region value from the
      game-server endpoint data consumed by head.

### Slice 2: Head startup loads strict shared config

Replace the current ad hoc regional env parsing in head with startup loading of
the shared config file.

Acceptance criteria:

- [ ] Head startup reads the shared region config through the frameworks layer.
- [ ] The existing routing abstraction is constructed from the loaded config
      rather than from `GAME_SERVER_REGION_MAP`.
- [ ] Invalid config terminates startup before the server begins listening.
- [ ] Duplicate region keys are rejected explicitly rather than collapsing in a
      `HashMap`.

### Slice 3: Exact region resolution in handoff

Keep routing resolution behind the existing use-case abstraction, but tighten
the resolution contract to exact-match-only behavior.

Acceptance criteria:

- [ ] A matched handoff uses the exact `region` returned by matchmaking.
- [ ] The resolved server's internal base URL is used for lobby creation.
- [ ] The resolved server's public `ws_url` is returned in the final matched
      head response.
- [ ] Unknown regions returned during runtime surface as errors rather than
      fallback success.

### Slice 4: Tests and observability

Add focused tests around resolution and config validation so rollout failures
are discovered early.

Acceptance criteria:

- [ ] Unit tests cover exact region resolution for at least two configured
      regions.
- [ ] Unit tests cover startup validation failures for malformed config,
      duplicate `matchmaking_key` values, and missing required fields.
- [ ] Existing matched-handoff tests continue to prove that resolved routing
      data drives both lobby creation and client-visible `ws_url`.

### Slice 5: Matchmaking reuse path

Expand phase 5 from a passive reuse note into a concrete matchmaking follow-up:
`matchmaking_server` should load the same shared region catalog and reject
queue-entry regions that are not declared there. Today matchmaking can still
preserve arbitrary client-submitted region strings and return them back to
head, which means head must defensively fail runtime handoff when the region
is not present in its loaded catalog.

Acceptance criteria:

- [ ] The shared config schema is documented clearly enough for
      `matchmaking_server` to reuse later.
- [ ] `matchmaking_server` startup loads the same shared region catalog through
      its frameworks/config layer.
- [ ] Matchmaking queue entry rejects regions that are not declared in the
      shared catalog.
- [ ] Matchmaking does not invent, normalize, or preserve out-of-catalog
      region values in stored ticket or match state.
- [ ] Once matchmaking consumes the shared catalog, head's unknown-region
      runtime path remains as a defensive invariant check rather than a
      routinely reachable validation path.

## File-level implementation outline

- `config/regions.toml`
  - Add the shared region catalog.
- `head_server/src/frameworks/`
  - Add or extend config-loading code for shared region parsing and
    validation.
- `head_server/src/frameworks/server.rs`
  - Replace env-map loading with shared-config startup wiring.
- `head_server/src/frameworks/game_server_directory.rs`
  - Preserve the routing abstraction, but remove any fallback assumptions that
    conflict with strict region mapping.
- `head_server/README.md`
  - Update runtime/config documentation to describe the shared config path and
    the strict validation behavior.
- `matchmaking_server/README.md`
  - Note the intended future reuse of the shared region catalog for request
    validation.
- `matchmaking_server/src/frameworks/`
  - Add shared region config loading for startup validation.
- `matchmaking_server/src/interface_adapters/handlers/`
  - Reject queue-entry requests whose `region` is not in the shared catalog.
- `matchmaking_server/src/use_cases/`
  - Ensure stored ticket and match state only uses validated catalog regions.

## Risks and deferred decisions

- Head will need a clear runtime error shape when matchmaking returns an
  unknown region despite startup validation. That is a runtime invariant break
  rather than a config parse error.
- If local development still needs quick env-only overrides, the repo should
  decide later whether those overrides are generated from `regions.toml` or are
  removed entirely.
- Matchmaking does not yet enforce the shared region catalog. Until that later
  change lands, head still needs defensive runtime handling for unknown
  regions.
- After matchmaking adopts the shared catalog, the repo should decide whether
  head and matchmaking both read the same full file directly or whether a
  narrower shared region-schema helper should be introduced to avoid config
  parsing drift across services.

## Stop condition

This plan is implementation-ready when head can only start with a valid shared
region catalog, exact region routing is enforced end-to-end, and the repo has
a documented shared artifact that other services can consume later without
redefining region policy.
