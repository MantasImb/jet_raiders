# Interface Adapters Layer

## Purpose

Translates external inputs into use-case requests and maps outputs into HTTP
responses or other delivery formats.

## What belongs here

- HTTP handlers and route wiring.
- Request and response DTOs for the wire format.
- State containers holding port implementations.
- Client adapters that implement domain ports.

## What does not belong here

- Core business rules or domain entities.
- Framework bootstrapping and server startup.
- Direct database schema definitions.

## Current contents

- `handlers/guest.rs` handles guest login HTTP requests.
- `routes.rs` wires HTTP routes to handlers.
- `protocol.rs` defines HTTP request/response DTOs.
- `clients.rs` implements the `AuthProvider` port via HTTP.
- `state.rs` holds runtime dependencies for handlers.

## Communication

- Receives HTTP requests and maps them to domain requests.
- Calls use cases or domain ports through trait objects.
- Returns DTOs to the client over HTTP.
