# Repository Testing Scope

## Context

Most services in this repository are internal-facing. They are called by other
services and clients developed in this repository and controlled by the same
team.

Because of that, test strategy should prioritize critical behavior contracts and
avoid unnecessary test volume for low-risk edge cases.

## Testing Priorities

Keep strong coverage for:

- Domain and use-case invariants (identity/session/game-state rules as
  applicable per service).
- Security behavior in auth-sensitive flows (invalid token rejection, expiry
  handling, revocation behavior).
- Core adapter contracts (HTTP/WebSocket/protocol status and payload mapping
  for primary success and failure paths).
- Route and handler wiring smoke checks (`404`/`405` and expected method
  behavior) to catch accidental routing regressions.

## What To Avoid Over-Testing

Prefer not to add many tests for:

- Highly specific error message string variants unless another internal service
  strictly depends on exact message text.
- Repeated malformed payload permutations when one extraction failure test is
  enough.
- Low-value micro-edge cases that do not affect business correctness, auth
  security, or service-to-service compatibility.

## Practical Rule

For new changes:

1. Add tests that protect behavior contracts and security boundaries.
2. Add one negative-path test for forbidden/invalid input.
3. Skip redundant tests that only duplicate already protected behavior.

This keeps the suite fast, readable, and aligned with internal service usage
across the repository.

Exception:
`auth_server` currently keeps broader-than-default test coverage because auth
flows are security-sensitive and shared by multiple internal services.
