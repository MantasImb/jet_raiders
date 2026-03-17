# Auth State Machine

This document explains the implemented authentication flow in the Godot client.
It focuses on:

- which nodes participate
- what each state does
- when each transition happens
- how the auth machine integrates with the rest of the game

## Overview

Authentication is implemented as a node-based state machine under `Network` in
`main.tscn`.

Current scene structure:

```text
Network
├── AuthContext
├── AuthStateMachine
│   ├── BootstrapState
│   ├── GuestIdentityState
│   ├── LoginState
│   └── AuthenticatedState
├── WorldSync
├── NetworkUI
└── SpawnedNodes
```

`NetworkManager.gd` is attached to the `Network` node itself; it is not a
separate child node.

High-level responsibilities:

- `AuthContext`: owns auth/profile data, persistence, and validation helpers
- `AuthStateMachine`: owns auth orchestration and transitions
- `AuthApiClient`: owns auth HTTP transport calls
- `WorldSync`: owns scene mutation for network world snapshots
- auth state nodes: implement one step of the auth workflow each
- `NetworkManager`: reacts to auth success and manages the WebSocket lifecycle

## Main Parts

### `AuthContext`

File:

- `Scripts/AuthContext.gd`

Owns:

- `guest_id`
- `auth_token`
- `local_username`
- `local_player_id`
- profile load/save from `user://guest_profile.json`
- display name validation
- guest id validation

It does not decide auth flow. It only stores data and exposes helper methods.

### `AuthStateMachine`

File:

- `Scripts/auth/AuthStateMachine.gd`

Owns:

- current active top-level state node
- `AuthApiClient`
- top-level transition rules
- retry counters
- retry timer
- auth error code/detail
- `authenticated(session_token)` signal

Important methods:

- `transition_to(...)`: swaps top-level state nodes
- `send_event(...)`: forwards events to the active state
- `is_authenticated()`: reports whether the active top-level state is
  `AuthenticatedState`
- `request_login()`: public entry point for manual login retry
- `schedule_retry(...)`: starts retry backoff timer
- `clear_retry(...)`: resets retry tracking

### `AuthApiClient`

File:

- `Scripts/auth/AuthApiClient.gd`

Owns:

- `POST /guest/init` request transport
- `POST /guest/login` request transport
- common HTTP request setup
- common JSON response parsing
- common transport error normalization

Why it exists:

- keeps HTTP transport out of the state scripts
- keeps `AuthStateBase` focused on state behavior
- avoids duplicating request/parse boilerplate in guest and login states

### `AuthStateBase`

File:

- `Scripts/auth/states/AuthStateBase.gd`

This is the base class for all auth states.

Common API:

- `enter(ctx)`
- `exit()`
- `handle_event(event, payload)`

## Top-Level State Flow

Normal guest-auth flow:

```text
BootstrapState
  -> GuestIdentityState
  -> LoginState
  -> AuthenticatedState
```

Fallback path:

```text
LoginState
  -> GuestIdentityState
```

That fallback only happens if login begins but `guest_id` is missing or invalid.

## Runtime Invariants

These rules are useful when reading the code and debugging runtime behavior:

- `AuthContext.guest_id` must be valid before `LoginState` can complete.
- `AuthContext.auth_token` is only expected to be populated after a successful
  login and before or during `AuthenticatedState`.
- `AuthContext.local_player_id` is not part of HTTP auth. It is assigned later
  from the game server `Identity` message after WebSocket join succeeds.
- In the normal guest flow, entering `LoginState` immediately starts login.
- `AuthStateMachine.request_login()` mainly matters for recovery from terminal
  failure states; it is not the mechanism that starts the normal happy path.

## State Details

### `BootstrapState`

File:

- `Scripts/auth/states/BootstrapState.gd`

Purpose:

- initialize auth runtime state
- clear stale auth errors
- reset retry counters

What happens on enter:

1. Clear auth error state.
2. Reset guest init retry tracking.
3. Reset login retry tracking.
4. Immediately transition to `GuestIdentityState`.

What happens on exit:

- nothing special

Why it exists:

- keeps startup/reset logic out of the guest identity flow

### `GuestIdentityState`

File:

- `Scripts/auth/states/GuestIdentityState.gd`

Purpose:

- load persisted profile
- decide whether a valid `guest_id` already exists
- request a new guest identity when needed

Internal substates:

- `LOAD_PROFILE`
- `REQUEST_GUEST_ID`
- `RETRY_WAIT`
- `FAILED`
- `READY`

What happens on enter:

1. Set substate to `LOAD_PROFILE`.
2. Load profile data from `AuthContext`.
3. Apply profile data into runtime state.
4. If `guest_id` is valid:
   - save normalized profile state
   - clear guest-init retry count
   - set substate to `READY`
   - transition to `LoginState`
5. If `guest_id` is missing or invalid:
   - start `POST /guest/init`
   - set substate to `REQUEST_GUEST_ID`

When guest init succeeds:

1. Validate the returned `guest_id`.
2. Store it in `AuthContext`.
3. Save the profile.
4. Clear retry/error state.
5. Set substate to `READY`.
6. Transition to `LoginState`.

When guest init fails:

1. Store error code/detail in `AuthStateMachine`.
2. If retries remain:
   - set substate to `RETRY_WAIT`
   - schedule retry with exponential backoff
3. If retries are exhausted:
   - set substate to `FAILED`

How it leaves `RETRY_WAIT`:

- retry timer fires
- `AuthStateMachine` sends `retry_timeout`
- `GuestIdentityState` handles that event and starts guest init again

How it leaves `FAILED`:

- manual `login_requested` event
- this re-attempts guest identity resolution

Late-response behavior:

- if an old `/guest/init` HTTP response arrives after the state has already
  moved out of `REQUEST_GUEST_ID`, it is ignored
- this prevents stale callbacks from mutating auth state after a transition

### `LoginState`

File:

- `Scripts/auth/states/LoginState.gd`

Purpose:

- use `guest_id` to obtain a fresh `session_token`

Internal substates:

- `IDLE_READY`
- `REQUEST_LOGIN`
- `RETRY_WAIT`
- `FAILED_TERMINAL`
- `SUCCESS`

What happens on enter:

1. Set substate to `IDLE_READY`.
2. Clear previous auth error state.
3. Immediately call `_begin_login()` for the current guest flow.

What `_begin_login()` does:

1. Verify that `AuthContext` still has a valid `guest_id`.
2. If not valid:
   - transition back to `GuestIdentityState`
3. If valid:
   - normalize display name
   - set substate to `REQUEST_LOGIN`
   - call `POST /guest/login`

When login succeeds:

1. Validate that `session_token` exists in the response.
2. Trim the token and reject it if it is empty or whitespace-only.
3. Store the validated token in `AuthContext.auth_token`.
4. Clear login retry count.
5. Clear auth errors.
6. Set substate to `SUCCESS`.
7. Transition to `AuthenticatedState`.

When login fails:

1. Store error code/detail in `AuthStateMachine`.
2. If retries remain:
   - set substate to `RETRY_WAIT`
   - schedule retry
3. If retries are exhausted:
   - set substate to `FAILED_TERMINAL`

How it leaves `RETRY_WAIT`:

- retry timer fires
- `AuthStateMachine` sends `retry_timeout`
- `LoginState` begins login again

How it leaves `FAILED_TERMINAL`:

- player presses the matchmaking/login button
- UI calls `AuthStateMachine.request_login()`
- state machine sends `login_requested`
- `LoginState` handles the event and restarts login

Late-response behavior:

- if an old `/guest/login` HTTP response arrives after the state has already
  moved out of `REQUEST_LOGIN`, it is ignored
- this prevents a late callback from overwriting newer auth progress

### `AuthenticatedState`

File:

- `Scripts/auth/states/AuthenticatedState.gd`

Purpose:

- represent successful authentication

What happens on enter:

1. Verify `AuthContext.auth_token` is present and not blank.
2. If it is blank:
   - store an auth error in `AuthStateMachine`
   - clear stale session fields from `AuthContext`
   - transition back to `LoginState`
3. If it is valid:
   - emit `AuthStateMachine.authenticated(auth_token)`

What happens on exit:

- nothing in the current implementation

Why this state matters:

- it is the handoff point from auth flow to networking flow

## Transition Rules

Top-level transitions allowed by `AuthStateMachine`:

- `BootstrapState -> GuestIdentityState`
- `GuestIdentityState -> LoginState`
- `LoginState -> GuestIdentityState`
- `LoginState -> AuthenticatedState`
- `AuthenticatedState -> LoginState`

Anything else is rejected as an invalid transition.

This keeps transitions explicit and prevents unrelated states from jumping
directly to each other.

## Retry Behavior

Retry behavior is centralized in `AuthStateMachine`.

Current settings:

- max guest init retries: `5`
- max login retries: `5`
- base backoff: `0.5s`
- max backoff: `6.0s`

How retry works:

1. A state detects failure.
2. The state asks `AuthStateMachine` whether retries remain.
3. If yes, the state schedules retry.
4. `AuthStateMachine` starts its shared timer.
5. On timer timeout, `AuthStateMachine` sends `retry_timeout` back to the
   current state.
6. That state restarts its failed operation.

Why the timer lives in the machine:

- retry policy is shared
- state scripts stay smaller
- backoff logic is defined in one place

## Failure Semantics

The auth machine has two terminal failure substates:

- `GuestIdentityState.FAILED`: guest identity resolution stopped after
  exhausting guest-init retries
- `LoginState.FAILED_TERMINAL`: login stopped after exhausting login retries

Manual recovery behavior:

- pressing the login button while in `GuestIdentityState.FAILED` retries guest
  identity resolution
- pressing the login button while in `LoginState.FAILED_TERMINAL` retries login
- `request_login()` re-drives the current failure state; it does not reset the
  whole FSM back to `BootstrapState`

## Integration With The Game

### Startup

`AuthStateMachine._ready()` starts the auth flow by transitioning into
`BootstrapState`.

That means auth startup is owned by the auth machine itself, not by
`NetworkManager`.

### UI

The login/matchmaking button in `main.tscn` is wired to:

- `Network/AuthStateMachine.request_login`

In the current guest flow, login auto-starts when `LoginState` is entered, so
this button mainly matters for manual retry from failure states.

The username field is still wired to:

- `Network/AuthContext._on_username_input_text_changed`

That keeps username persistence in the auth context.

### NetworkManager

`NetworkManager` listens to:

- `AuthStateMachine.authenticated`

When that signal fires:

1. `NetworkManager` starts the WebSocket connection in test mode.
2. On socket open, it sends `Join { session_token }`.
3. The game server responds with `Identity`.
4. `NetworkManager` stores `player_id` into `AuthContext.local_player_id`.

`NetworkManager` does not drive auth flow anymore.

### WorldSync

`WorldSync` receives `WorldUpdate` payloads from `NetworkManager`.

It is responsible for:

- spawning synchronized player and projectile nodes
- updating existing nodes from snapshots
- despawning nodes missing from the latest snapshot
- clearing synchronized nodes when the connection closes

This keeps scene mutation separate from transport logic and makes it easier to
reuse the same sync behavior with a future non-WebSocket transport.

### Player and Input Ownership

`Player.gd` and `PlayerInput.gd` read:

- `AuthContext.local_player_id`

That is how the client identifies which spawned entity is the local player and
which player node is allowed to send input.

## Data Flow Summary

Startup path:

1. `AuthStateMachine` starts.
2. Profile is loaded from local disk.
3. Missing `guest_id` is created through head server.
4. Login is performed through head server.
5. `session_token` is stored in `AuthContext`.
6. Auth success signal is emitted.
7. `NetworkManager` opens WebSocket.
8. WebSocket `Identity` sets `local_player_id`.

Runtime identity data:

- `guest_id`: guest identity used for guest auth
- `auth_token`: head/auth session token used in `Join`
- `local_player_id`: authoritative in-game identity from game server

## Why This Design Is Cleaner Than Before

Compared with the old boolean-driven flow:

- auth transitions are explicit
- retry behavior is centralized
- networking no longer starts auth
- auth/profile data is separated from auth orchestration
- the scene tree shows the auth workflow structure directly

Compared with a fully split 9-state FSM:

- fewer files
- related guest/login logic stays grouped
- less scene-tree and script overhead

## Relevant Files

- `game_client/Scripts/AuthContext.gd`
- `game_client/Scripts/auth/AuthStateMachine.gd`
- `game_client/Scripts/auth/AuthApiClient.gd`
- `game_client/Scripts/auth/states/AuthStateBase.gd`
- `game_client/Scripts/auth/states/BootstrapState.gd`
- `game_client/Scripts/auth/states/GuestIdentityState.gd`
- `game_client/Scripts/auth/states/LoginState.gd`
- `game_client/Scripts/auth/states/AuthenticatedState.gd`
- `game_client/Scripts/WorldSync.gd`
- `game_client/Scripts/NetworkManager.gd`
- `game_client/Scripts/Player.gd`
- `game_client/Scripts/PlayerInput.gd`
- `game_client/Scenes/main.tscn`
