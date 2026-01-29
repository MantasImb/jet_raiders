# Domain Layer

## Purpose

Defines the core business rules and ports for the head service. This layer owns
abstract interfaces and domain data that are independent of frameworks.

## What belongs here

- Domain entities and value objects.
- Port traits (interfaces) consumed by inner layers.
- Domain request/response types used by ports.

## What does not belong here

- Axum, HTTP, or transport concerns.
- Database drivers or external clients.
- Serialization details tied to a specific wire format.

## Current contents

- `auth.rs` defines `AuthProvider` and its request/response types.

## Communication

- Inner layers depend on this layer only.
- Interface adapters implement the port traits defined here.
- Use cases call the ports without knowing the concrete implementations.
