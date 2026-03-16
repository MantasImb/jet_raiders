# Auth and Networking Test A

Use this as a fill-in test after reviewing the current auth and networking
refactor in `game_client/`.

## Session

- Date:
- Time spent:
- Confidence before starting (1-5):
- Confidence after finishing (1-5):

## Part 1: Responsibility Mapping

Write the main responsibility of each script or node.

| Component | Main responsibility |
| --- | --- |
| `AuthContext.gd` | ________________________________ |
| `AuthStateMachine.gd` | ________________________________ |
| `AuthApiClient.gd` | ________________________________ |
| `GuestIdentityState.gd` | ________________________________ |
| `LoginState.gd` | ________________________________ |
| `AuthenticatedState.gd` | ________________________________ |
| `NetworkManager.gd` | ________________________________ |
| `WorldSync.gd` | ________________________________ |
| `PlayerInput.gd` | ________________________________ |
| `Player.gd` | ________________________________ |

## Part 2: Happy Path Sequence

Fill in the normal startup flow from boot to realtime gameplay.

1. `AuthStateMachine` starts in `__________________`.
2. It immediately transitions to `__________________`.
3. Profile data is loaded from `__________________`.
4. If `guest_id` already exists and is valid, the next top-level state is
   `__________________`.
5. If `guest_id` is missing, the client calls `POST __________________`.
6. The response field that must be validated and stored is
   `__________________`.
7. Login uses `POST __________________`.
8. The response field that must be validated and stored is
   `__________________`.
9. The auth success state is `__________________`.
10. The signal emitted to networking is `__________________`.
11. `NetworkManager` opens a `__________________` connection.
12. After the socket opens, it sends a `__________________` message.
13. That message includes the auth field `__________________`.
14. The server responds with `__________________`, then with
    `__________________`.
15. The field used to identify the local in-game entity is
    `__________________`.

## Part 3: Message Contract Recall

Fill in the important fields only.

### Join

```json
{
  "type": "__________",
  "data": {
    "________________": "..."
  }
}
```

### Input

```json
{
  "type": "__________",
  "data": {
    "__________": 1.0,
    "__________": 0.0,
    "__________": true
  }
}
```

### Identity

```json
{
  "type": "__________",
  "data": {
    "________________": "..."
  }
}
```

### WorldUpdate

List the top-level fields that matter:

- `__________________`
- `__________________`
- `__________________`

## Part 4: Retry and Failure Recall

Fill in the blanks.

1. Guest init retries are tracked by the key `__________________`.
2. Login retries are tracked by the key `__________________`.
3. The shared base retry delay is `__________________`.
4. The shared max retry delay is `__________________`.
5. Guest init terminal substate: `__________________`.
6. Login terminal substate: `__________________`.
7. The event used to resume a scheduled retry is `__________________`.
8. The public method used for manual retry from the UI is
   `__________________`.

## Part 5: Networking and World Sync Rules

Answer in one short sentence each.

1. Why does `NetworkManager` wait for auth before auto-connecting?

   ________________________________________________________________

2. What does `WorldSync` do when a player exists in the latest snapshot but
   not in the scene tree?

   ________________________________________________________________

3. What does `WorldSync` do when a projectile node is missing from the latest
   snapshot?

   ________________________________________________________________

4. When does `PlayerInput` send input to the server?

   ________________________________________________________________

5. How does `Player.gd` know that a spawned player is the local player?

   ________________________________________________________________

## Part 6: Short Reflection

- Which part did you miss most often?
- Which state or message name still feels weak?
- What should you review before taking Test B?
