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

- `config.rs` loads and validates shared startup configuration such as the
  strict region catalog.
- `server.rs` builds state, routes, and starts the Axum server.
- `auth_client.rs` implements the `AuthProvider` port with reqwest.
- `game_server_client.rs` implements the game-server lobby provisioning port.
- `game_server_directory.rs` adapts validated shared region config into the
  runtime routing directory.
- `matchmaking_client.rs` implements the `MatchmakingProvider` port with reqwest.

## Communication

- Instantiates interface adapters and passes them to the router.
- Provides configuration and runtime services to adapters.
- Does not contain application logic itself.
