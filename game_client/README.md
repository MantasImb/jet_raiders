# Game Client (Godot)

This directory contains the active Jet Raiders Godot client project.

## Current Status

- The runnable Godot project is in `game_client/` (`project.godot`).
- Some top-level architecture docs still mention `client/`; treat
  `game_client/` as the source of truth for the current client code.

## Quick Start

1. Start backend services from the repository root:

   ```bash
   process-compose up
   ```

2. Open the Godot project at `game_client/project.godot`.
3. Run the main scene from the editor.

## Runtime Defaults

- Engine/features: Godot 4.5 (`GL Compatibility` renderer profile).
- Test mode: enabled by default (`GameManager.TEST_MODE == true`).
- Head service base URL: `http://127.0.0.1:3000`
- Game WebSocket URL (test mode): `ws://127.0.0.1:3001/ws`

With test mode enabled, the client auto-connects to the game server after
guest login succeeds.

## Controls

- `W` / `S`: throttle up/down
- `A` / `D`: turn left/right
- `Space`: shoot

## Key Scripts

- `Scripts/GameManager.gd`: game-level state and test mode toggle.
- `Scripts/UserManager.gd`: guest profile, guest init, and login flow.
- `Scripts/NetworkManager.gd`: WebSocket lifecycle, join, reconnect, and world
  snapshot application.
- `Scripts/PlayerInput.gd`: per-frame local input capture and send.
- `Scripts/Player.gd`, `Scripts/Projectile.gd`: visual smoothing from server
  snapshots.

## Architecture Guidelines

Follow repository-wide `CLEAN_ARCHITECTURE_GUIDELINES.md` when introducing
shared client services or cross-cutting modules.

## Related Docs

- `game_client/CLIENT_NETWORKING.md`
- `ARCHITECTURE.md`
- `CLEAN_ARCHITECTURE_GUIDELINES.md`
