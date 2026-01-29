# Use Cases Layer

## Purpose

Owns application-specific workflows that orchestrate domain logic and ports.
This layer contains the business logic for the head service.

## What belongs here

- Use-case orchestrators and application services.
- Policy decisions and workflow sequencing.
- Interfaces to external systems via domain ports.

## What does not belong here

- HTTP handlers or routing.
- Concrete database or HTTP client implementations.
- Framework configuration or server startup.

## Current contents

- No use cases yet. Add orchestrators as features grow.

## Communication

- Depends on domain types and port traits.
- Called by interface adapters such as HTTP handlers.
- Invokes port traits to reach external systems.
