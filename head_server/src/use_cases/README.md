# Use Cases Layer

## Purpose

Owns application-specific workflows that orchestrate domain logic and ports.
This layer contains the business logic for the head service.

## What belongs here

- Use-case orchestrators and application services.
- Policy decisions and workflow sequencing.
- Interfaces to external systems via use-case ports.
- Application request/response types shared across boundaries.

## What does not belong here

- HTTP handlers or routing.
- Concrete database or HTTP client implementations.
- Framework configuration or server startup.

## Current contents

- `guest.rs` orchestrates guest init/login flows.
- `guest.rs` defines the `AuthProvider` port and application-level errors.
- `matchmaking.rs` orchestrates matchmaking queue entry and ticket polling.
- `matchmaking.rs` defines the `MatchmakingProvider` port and application-level
  errors for both flows.

## Communication

- Depends on domain types and locally defined port traits.
- Called by interface adapters such as HTTP handlers.
- Invokes port traits to reach external systems.
