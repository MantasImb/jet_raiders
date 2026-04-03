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
- `head/assert-lobby-visibility.mjs` requires `Node >=21` because it relies on
  the built-in `WebSocket` client.
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
- `09-us-east-enqueue-waiting-player-c.bru`
- `10-us-east-immediate-match-player-d.bru`
- `11-us-east-poll-player-c-matched.bru`
- `12-invalid-region-uppercase-returns-400.bru`
- `13-invalid-region-trailing-space-returns-400.bru`
- `14-invalid-region-unknown-returns-400.bru`

The current collection also includes setup requests for three guest sessions
and a follow-up cancel request for the waiting-ticket flow. The matched-flow
requests assert that head returns `ticket_id` together with `match_id`,
`lobby_id`, `ws_url`, and `region`.

The phase-5 regional additions also assert that:

- `us-east` follows the same game-ready matched handoff contract as `eu-west`.
- region matching remains exact with no case-folding, trimming, or unknown-region
  fallback.

`04-waiting-ticket-transitions-to-matched.bru` also runs
`head/assert-lobby-visibility.mjs`, which uses the matched `ws_url`,
`lobby_id`, and both players' `session_token` values to verify that:

- both players can complete the public WebSocket join handshake
- both players receive the expected identity assignment
- both players observe each other in `WorldUpdate.entities`

## Local-Only Assumptions

This Bruno collection intentionally includes machine-specific assumptions for
the current solo-developer workflow.

- The WebSocket visibility check may rely on local absolute paths.
- The helper execution may rely on the current machine's Node installation
  layout.
- These tradeoffs are accepted for the Bruno tests only.

Do not treat this as a repository-wide portability standard. Outside the Bruno
test collection, keep the usual expectation that code and tooling remain
portable and not tied to one developer machine.
