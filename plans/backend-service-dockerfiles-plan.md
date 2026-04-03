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
- **Ports**: Service ports are centralized in a shared backend port catalog in
  `config` after initial Docker baseline is established.
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

## Phase 3: Shared Backend Port Catalog and Override Precedence

**User stories**: 5, 6, 9, 10, 11, 12, 14

### What to build

Introduce a shared backend service-port catalog in `config` and move services to
use it as the default source of port truth, while preserving environment
variables as explicit runtime overrides for Docker and CI deployments.

### Acceptance criteria

- [ ] Shared port catalog is defined for all backend services.
- [ ] Backend services read default ports from shared config.
- [ ] Environment overrides take precedence over shared config defaults.
- [ ] Bind-host behavior is container-compatible and does not force localhost-only
      networking.
- [ ] Config parsing and precedence behavior is covered by focused tests.

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
