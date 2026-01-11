# Networking Allocations Deep Dive (Jet Raiders)

This document explains (in a junior-friendly way) why our current WebSocket sending
code allocates a lot, what that means, and what "sending `Utf8Bytes`" actually
solves.

It is specifically about this server code:

- `server/src/net.rs`
- `send_message(...)` and callers like `forward_world_update(...)`

## Quick glossary

### Allocation

An **allocation** is when the program asks the operating system / allocator for a new
chunk of memory on the heap.

In Rust, common things that allocate are:

- `String` (when it grows)
- `Vec<T>` (when it grows)
- `serde_json::to_string` (creates a new `String`)

Allocations are not "always bad", but doing *many allocations per second* in a hot
network loop can:

- increase CPU usage
- increase latency/jitter (spikes when the allocator does work)
- create more garbage for the allocator to manage

### Heap vs stack (very short)

- **Stack**: fast, fixed-size, used for most local variables.
- **Heap**: flexible-size, used for data that can grow (like `String`/`Vec`).

### Serialization

**Serialization** means converting a Rust value (like `ServerMessage`) into bytes.

In our current protocol, the bytes we send are **JSON text**.

## What we do today

In `server/src/net.rs`, our send path is roughly:

1. Convert `ServerMessage` to JSON
2. Send the JSON text via WebSocket

Simplified example:

```rust
let txt = serde_json::to_string(msg)?;
socket.send(Message::Text(txt.into())).await?;
```

### Why this allocates

`serde_json::to_string(msg)` must produce a `String` (owned, growable text). That
almost always means:

- allocate a new heap buffer
- write JSON into it
- return the `String`

If we do this for every world tick for every connected client, we end up allocating
*many* strings per second.

## What is `Utf8Bytes` and why it exists

Axum's WebSocket `Message::Text` uses the type `Utf8Bytes`.

`Utf8Bytes` is essentially "owned bytes that are guaranteed to be valid UTF-8".
Think of it like:

- similar to `String` (because it's UTF-8 text)
- but stored as bytes for efficient WebSocket handling

Axum provides conversions like:

- `Utf8Bytes::from(String)`
- `Utf8Bytes::try_from(Vec<u8>)` (checks the bytes are UTF-8)

So when you send:

```rust
socket.send(Message::Text(txt.into())).await?;
```

You are converting `String -> Utf8Bytes`.

### Important: `Utf8Bytes` does NOT remove JSON serialization

Even if you "switch to sending `Utf8Bytes`", you still need to produce the JSON
bytes from the Rust struct.

So there are two separate costs:

1. **serialization cost** (turning `ServerMessage` into JSON bytes)
2. **allocation / copying cost** (how you build and store those bytes)

Using `Utf8Bytes` can help with (2), but you still pay (1) unless you change the
protocol away from JSON.

## Why just switching types often doesn't help

If you do this:

```rust
let txt: String = serde_json::to_string(msg)?;
let utf8: Utf8Bytes = txt.into();
socket.send(Message::Text(utf8)).await?;
```

You still allocated the `String` from `to_string`.

So, this is mostly a "type cleanup" rather than a performance fix.

## The real allocation problem in our hot path

The hot path is typically:

- `world_rx.recv()` produces a `WorldUpdate`
- we wrap it into a `ServerMessage::WorldUpdate`
- we serialize it and send it

If the server ticks at 60 Hz and you have N clients:

- you might serialize + allocate ~60 × N messages per second

Even if each JSON blob is not huge, frequent allocations are the main issue.

## What we usually want instead: buffer reuse

### The idea

Instead of allocating a brand new `String` every time, reuse a buffer.

That means:

- allocate once (or only occasionally)
- keep capacity around
- write new JSON into the same buffer each send

In Rust, a buffer you can reuse is usually one of:

- `String`
- `Vec<u8>`

### Serializing without creating a `String`

Instead of:

```rust
let s = serde_json::to_string(&msg)?;
```

You can write JSON into an existing buffer:

```rust
out_buf.clear();
serde_json::to_writer(&mut out_buf, &msg)?;
```

Where `out_buf` is a `Vec<u8>` stored in your connection context.

This avoids the "create a brand new String" allocation every time.

## The tricky part: ownership when sending

A WebSocket send needs the message payload to be **owned** by the `Message` you
pass into `socket.send(...)`.

That means you cannot safely do:

- "serialize into a buffer" and then send a reference to that buffer

Because:

- the send is async
- the buffer might be mutated before the send finishes

So you generally have these options:

### Option A: move the buffer into the message

You can build an owned payload and move it into `Message::Text`.

For bytes:

- create a `Vec<u8>` containing JSON
- convert it into `Utf8Bytes`
- send it

But if you *move* the buffer out of your context, you don't have it anymore to
reuse.

### Option B: keep two buffers (double-buffering)

A common pattern is to have:

- `out_buf_a: Vec<u8>`
- `out_buf_b: Vec<u8>`

Then:

1. serialize into the "free" buffer
2. swap it into the message to send
3. keep the other buffer for the next serialization

This way, you avoid allocating every send, while still satisfying ownership.

### Option C: accept one allocation per send

If you keep using `to_string`, you're effectively doing this.

It's the simplest, but it is also the highest-allocation approach.

## What would remove serialization cost too

If we want to remove *both* allocation and JSON serialization overhead, we'd need a
protocol change:

- send binary data instead of JSON (e.g. `postcard`, `bincode`, etc.)
- potentially compress or delta-encode updates

That is a larger design decision because it affects:

- clients
- debugging tools
- backwards compatibility

## Recommendation for Jet Raiders (incremental)

If we want performance improvements without changing the protocol:

1. Keep JSON for now (still easy to debug).
2. Stop using `serde_json::to_string` in the hot path.
3. Use `serde_json::to_writer` into a reusable `Vec<u8>`.
4. Send `Message::Text(Utf8Bytes)` where the `Utf8Bytes` is constructed from owned
   bytes.
5. Use double-buffering to keep reuse benefits.

This is a good intermediate step before switching protocols.

## How to validate that an optimization helped

Good signs:

- CPU usage drops under many clients.
- Fewer latency spikes.
- Profiling shows fewer calls into allocator functions.

Simple local testing approach:

- connect multiple clients
- run with `RUST_LOG=info` (or `debug` temporarily)
- stress with high tick rate / lots of world updates

## Common beginner gotchas

- Reusing a buffer is safe only if you don't hand out references that live across an
  `await`.
- `Vec::clear()` does not free memory; it keeps capacity for reuse.
- `String` is UTF-8; `Vec<u8>` is bytes. JSON text is UTF-8, so both can work.
- A type change to `Utf8Bytes` alone doesn’t reduce allocations if you still call
  `serde_json::to_string`.
