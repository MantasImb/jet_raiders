# Plan: Backend Service Dockerfiles and Runtime Port Configuration

> Source PRD: PRD/backend-service-dockerfiles-prd.md

## Architectural decisions

Durable decisions that apply across all phases:

- **Service scope**: Only backend services are in scope: auth, head,
  matchmaking, and game service.
- **Out of scope**: Godot client and web client are excluded.
- **Container packaging**: Each backend service gets its own production-oriented
  multi-stage Dockerfile with a shared convention.
- **Runtime config precedence**: Shared config defaults in `config` and explicit
  environment variable overrides at runtime.
- **Ports**: Auth/head/matchmaking listener ports are centralized in a shared
  backend port catalog in `config` after initial Docker baseline is
  established. Game-server routing endpoints remain in `config/regions.toml`.
- **Networking**: Containerized services must not depend on localhost-only
  cross-service assumptions.
- **Auth/runtime invariants**: Auth must continue to enforce required database
  configuration and run migrations at startup.
- **Region invariants**: Head and matchmaking keep strict shared-region config
  validation and fail fast on invalid catalog input.
- **Architecture boundaries**: Runtime/bootstrap/container/config wiring remains
  in frameworks/adapters; domain and use-case behavior should not be changed
  except where required for container runtime compatibility.
- **Verification strategy**: Prefer thin, behavior-focused vertical slices with
  demoable build/start/smoke outcomes per phase.

---

## Phase 1: Docker Baseline for One Service (Auth)

**User stories**: 1, 2, 3, 4, 7, 8, 14

### What to build

Create the first end-to-end containerization slice using the auth service as the
tracer bullet. Deliver one production-style Dockerfile, runtime contract
documentation, and smoke verification for build and startup behavior with DB
requirements and migration-on-start semantics.

### Acceptance criteria

- [ ] Auth service Dockerfile builds successfully with a multi-stage flow.
- [ ] Auth container starts with required runtime env and exposes expected
      service port.
- [ ] Missing database configuration fails fast with clear startup failure.
- [ ] Migration-on-start behavior remains intact in container runtime.
- [ ] A concise runbook exists for building and running the auth image.

---

## Phase 2: Docker Baseline for Remaining Backend Services

**User stories**: 1, 2, 3, 4, 9, 10, 11, 14

### What to build

Extend the Docker convention to head, matchmaking, and game services so all
backend services can be built and run as containers with consistent image shape,
entrypoint conventions, and runtime env contracts.

### Acceptance criteria

- [ ] Head, matchmaking, and game service Dockerfiles build successfully.
- [ ] Each service container starts and exposes its expected API surface.
- [ ] Existing upstream env contracts remain usable in container runtime.
- [ ] Region-catalog loading for head/matchmaking remains strict and fail-fast.
- [ ] Per-service container build and startup smoke checks are documented.

---

## Phase 3: Shared Port Catalog for Auth/Head/Matchmaking

**User stories**: 5, 6, 9, 10, 11, 12, 14

### What to build

Introduce a shared backend service-port catalog in `config` for auth, head, and
matchmaking listener ports, while preserving service-specific environment
overrides as explicit runtime precedence for Docker and CI deployments.

`game_server` listener port configuration is intentionally excluded from this
phase and continues to use `GAME_SERVER_PORT`. Region handoff endpoints remain
in `config/regions.toml`.

### Acceptance criteria

- [ ] Shared port catalog exists at `config/backend_ports.toml` with:
      `ports.auth_server`, `ports.head_server`, and
      `ports.matchmaking_server`.
- [ ] Auth/head/matchmaking read default listener ports from
      `config/backend_ports.toml` when no service override env var is provided.
- [ ] Override env vars are supported and standardized:
      `AUTH_SERVER_PORT`, `HEAD_SERVER_PORT`, and
      `MATCHMAKING_SERVER_PORT`.
- [ ] Effective precedence is `*_PORT` env override > shared file value.
- [ ] Override usage emits a structured warning log with relevant fields.
- [ ] Invalid non-empty override values fail fast as
      `InvalidConfiguration` (exit code `2`).
- [ ] Exact empty-string override values (`""`) are treated as unset
      placeholders and fall back to file loading.
- [ ] `BACKEND_PORTS_CONFIG_PATH` acts as an optional path override.
- [ ] When `BACKEND_PORTS_CONFIG_PATH` is not set, services try canonical
      defaults (`../config/backend_ports.toml` locally, then
      `/app/config/backend_ports.toml` in containers).
- [ ] Missing service key in the loaded file fails fast as
      `InvalidConfiguration` (exit code `2`).
- [ ] Services skip backend-ports file loading when a valid `*_PORT` override
      is present.
- [ ] `game_server` runtime port behavior remains unchanged in code during this
      phase.
- [ ] Documentation explicitly states that `GAME_SERVER_PORT` must align with
      the game-server endpoint ports declared in `config/regions.toml` for
      local single-node setups.
- [ ] Config parsing and precedence behavior is covered by focused tests for
      auth/head/matchmaking.

### Phase 3 execution plan (detailed)

1. Add shared catalog artifact
   - Create `config/backend_ports.toml` with `[ports]` entries for auth, head,
     and matchmaking defaults.
   - Keep catalog intentionally minimal and listener-port focused.
2. Implement auth service port-source precedence
   - Add config parsing in auth frameworks config/runtime wiring.
   - Resolve listener port from `AUTH_SERVER_PORT` override first, then file.
   - Enforce startup-failure semantics for missing/invalid config.
   - Emit structured warning when override is used.
3. Implement head service port-source precedence
   - Add equivalent parsing and precedence behavior in head runtime config.
   - Preserve existing upstream URL and region-catalog behavior.
   - Emit structured warning when override is used.
4. Implement matchmaking service port-source precedence
   - Add equivalent parsing and precedence behavior in matchmaking runtime
     config.
   - Preserve existing strict region-catalog loading and validation.
   - Emit structured warning when override is used.
5. Keep game server behavior unchanged
   - Do not refactor `game_server` listener-port source in this phase.
   - Preserve current `GAME_SERVER_PORT` runtime behavior.
6. Update runtime and container docs
   - Document default container path `/app/config/backend_ports.toml`.
   - Document `BACKEND_PORTS_CONFIG_PATH` and the `*_PORT` override contract.
   - Document game-server/region-port alignment requirement.
7. Add focused tests and smoke checks
   - Per service: file default load, env override precedence, empty-string
     placeholder handling, invalid override failure, missing path failure, and
     missing-key failure.
   - Run service-level checks and keep outcomes easy to reproduce.

---

## Phase 4: Backend Container Orchestration and Connectivity Smoke

**User stories**: 5, 13, 14

### What to build

Add backend-only container orchestration for local validation and run an
end-to-end smoke flow that verifies cross-service connectivity and core startup
contracts.

### Acceptance criteria

- [ ] Backend-only container stack can be launched with one documented command.
- [ ] Services can resolve each other through container networking.
- [ ] Cross-service smoke flow passes for auth, head, matchmaking, and game
      integration touchpoints.
- [ ] Critical misconfiguration scenarios fail predictably and are documented.

---

## Phase 5: Documentation and CI-Oriented Verification Contract

**User stories**: 13, 14, 15

### What to build

Finalize the operational documentation and define a repeatable verification
contract that can be run locally and in CI for build/start/smoke confidence.

### Acceptance criteria

- [ ] Developer docs cover per-service image build, env setup, and run commands.
- [ ] Operator docs cover startup dependencies, failure modes, and troubleshooting
      basics.
- [ ] Verification commands for per-service build/start and stack smoke are
      documented in a CI-friendly sequence.
- [ ] Scope boundaries (backend-only, client exclusions) are explicit in final
      docs.
