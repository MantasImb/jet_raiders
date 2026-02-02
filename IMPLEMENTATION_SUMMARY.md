# Implementation Summary

## Overview

- Introduced a lobby registry that spawns one world task per lobby and
  maintains per-lobby channels for inputs, world updates, and state.
- Added HTTP and WebSocket wiring for lobby creation and selection,
  including a default `test` lobby for the existing test world.
- Enforced allowlists for player spawning while keeping non-allowlisted
  connections in spectator mode.

## Endpoints

- `POST /lobbies` creates a lobby with an optional `lobby_id` and a list of
  `allowed_player_ids`, returning the lobby identifier.
- `GET /ws` accepts `lobby_id` (optional) and `player_id` (optional) query
  parameters to connect to a specific lobby.

## Notes

- The default test lobby is created at startup using the lobby id `test`.
- Non-allowlisted connections still receive world updates but their inputs
  are ignored to keep them in spectator mode.
