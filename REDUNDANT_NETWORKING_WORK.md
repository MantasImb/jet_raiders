# The Redundancy Problem

This document explains the root cause of our server performance bottleneck. It goes beyond "memory allocations" to explain the architectural flaw of **Redundant Work**.

## 1. The Core Problem: O(N) Serialization

Currently, our server performs **N** serialization jobs for **1** actual game event, where N is the number of connected players.

If we have 100 players and a 60Hz tick rate:

- We produce **1** global state update.
- We serialize that identical state **100** separate times.
- We allocate memory for that state **100** separate times.

This happens 60 times a second. That is **6,000 serializations/sec** for the exact same data.

## 2. Where it happens in the code

The issue lies in how `game_server/src/net.rs` connects the Game Loop to the Client Loop.

### The Isolation

The function `run_client_loop` runs independently for every connection.

```rust
// game_server/src/net.rs
async fn run_client_loop(...) {
    loop {
        // ...
    }
}
```

### The Private Trigger

Inside that loop, every client listens to the broadcast channel individually:

```rust
// game_server/src/net.rs
world_msg = world_rx.recv() => { ... }
```

The `world_rx` channel broadcasts the **Source Data** (`WorldUpdate` struct), not the **Network Packet**.

### The Redundant Execution

Because each client receives the raw struct, each client must handle the conversion to text/bytes privately:

```rust
// game_server/src/net.rs (inside send_message)
let txt = serde_json::to_string(msg)?; // <-- EXECUTES N TIMES
```

## 3. Why migration to Binary (`u8`) won't fix this

A common misconception is that switching to a binary protocol (like `bincode` or `protocol.rs` binary buffers) eliminates this cost. **It does not.**

If we switch to binary but keep the current architecture:

1. **Current (JSON):** 100 clients = 100 allocations of `String` + 100 JSON serialization jobs.
2. **Migrated (Binary):** 100 clients = 100 allocations of `Vec<u8>` + 100 Binary serialization jobs.

You reduce the _weight_ of the CPU task (binary is faster than JSON), but you do not remove the **Recalculation**. You are still doing the work 100 times.

## 4. The Levels of Optimization

### Level 1: Failure (Current Status)

- **Architecture:** Broadcast Structs.
- **Memory:** Allocate new buffer every send.
- **Cost:** High Memory, High CPU.

### Level 2: Buffer Reuse (The "Patch")

- **Architecture:** Broadcast Structs.
- **Memory:** Reuse one `Vec<u8>` per client connection (never free it).
- **Cost:** Low Memory (no allocs), High CPU (still serializing N times).
- _Note: This was the original recommendation in `NETWORKING_ALLOCATIONS.md`._

### Level 3: "Broadcast the Bytes" (The Architectural Fix)

- **Architecture:** Broadcast **Serialized Bytes**.
- **Memory:** Allocate 1 buffer globally per tick.
- **Mechanics:**
  1. Game System produces `WorldUpdate` struct.
  2. Game System serializes it _once_ into an `Arc<Vec<u8>>` (or similar shared ref).
  3. Broadcast channel sends the `Arc<Vec<u8>>`.
  4. Client Loop receives reference.
  5. Client Loop calls `socket.send(ref)`.
- **Cost:** Low Memory, Lowest CPU (1 serialization per tick, regardless of player count).

## 5. Summary

The bottleneck isn't just that `to_string` is slow. It's that we are asking the server to translate the same sentence into English 100 times for 100 different people listening to the same speech.
