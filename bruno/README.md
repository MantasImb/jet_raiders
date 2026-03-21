# Bruno E2E Tests

This directory holds Bruno collections for end-to-end coverage that exercises
real service boundaries outside the Rust unit and adapter test suites.

## Current Collection Layout

- `head/`: client-facing head-service coverage.
- `auth/`: reserved for future auth-service QA coverage.
- `matchmaking/`: reserved for future matchmaking-service QA coverage.

Head tests live under `bruno/head/` because they exercise the head service as
the client-facing entrypoint, even when the underlying flow reaches auth,
matchmaking, or game-server dependencies.

## Local Setup

- Start `auth_server`, `matchmaking_server`, `game_server`, and `head_server`.
- In Bruno, open the `bruno/` collection root.
- The collection reads `bruno/.env` automatically through `process.env.*`.
- Run the `head/` requests in sequence because the later cases depend on state
  created by the earlier setup and queue requests.

## Planned Head Matchmaking Cases

- `01-enqueue-waiting.bru`
- `02-poll-waiting.bru`
- `03-immediate-match-returns-ticket-and-match.bru`
- `04-waiting-ticket-transitions-to-matched.bru`
- `05-reenqueue-while-matched-returns-matched.bru`
- `06-cancel-waiting-ticket.bru`
- `07-cancel-matched-ticket-rejected.bru`
- `08-unknown-ticket-returns-404.bru`

The current collection also includes setup requests for three guest sessions
and a follow-up cancel request for the waiting-ticket flow.
