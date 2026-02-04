# Cross-Cutting Concerns Guide (Jet Raiders)

This document explains how to handle cross-cutting concerns in a clean
architecture. It applies to every service in this repository, not just the
`game_server`.

It is intentionally detailed so that each service can implement consistent
patterns without leaking framework dependencies into inner layers.

## Goals

- Preserve the dependency rule: dependencies point inward.
- Keep domain logic pure and portable.
- Make infrastructure choices replaceable (logging backend, metrics exporter,
  config source, etc.).
- Provide a shared language for error handling, IDs, and time.

## Scope of cross-cutting concerns

Cross-cutting concerns are behaviors or data that appear throughout the codebase
and are not part of the core game rules. Examples include:

- Logging and tracing
- Metrics
- Configuration
- Error modeling and translation
- IDs, time, randomness
- Feature flags
- Correlation and request context

## Core rule of placement

- Domain: no direct dependencies on frameworks, logging, metrics, or config.
- Use cases: depend on **interfaces** for cross-cutting concerns, not concrete
  implementations.
- Interface adapters: translate between transport concerns and application
  concerns.
- Frameworks/drivers: own the concrete implementations and wiring.

If a type touches an external system or framework, it belongs in the outer
layers.

## Logging and tracing

### Domain

- No logging macros or tracing spans.
- Return structured domain errors or domain events instead.

### Use cases

- Emit meaningful business-level events through a small interface.
- Do not decide transport details (log format, sinks, sampling).

### Interface adapters

- Attach request context (correlation IDs, client IDs).
- Add transport-specific details (route names, HTTP status).

### Frameworks/drivers

- Configure tracing/logging backends.
- Own subscriber initialization and exporters.

### Example interface

```rust
// Application-facing interface for tracing important events.
pub trait Telemetry {
    // Records a structured event for domain actions.
    fn event(&self, name: &str, fields: &[(&str, &str)]);
}
```

## Metrics

### Domain

- No counters, gauges, or histograms.

### Use cases

- Emit metrics through a small interface for business events.
- Prefer coarse-grained metrics that represent user-visible outcomes.

### Interface adapters

- May record transport-level metrics (HTTP latency, WS frame sizes).

### Frameworks/drivers

- Own exporter setup (Prometheus, OpenTelemetry, etc.).
- Decide on runtime registries and aggregation.

### Example interface

```rust
// Application-facing interface for metrics used by use cases.
pub trait Metrics {
    // Increments a counter for a named event.
    fn incr(&self, name: &str, value: u64);

    // Records a duration in milliseconds for a named timer.
    fn timing_ms(&self, name: &str, value: u64);
}
```

## Configuration

### Domain

- No config structs or env access.

### Use cases

- Accept configuration values as inputs or simple structs.
- Treat config values as immutable inputs.

### Interface adapters

- Validate request-level config overrides if allowed.

### Frameworks/drivers

- Load config from env/files/flags.
- Construct config objects and pass into use cases.

## Errors

Error handling is layered. Each layer has different responsibilities.

### Domain errors

- Represent pure business rules.
- Avoid transport codes and logging.
- Use enums or typed error structs.

### Application errors

- Wrap domain errors and add context.
- Remain transport-agnostic.

### Transport errors

- Map application errors to protocol codes, status, and messages.

### Suggested shape

- `domain::DomainError`
- `use_cases::AppError`
- `interface_adapters::TransportError`

### Example conversion

```rust
// Converts a domain error into an application error.
impl From<DomainError> for AppError {
    // Keeps the mapping transport-agnostic.
    fn from(err: DomainError) -> Self {
        AppError::Domain(err)
    }
}
```

## IDs, time, and randomness

### Domain

- Accept IDs and timestamps as inputs.
- Do not call `Uuid::new_v4()` or `SystemTime::now()`.

### Use cases

- Depend on small interfaces for IDs, time, and RNG.

### Frameworks/drivers

- Provide concrete implementations wired at startup.

### Example interfaces

```rust
// Abstract clock for time-sensitive logic.
pub trait Clock {
    // Returns the current timestamp in milliseconds.
    fn now_ms(&self) -> u64;
}

// Abstract ID generator used by use cases.
pub trait IdGenerator {
    // Returns a new unique ID string.
    fn new_id(&self) -> String;
}
```

## Feature flags

Feature flags are a cross-cutting decision mechanism.

### Domain

- No flag lookups.

### Use cases

- Accept a `FeatureFlags` interface or config struct.

### Frameworks/drivers

- Own flag provider integration (env, config file, remote service).

### Example interface

```rust
// Interface for feature flags used by use cases.
pub trait FeatureFlags {
    // Returns true when the flag is enabled.
    fn enabled(&self, name: &str) -> bool;
}
```

## Correlation IDs and request context

### Interface adapters

- Extract correlation IDs from inbound requests.
- Attach correlation IDs to outbound calls and telemetry.

### Use cases

- Accept an optional context struct when it improves observability.
- Avoid direct dependency on transport types.

## Testing guidance

- Domain tests: no mocks for logging/metrics/config.
- Use-case tests: mock interfaces for telemetry, metrics, clock, IDs, flags.
- Adapter tests: verify error mapping and request context handling.
- Framework tests: smoke tests for wiring only.

## Anti-patterns to avoid

- Logging from domain code.
- `serde` or transport DTOs in domain structs.
- Calling system time or UUIDs inside domain systems.
- Importing metrics crates in use cases.
- Mapping HTTP status codes in use cases.

## When exceptions are acceptable

Exceptions must be explicit and documented. Examples:

- Early-stage prototypes where use cases emit protocol messages, as long as
  domain types remain clean.
- Migration phases with a clear path back to interfaces.

Document exceptions in the service-level README and link to this guide.

## Checklist for new services

- Define traits for telemetry, metrics, clock, IDs, and flags.
- Wire concrete implementations in the frameworks layer.
- Ensure domain code has zero framework dependencies.
- Ensure adapters only translate and do not own game rules.

## References

- `game_server/CLEAN_ARCHITECTURE_GUIDELINES.md`
- `ARCHITECTURE.md`
