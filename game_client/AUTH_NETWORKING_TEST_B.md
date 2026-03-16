# Auth and Networking Test B

Use this as a second-pass memory test. It covers the same auth and networking
system from a debugging and architecture angle instead of a pure sequence
recall angle.

## Session

- Date:
- Time spent:
- Confidence before starting (1-5):
- Confidence after finishing (1-5):

## Part 1: Architecture From Memory

Answer without looking up the files.

1. Why was the old `UserManager` split apart?

   ________________________________________________________________

2. What data belongs in `AuthContext` and should not be owned by
   `NetworkManager`?

   ________________________________________________________________

3. What behavior belongs in `WorldSync` and should not live in transport code?

   ________________________________________________________________

4. Why is `local_player_id` not the same thing as `auth_token`?

   ________________________________________________________________

5. Which component is the source of truth for auth lifecycle state?

   ________________________________________________________________

## Part 2: State and Substate Drill

Fill in the missing names.

| Situation | Correct state or substate |
| --- | --- |
| Startup reset state | __________________ |
| Profile is being loaded | __________________ |
| Guest init HTTP request is in flight | __________________ |
| Waiting to retry guest init | __________________ |
| Guest init has fully exhausted retries | __________________ |
| Login is ready or auto-starting | __________________ |
| Login HTTP request is in flight | __________________ |
| Waiting to retry login | __________________ |
| Login has fully exhausted retries | __________________ |
| Auth is complete | __________________ |

## Part 3: Failure Matrix

Fill in the expected system behavior.

| Failure or event | What happens next? |
| --- | --- |
| Stored `guest_id` is invalid | ________________________________ |
| `/guest/init` response has no `guest_id` | ________________________________ |
| `/guest/login` response has no `session_token` | ________________________________ |
| Login starts but `guest_id` is missing | ________________________________ |
| Socket closes after a successful connection | ________________________________ |
| Socket fails while connecting | ________________________________ |
| World snapshot omits an existing player | ________________________________ |
| World snapshot omits an existing projectile | ________________________________ |

## Part 4: Ownership and Input

Answer briefly.

1. Which exact `NetworkManager` method does `PlayerInput` use to decide if the
   socket is usable?

   `__________________`

2. What second condition must be true before a `Player` sends input?

   ________________________________________________________________

3. Where does the value for local ownership come from originally?

   ________________________________________________________________

4. Which server message sets that value?

   `__________________`

## Part 5: Explain the Data Flow

Write 2-4 sentences for each prompt.

### Prompt A

Explain the difference between these three fields and where each one is used:

- `guest_id`
- `auth_token`
- `local_player_id`

Answer:

_______________________________________________________________

_______________________________________________________________

_______________________________________________________________

### Prompt B

Explain the reconnect rules in test mode, including:

- when reconnect is allowed
- which states block duplicate reconnect work
- what happens after reconnect succeeds

Answer:

_______________________________________________________________

_______________________________________________________________

_______________________________________________________________

## Part 6: Practical Scenarios

Write what you would inspect first and why.

1. The client never reaches the WebSocket join step.

   First thing to inspect:
   _______________________________________________

   Why:
   ________________________________________________________________

2. The socket connects, but local input never affects the ship.

   First thing to inspect:
   _______________________________________________

   Why:
   ________________________________________________________________

3. Other players keep remaining in the scene after they disappear from the
   authoritative snapshot.

   First thing to inspect:
   _______________________________________________

   Why:
   ________________________________________________________________

4. Login never recovers after repeated HTTP failures until the user presses a
   button.

   Which substate is this?
   `__________________`

   Which public method restarts the flow?
   `__________________`

## Part 7: Self-Check

Mark each item `Strong`, `Weak`, or `Need Review`.

| Topic | Rating |
| --- | --- |
| Auth state flow | __________ |
| Retry behavior | __________ |
| Join and identity messages | __________ |
| World snapshot handling | __________ |
| Local input ownership rules | __________ |
| Reconnect behavior | __________ |

## Part 8: Review Targets

- What should you review before the next repetition?
- Which 3 names or transitions should be memorized exactly?
