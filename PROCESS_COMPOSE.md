# Process Compose

## Purpose

Use `process-compose` to run all Jet Raiders services in a single terminal. The
configuration restarts each service when files change inside its directory.

## Usage

```bash
process-compose up
```

## Notes

- Services without runnable binaries (head, matchmaking, website) use
  placeholders that keep the process alive and will restart on file changes.
- The auth and game servers run via `cargo run` and restart on Rust source or
  manifest updates.
