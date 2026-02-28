# Auth Init Failure Handling Plan

## Objective

Prevent startup lockups and infinite login loops when guest ID initialization or
login requests fail.

## Problems To Address

1. Login can be attempted before `guest_id` is ready.
2. `_init_guest_id()` does not handle `HTTPRequest.request(...)` startup failure.
3. `_process()` retries can become unbounded and noisy.
4. The user does not get a clear terminal error state after repeated failures.

## Proposed Design

### 1. Add Explicit Auth Bootstrap State

In `UserManager.gd`, replace loosely coupled booleans with a single state enum-like
string (or integer constants), for example:

- `IDLE`
- `INITIALIZING_GUEST`
- `AUTHENTICATING`
- `READY`
- `FATAL_ERROR`

Keep `auth_token` and `guest_id` as data, but let state drive control flow.

### 2. Retry Policy With Backoff

Add separate retry counters for:

- Guest init (`/guest/init`)
- Login (`/guest/login`)

Policy:

- Max retries: configurable constant (example: `MAX_RETRIES = 3`)
- Delay strategy: exponential backoff with cap
- On success: reset retry counter

### 3. Catch-All Fatal Error Model

Add fields in `UserManager.gd`:

- `has_fatal_error: bool`
- `fatal_error_message: String`

When retries are exhausted:

- Set `has_fatal_error = true`
- Set `fatal_error_message` to a user-readable cause
- Move state to `FATAL_ERROR`

### 4. Process Gating In `NetworkManager`

In `_process()`:

- If user state is `FATAL_ERROR`, return immediately and do not run login/network
  bootstrap loops.
- If user state is `INITIALIZING_GUEST` or `AUTHENTICATING`, return.
- Only trigger `login()` when user state is ready to authenticate and `guest_id > 0`.

### 5. Request Startup Error Handling

In `_init_guest_id()` and `login()`:

- Capture `request_error` from `http.request(...)`.
- If not `OK`, free the request node, count a retry, and schedule the next attempt.
- Do not rely only on `request_completed`; startup failure never triggers callback.
- Ensure immediate `login()` request failure also calls `http.queue_free()` to avoid
  leaking `HTTPRequest` nodes (third-fix requirement).

### 6. UI/UX Behavior

When in `FATAL_ERROR`:

- Show `fatal_error_message` in network UI panel.
- Keep reconnect/login attempts stopped.
- Optional next step: add a manual `Retry` button that resets state and counters.

## Implementation Steps

1. Add state constants and error fields in `UserManager.gd`.
2. Refactor `_init_guest_id()` to include request startup error handling + retries.
3. Refactor `login()` to run only when `guest_id` is available and to use retry
   policy; preserve the immediate-failure cleanup (`http.queue_free()`).
4. Update `NetworkManager.gd` `_process()` to be state-driven.
5. Add fatal error rendering in existing network UI.
6. Validate startup flow with auth server online and offline scenarios.

## Validation Checklist

1. First run with no profile:
   `guest_id` is created, login succeeds, WebSocket joins.
2. Auth server offline on startup:
   retries occur with backoff, then fatal error is shown, loops stop.
3. Auth server returns `400/500`:
   retries and final error behavior are deterministic.
4. No unbounded `HTTPRequest` node accumulation.
5. Returning user with valid profile:
   skips init and proceeds directly to login.

## Notes

- Keep changes minimal and localized to `UserManager.gd` and `NetworkManager.gd`.
- Prefer explicit state transitions over multiple independent booleans.
- Use concise error messages suitable for players, with detailed logs for debugging.
