# Guest Init Continuation Plan

## Objective

Ensure `load_or_create_profile()` does not run post-init setup before
`_init_guest_id()` finishes when guest identity is missing or invalid.

## Scope

- `game_client/Scripts/UserManager.gd`
- Startup sequencing only
- No server/API contract changes

## Current Issue

`_init_guest_id()` is asynchronous. `load_or_create_profile()` continues to
run and calls `_save_profile()` before the HTTP callback completes.

This can persist a transient empty `guest_id` and create ambiguous startup
ordering.

## Proposed Approach

Use continuation-based flow.

1. Add a small helper for post-load setup.
2. In `load_or_create_profile()`, return immediately after `_init_guest_id()`
   when guest ID is missing or invalid.
3. Call the helper from:
   - The valid stored guest ID path (immediate).
   - `_init_guest_id()` callback after successful guest ID validation.

This guarantees post-init actions run only after ID availability in the init
branch.

## Detailed Design

### 1. Extract Post-Load Setup

Create `_finish_profile_setup()` to hold logic that currently runs after guest
ID branch handling.

Expected contents:

- Assign `username_input.text = local_username`.
- Persist profile via `_save_profile()`.

### 2. Early Return In Init Branches

In `load_or_create_profile()`:

- If stored guest ID is invalid:
  - Set `guest_id = ""`.
  - Call `_init_guest_id()`.
  - `return`.
- If guest ID is absent:
  - Call `_init_guest_id()`.
  - `return`.
- If stored guest ID is valid:
  - Call `_finish_profile_setup()` immediately.

### 3. Continue In Init Callback

In `_init_guest_id()` callback:

- Keep existing request/JSON checks.
- Validate resolved `guest_id` with `_is_guest_id_valid(...)` before persisting.
- On success:
  - Assign `guest_id`.
  - Call `_finish_profile_setup()`.

### 4. Failure Behavior

If init fails, do not call `_finish_profile_setup()`.

Optional follow-up behavior (separate decision):

- Show explicit failure UI.
- Trigger retry logic.

## Expected Benefits

- Deterministic startup order in missing/invalid ID path.
- No profile write before init completion in that path.
- Cleaner separation between bootstrap decision and completion actions.

## Risks And Mitigations

1. UI field not initialized on init failure.
   Mitigation: Decide whether UI should still be populated on failure.

2. Duplicate save calls in success path.
   Mitigation: Ensure `_finish_profile_setup()` is called once per startup path.

3. Callback writes malformed non-empty IDs.
   Mitigation: Reuse `_is_guest_id_valid(...)` in callback before assignment.

## Implementation Steps

1. Add `_finish_profile_setup()` helper in `UserManager.gd`.
2. Refactor `load_or_create_profile()` to early-return in init branches.
3. Update `_init_guest_id()` callback to call helper on successful init.
4. Add callback-side guest ID validation before assignment/save.
5. Verify no path calls `_save_profile()` prior to init completion in init
   branches.

## Validation Checklist

1. No profile file present:
   `_save_profile()` runs only after successful init callback.
2. Stored invalid `guest_id`:
   old ID is discarded, new ID fetched, setup completes once.
3. Stored valid `guest_id`:
   setup runs immediately without init request.
4. Init request fails:
   no premature save from startup continuation path.
5. Username edit after startup:
   save behavior remains unchanged.

## Notes

This plan intentionally avoids introducing new synchronization flags.
The ordering guarantee comes from control flow and callback continuation.
