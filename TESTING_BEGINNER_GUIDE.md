# Testing Beginner Guide

This guide is for contributors who are new to tests and want a practical way to
start in this repository.

## What is a test?

A test is code that verifies expected behavior.

Think of it as an executable check that answers:

- "If I give the system this input, do I get the expected result?"
- "If I send invalid input, do I get the expected error?"
- "Did a new change break previously working behavior?"

## Why tests matter

Tests help you:

- Catch regressions before code is merged.
- Document expected behavior in a precise way.
- Refactor safely with fast feedback.
- Give Codex clear boundaries for implementation.

## Where tests live in this repository

Current test files are in the `game_server` service:

```text
game_server/
  tests/
    lobby_create.rs
    ws_join.rs
    support/
      mod.rs
```

What these files do:

- `lobby_create.rs`: integration tests for `POST /lobbies` behavior.
- `ws_join.rs`: WebSocket join contract test area.
- `support/mod.rs`: shared server startup helper for tests.

For additional context, read:

- `TESTING.md` (game server testing notes).
- `CLEAN_ARCHITECTURE_GUIDELINES.md` (where test logic should live by layer).

## First mental model: test at the right boundary

Use this simple mapping:

- Domain tests: business rules and invariants.
- Use-case tests: orchestration/workflow behavior.
- Adapter tests: DTO mapping and error translation.
- Framework tests: runtime wiring smoke checks.

If your change is an HTTP contract, start with integration tests near the
endpoint boundary.

## How to run tests

From `game_server/` run:

```bash
cargo test --tests -- --test-threads=1
```

Why single-threaded for now:

- Current test support starts one shared server per test binary.
- Single-threaded runs keep lifecycle behavior predictable.

## How to write your first test (step by step)

### 1. Pick one behavior

Example behavior:

- "Creating a lobby with a valid `lobby_id` should return `201`."

### 2. Add one test file change

Follow the existing pattern in `game_server/tests/lobby_create.rs`:

- Call `support::ensure_server()` to get the base URL.
- Send request with `reqwest::Client`.
- Assert status code first.
- Assert JSON response fields second.

### 3. Run tests and confirm failure first

Before implementing logic, make sure your new test fails for the expected
reason. This is the "red" step in TDD.

### 4. Implement minimal code

Add only the code needed to make this test pass. Avoid unrelated refactors.

### 5. Re-run tests

Run the targeted tests, then broader service tests.

### 6. Add a negative-path test

For the same feature, add at least one test for an invalid/forbidden case.

Example:

- Missing `lobby_id` should return `400` with an error payload.

This protects against over-permissive implementations.

## Practical test-writing tips

- Keep each test focused on one rule.
- Use clear test names that describe context and expected outcome.
- Avoid hidden randomness; use deterministic inputs when possible.
- Assert error contracts explicitly (status code and error payload).
- Prefer behavior assertions over internal implementation details.

## Common beginner mistakes (and fixes)

1. **Mistake:** Testing too much in one test.
   **Fix:** Split into small tests, one behavior each.

2. **Mistake:** Writing tests after implementation.
   **Fix:** Write or update a failing test first.

3. **Mistake:** Asserting only "request succeeded".
   **Fix:** Also assert response body contract.

4. **Mistake:** Changing unrelated code while fixing a test.
   **Fix:** Keep changes minimal and scoped.

5. **Mistake:** Ignoring failure messages.
   **Fix:** Improve assertion clarity so failures are actionable.

## Suggested checklist before opening a PR

- [ ] Did I add or update tests for new behavior?
- [ ] Did I include at least one negative-path test?
- [ ] Do tests pass locally for the affected service?
- [ ] Are assertions specific and readable?
- [ ] Did I avoid unrelated changes?

## If you are using Codex to implement code

Use a test-first prompt style:

```text
Implement <feature> using TDD.
Add/adjust tests first and show failing tests.
Keep implementation scoped to <layer>.
Add at least one negative-path test.
Run targeted tests, then broader suite.
```

This keeps generated code aligned with expected behavior and reduces
regressions.
