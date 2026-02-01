# Challenges and Solutions

This document tracks notable development challenges and their solutions.

## Challenge 1: Redundant per-client serialization

### Challenge

The server currently performs the same serialization work once per connected
client. For each world tick, the same update is converted into JSON for every
connection, which scales linearly with player count and becomes a hot CPU and
allocation path.

### Where it happens

- `game_server/src/interface_adapters/net.rs` runs one client loop per
  connection.
- Each loop receives a `WorldUpdate` (the source struct), not a prebuilt
  network payload.
- Each loop then serializes that struct independently, which repeats work N
  times for N clients.

Example (per-connection loop):

```rust
// Each connection runs its own loop.
async fn run_client_loop(...) {
    loop {
        // Each connection receives the same world update.
        world_msg = world_rx.recv() => {
            // Each connection serializes the same data again.
            let txt = serde_json::to_string(&world_msg)?;
            socket.send(Message::Text(txt.into())).await?;
        }
    }
}
```

### Why it is costly

- The hot path is `serde_json::to_string`, which allocates a new `String` for
  every send.
- With a 60 Hz tick rate and many clients, this means thousands of allocations
  and serializations per second.
- Allocator pressure adds latency spikes and wastes CPU on repeated work that
  is identical across clients.

Example (allocation-heavy send):

```rust
// This allocates a new String every time.
let txt = serde_json::to_string(msg)?;
socket.send(Message::Text(txt.into())).await?;
```

### Misconceptions that do not fix it

- Switching to `Utf8Bytes` does not remove serialization cost. It only changes
  the message container and still requires building JSON text first.
- Switching to a binary protocol reduces the cost per serialization, but does
  not eliminate the redundant N-times-per-tick work if the architecture stays
  the same.

Example (type change only, still serializes):

```rust
// Utf8Bytes still needs JSON bytes from serialization.
let txt = serde_json::to_string(msg)?;
let utf8: Utf8Bytes = txt.into();
socket.send(Message::Text(utf8)).await?;
```

### Constraints and tradeoffs

- WebSocket sends require ownership of the message buffer, so a simple
  "serialize into a shared buffer and send a reference" is not safe across
  `await` boundaries.
- Buffer reuse (for example, double-buffering with `Vec<u8>`) reduces
  allocations but still keeps the N-per-tick serialization cost.

Example (buffer reuse still serializes per client):

```rust
// Reuse a buffer, but serialization still happens per connection.
out_buf.clear();
serde_json::to_writer(&mut out_buf, &msg)?;
let utf8 = Utf8Bytes::try_from(out_buf.clone())?;
socket.send(Message::Text(utf8)).await?;
```

### Solution

Introduce a dedicated adapter-layer task that receives `WorldUpdate` once per
tick, serializes it once into shared bytes, and broadcasts those bytes to all
connections. Each client loop then sends the shared bytes without redoing
serialization.
