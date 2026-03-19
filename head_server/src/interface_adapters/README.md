# Interface Adapters Layer

## Purpose

Translates external inputs into use-case requests and maps outputs into HTTP
responses or other delivery formats.

## What belongs here

- HTTP handlers and route wiring.
- Request and response DTOs for the wire format.
- State containers holding use-case services.
- Validation and mapping between HTTP DTOs and application inputs.
- HTTP-specific error translation.

## What does not belong here

- Core business rules or domain entities.
- Framework bootstrapping and server startup.
- Direct database schema definitions.

## Current contents

- `handlers/guest.rs` handles guest HTTP requests.
- `handlers/matchmaking.rs` handles matchmaking queue-entry HTTP requests.
- `routes.rs` wires HTTP routes to handlers.
- `protocol.rs` defines HTTP request/response DTOs for guest auth and matchmaking.
- `state.rs` holds runtime dependencies for handlers.

## Communication

- Receives HTTP requests and maps them to application inputs.
- Calls use cases through injected services.
- Returns DTOs to the client over HTTP.
