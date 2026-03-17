# Domain Layer

## Purpose

Defines core business entities and invariants for the head service. This layer
stays independent of frameworks, transport concerns, and use-case ports.

## What belongs here

- Domain entities and value objects.
- Business invariants shared across use cases.

## What does not belong here

- Axum, HTTP, or transport concerns.
- Database drivers or external clients.
- Serialization details tied to a specific wire format.

## Current contents

- No head-specific domain entities yet.

## Communication

- Inner layers depend on this layer only.
- Use cases may introduce head-specific business entities here as the service
  grows.
