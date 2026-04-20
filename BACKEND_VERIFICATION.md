# Backend Verification Contract

## Purpose

This document defines the repeatable backend verification contract for local
runs and CI.

The contract is intentionally backend-only and excludes `game_client/` and
`website/` from execution scope.

## Scope Boundaries

Included services:

- `auth_server`
- `head_server`
- `matchmaking_server`
- `game_server`

Excluded components:

- `game_client`
- `website`

## Developer Build and Run References

Per-service Docker build and runtime contracts are documented in each service
README:

- [auth_server/README.md](auth_server/README.md)
- [head_server/README.md](head_server/README.md)
- [matchmaking_server/README.md](matchmaking_server/README.md)
- [game_server/README.md](game_server/README.md)

Each README includes:

- Image build command (`docker build -f <service>/Dockerfile ...`)
- Runtime environment requirements
- Service run command and health smoke command

## Verification Sequence (Local and CI)

Run this command from repository root:

```bash
scripts/ci_smoke.sh
```

Execution contract:

1. If `DATABASE_URL` is already set, use it for `auth_server`.
2. If `DATABASE_URL` is not set, start an ephemeral Postgres container on
   `127.0.0.1:55432`.
3. Start backend services with local loopback wiring.
4. Wait for `/health` readiness checks on all backend services.
5. Call `POST /guest/init` for two players.
6. Call `POST /matchmaking/queue` for player one.
7. Call `POST /matchmaking/queue` for player two.
8. Assert the second queue response is `matched` and includes `ws_url`.
9. Tear down started services and ephemeral dependencies.

Expected successful output:

```text
ci_smoke passed
```

## Startup Dependencies

- Rust toolchain available in the environment for `cargo run`.
- `curl` available for health and API checks.
- Docker available when `DATABASE_URL` is not supplied (for ephemeral Postgres).

## Failure Modes and Troubleshooting

Common failures:

- Missing Docker with no `DATABASE_URL` set.
- Service startup timeout from compile delay or dependency startup delay.
- Invalid runtime config causing fail-fast startup exits.

Troubleshooting basics:

- Inspect service logs under `.tmp/ci-smoke-logs/`.
- Verify the database connection URL used by `auth_server`.
- Re-run the script locally before pushing workflow changes.

## CI Wiring

GitHub Actions workflow:

- `.github/workflows/backend-smoke.yml`

The workflow runs `bash scripts/ci_smoke.sh` as the single backend verification
entrypoint.
