# Containerize Backend Services with Per-Service Dockerfiles (Excluding Clients)

## Problem Statement

Jet Raiders currently runs backend services directly through local Rust tooling,
which is good for day-to-day coding but does not provide a consistent container
runtime contract per service.

The backend stack needs per-service Dockerfiles so developers and operators can
build and run the same service artifacts across environments. Right now, this
is blocked by missing container build definitions and by runtime assumptions
that are local-machine oriented (especially bind-address and service-discovery
defaults).

This work should cover backend services only and explicitly exclude both game
clients and web clients.

## Solution

Define a containerization baseline for all backend services by introducing one
Dockerfile per backend service and a shared runtime contract for environment
variables, ports, and startup expectations.

The solution will target these backend services:

- Auth service
- Head service
- Matchmaking service
- Game service

The solution will also formalize container runtime behavior so each service can
run in a networked container environment without relying on localhost-only
assumptions.

## User Stories

1. As a backend developer, I want one Dockerfile per backend service, so that
   each service can be built independently.
2. As a backend developer, I want consistent image structure across services,
   so that maintenance and troubleshooting are predictable.
3. As a backend developer, I want small runtime images, so that pull times and
   startup overhead are reduced.
4. As a backend developer, I want deterministic dependency builds, so that
   image builds are reproducible across machines.
5. As a local operator, I want backend services to communicate through
   container DNS names, so that cross-service calls work without localhost
   assumptions.
6. As a local operator, I want services to bind to container-accessible
   interfaces, so that port publishing and service-to-service traffic work
   reliably.
7. As an auth service operator, I want startup to fail clearly when database
   configuration is missing, so misconfiguration is obvious.
8. As an auth service operator, I want migrations to run during service startup
   as they do today, so container behavior matches current runtime
   expectations.
9. As a head service operator, I want auth and matchmaking upstream URLs to be
   configurable in containers, so the head service can route requests
   correctly.
10. As a matchmaking operator, I want region-catalog loading to remain strict
    in containers, so invalid region configuration fails fast.
11. As a game service operator, I want auth verification dependencies to be
    configurable by environment, so game service token checks work in a
    container network.
12. As a backend developer, I want auth/head/matchmaking listener ports defined
    in one shared config location, so local and container startup use the same
    source of truth for those services.
13. As a team member, I want clear docs for image build and run commands, so
    onboarding does not require reverse-engineering runtime assumptions.
14. As a reviewer, I want explicit acceptance checks for container builds and
    startup smoke tests, so regressions are caught before merge.
15. As a product owner, I want client repositories excluded from this scope, so
    containerization work stays focused on backend delivery risk.

## Implementation Decisions

- Create one production-oriented Dockerfile per backend service, with
  multi-stage builds that compile Rust binaries in a builder stage and run
  binaries in a minimal runtime stage.
- Keep a shared Dockerfile convention across all backend services: same stage
  naming, environment variable style, and entrypoint structure.
- Preserve clean-architecture boundaries by limiting this work to
  runtime/bootstrap and deployment surface; avoid domain/use-case behavior
  changes except where required for container runtime compatibility.
- Introduce configurable bind-address behavior where needed so services can
  listen on container-accessible interfaces in container deployments.
- Define a shared backend service-port catalog in `config` for auth/head/
  matchmaking listener ports.
- Keep game-server listener port runtime-configurable via `GAME_SERVER_PORT`
  for this scope, while game-server routing endpoints remain declared in
  `config/regions.toml`.
- Keep existing default ports aligned with current service contracts through
  that shared config, with environment variables as explicit runtime overrides.
- Preserve existing service-to-service environment contracts and define
  container-safe default examples for local orchestration.
- Ensure auth service container startup still enforces required database
  configuration and runs migrations before serving traffic.
- Define backend-only orchestration expectations for local container runs,
  including required dependencies and startup ordering constraints.
- Keep region catalog behavior strict in containerized runs, including fail-fast
  validation on malformed or missing region entries.
- Exclude both game clients and web clients from Dockerfile creation in this
  PRD.
- Keep image-publishing automation, registry lifecycle, and release promotion
  as follow-up work unless explicitly requested in a separate scope.
- Document the container contract clearly: required env vars, optional env
  vars, exposed ports, and expected startup failure modes.

## Testing Decisions

- Good tests validate externally observable behavior only: image build success,
  container startup behavior, reachable service endpoints, and
  service-to-service contract outcomes.
- Add per-service container build smoke checks that verify each backend
  Dockerfile builds successfully.
- Add per-service runtime smoke checks that verify each container starts with
  expected env configuration and exposes the intended port.
- Add configuration tests that verify auth/head/matchmaking listener ports are
  loaded correctly from the shared config catalog and that environment
  overrides take precedence.
- Add negative-path checks for critical misconfiguration, especially missing
  auth database configuration and invalid region catalog configuration.
- Add a backend stack integration smoke scenario that validates basic
  cross-service connectivity for auth, head, matchmaking, and game service
  interactions.
- Reuse existing repository testing philosophy: behavior-focused tests, clear
  failure semantics, and minimal coupling to implementation details.
- Prior art in the codebase already includes strong route-level tests, use-case
  tests, and configuration-validation tests; container tests should complement
  these by proving deployment behavior rather than duplicating unit-level
  logic.

## Out of Scope

- Dockerfiles for game clients
- Dockerfiles for web clients
- Game-server listener-port centralization into the shared backend ports
  catalog (deferred; game routing endpoints remain in `config/regions.toml`)
- Kubernetes manifests and production cluster deployment
- Full CI/CD image publishing pipelines
- Registry governance and release promotion policies
- TLS termination, secret-management platform changes, or infrastructure-wide
  hardening beyond current service requirements
- Functional feature work unrelated to containerization

## Further Notes

This PRD intentionally focuses on backend runtime parity and repeatable
packaging first. It does not attempt to redesign service behavior.

The highest-risk technical constraint identified is localhost-only bind behavior
in some backend services, which must be addressed for reliable container
networking.

If needed, a follow-up PRD can define registry publishing, versioning policy,
and production orchestration standards once per-service Dockerfiles are in
place.
