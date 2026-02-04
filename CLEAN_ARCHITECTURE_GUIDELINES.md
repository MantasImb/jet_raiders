# Clean Architecture Guidelines (Repository-Wide)

This document is the repository-wide source of truth for clean architecture.
It applies to all current and future services. Service-specific documents may
add detail, but they must not contradict these rules.

## Dependency rule (non-negotiable)

Dependencies must point inward:

- Outer layers may depend on inner layers.
- Inner layers must not depend on outer layers.

If a module touches a framework, transport, or runtime concern, it belongs in
an outer layer.

## Standard layers (general mapping)

All services use the same logical layers. Directory names may vary per service,
but the responsibilities are consistent.

- Entities (Domain): core business rules and data models.
- Use Cases (Application): orchestration and workflow logic.
- Interface Adapters: transport mapping, DTO validation, conversions.
- Frameworks/Drivers: runtime setup, wiring, and concrete integrations.

## Directory mapping guidance

Choose one of these patterns per service and stay consistent:

- `domain/`, `use_cases/`, `interface_adapters/`, `frameworks/`
- `domain/`, `app/`, `adapters/`, `infrastructure/`
- `domain/`, `application/`, `adapters/`, `frameworks/`

Avoid mixing naming styles within a single service.

## Layer responsibilities

### Entities (Domain)

Owns:

- Canonical domain types and invariants.
- Core rules and state mutation logic.

Must not own:

- DTOs or serialization.
- Runtime/framework types.
- Networking, persistence, or tracing.

### Use Cases (Application)

Owns:

- Workflow orchestration.
- Business process coordination.
- Domain-facing interfaces (ports/traits).

Must not own:

- Transport concerns (HTTP, WebSocket, JSON).
- Framework/runtime setup.
- Concrete infrastructure implementations.

### Interface Adapters

Owns:

- Request/response DTOs and validation.
- Mapping between transport models and domain types.
- Error translation to transport-specific outputs.

Must not own:

- Core domain rules.
- Infrastructure wiring.

### Frameworks/Drivers

Owns:

- Runtime setup and lifecycle.
- Dependency wiring.
- Concrete integrations (DB, cache, telemetry, metrics).

Must not own:

- Domain rules or workflows.
- DTO definitions used as domain types.

## Boundaries: domain types vs protocol DTOs

Domain types are authoritative and belong only to the domain layer. Protocol
DTOs exist only in the adapter layer for serialization and versioning.

Rules:

- Do not store `protocol::*` (or transport DTOs) inside domain entities.
- Do not accept DTOs as inputs to domain systems or use cases.
- Convert at the boundary (adapter layer) and pass in domain types.

## Cross-cutting concerns (summary)

Cross-cutting concerns are handled via interfaces in inner layers and concrete
implementations in outer layers.

- Domain: no logging, metrics, config, env access, or direct time/UUID calls.
- Use cases: depend on small interfaces (telemetry, metrics, clock, IDs).
- Adapters: attach request context and map errors.
- Frameworks: own tracing setup, metrics exporters, config parsing, and
  concrete implementations.

If you need the detailed guide, see `game_server/CROSS_CUTTING_CONCERNS.md`.

## Pragmatic early stage (exception with guardrails)

Use cases may temporarily emit protocol DTOs to accelerate early development,
but only under strict guardrails.

Allowed:

- A single use case may emit protocol output types (e.g., game loop snapshots).
- No transport framework types in use cases.

Not allowed:

- Protocol types in domain entities or domain systems.
- Use cases accepting protocol DTOs as inputs.
- Multiple use cases emitting protocol DTOs.

Guardrails (required):

- Document the exception in the service README with a removal milestone.
- Keep inputs as domain types; only outputs may be protocol DTOs.
- Keep protocol dependencies out of all other use cases.

Migration path:

- Introduce domain events/snapshots in use cases.
- Convert outputs in interface adapters.
- Remove protocol types from use cases.

## Testing guidance

- Domain tests: no mocks for logging/metrics/config.
- Use-case tests: mock interfaces for telemetry, metrics, clock, IDs.
- Adapter tests: verify error mapping and DTO conversion.
- Framework tests: smoke tests for wiring and config only.

## Checklist for new services

- Pick a consistent directory mapping for the four layers.
- Keep domain pure: no framework or transport imports.
- Define interfaces for cross-cutting concerns in use cases.
- Implement adapters for request/response conversion.
- Wire all concrete implementations in frameworks/drivers.

## Related docs

- `ARCHITECTURE.md`
- `game_server/CLEAN_ARCHITECTURE_GUIDELINES.md`
- Service-specific architecture documents under each service directory.
