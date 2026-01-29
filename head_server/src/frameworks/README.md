# Frameworks Layer

## Purpose

Wires up runtime concerns such as configuration, tracing, and server startup.
This is the outermost layer and depends on all inner layers.

## What belongs here

- Server startup and shutdown logic.
- Tracing and logging initialization.
- Configuration loading and environment wiring.

## What does not belong here

- Business rules or use-case logic.
- HTTP request parsing or DTO definitions.
- Domain model definitions.

## Current contents

- `server.rs` builds state, routes, and starts the Axum server.

## Communication

- Instantiates interface adapters and passes them to the router.
- Provides configuration and runtime services to adapters.
- Does not contain application logic itself.
