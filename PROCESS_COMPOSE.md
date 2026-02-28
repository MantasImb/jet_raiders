# Process Compose

## Purpose

Use `process-compose` to run all Jet Raiders services in a single terminal. The
configuration uses `cargo watch` to re-run active Rust services when files
change.

## Usage

```bash
process-compose up
```

## Active services

- `auth-server`: runs in `auth_server` via `cargo watch -x check -x run`
- `game-server`: runs in `game_server` via `cargo watch -x check -x run`
- `head-server`: runs in `head_server` via `cargo watch -x check -x run`

## Inactive entries in config

- `matchmaking-server` is currently commented out in `process-compose.yaml`,
  even though the service has a runnable binary.
- `website` is currently commented out and remains a placeholder.
- Uncomment and configure these entries when you want them in the local
  process-compose stack.

## Notes

- Run `process-compose` from the repository root so `working_dir` paths resolve
  correctly.
- The current setup depends on `cargo watch` being installed.
