# Simple Auth Plan (Guest ID)

## Overview

This plan describes a minimal, insecure "guest ID" approach to identify a
returning user across sessions. It stores a randomly generated ID and a display
name in browser storage and sends them to the server on connect. This is for
early development only and should not be used for anything sensitive.

## Goals

- Remember a player between sessions with a locally stored ID.
- Persist basic player metadata (name, future user variables).
- Keep server changes minimal and avoid complex auth flows.
- Provide a clear upgrade path to real auth later.

## Non-Goals

- Security, identity verification, or account recovery.
- Preventing impersonation or spoofing.
- Cross-device identity linking.

## Data Model (Client)

- `guest_id`: random string (e.g., 128-bit base64url or hex).
- `display_name`: short string with basic validation.
- `client_flags`: optional JSON map for future client-only preferences.

Storage location:

- `localStorage` (simplest) or `IndexedDB` if size needs grow later.

## Data Model (Server)

- `guest_id`: string identifier for this connection.
- `display_name`: latest name supplied by the client.
- `last_seen`: timestamp to support basic cleanup.
- `metadata`: optional JSON blob for future non-sensitive fields.

Persistence options:

- In-memory map for MVP (lost on server restart).
- Lightweight DB table if you need durability.

## Client Flow

1. On first load, check storage for `guest_id`.
2. If missing, generate a new `guest_id` and store it.
3. Load `display_name` from storage or prompt user for one.
4. On game connect, send `guest_id` and `display_name` in `Join`.
5. On name change, update storage and send an update to server.

## Server Flow

1. On `Join`, accept `guest_id` and `display_name` as-is.
2. Bind `guest_id` to the connection for labeling and state lookup.
3. Store or update player metadata (name, last_seen, metadata).
4. Use `guest_id` for non-sensitive personalization only.

## Protocol Updates

Add fields to `Join`:

- `guest_id: String`
- `display_name: String`

Optional future message:

- `UpdateProfile { display_name, metadata }`

## Validation Rules

- `guest_id`: length limit, allow only base64url or hex charset.
- `display_name`: length limit and basic sanitization for UI.

## Risks and Limitations

- Anyone can spoof or copy another user's `guest_id`.
- Clearing browser storage resets identity.
- No security or trust guarantees.

## Migration Path to Real Auth

1. Keep `guest_id` for guest mode only.
2. Add wallet or OAuth login to issue server sessions.
3. Map authenticated accounts to existing guest data if desired.

## Checklist

- Client: generate + store `guest_id`.
- Client: include `guest_id` and `display_name` in `Join`.
- Server: accept and bind `guest_id` to connection.
- Server: store and update player metadata.
- Docs: mark this approach as insecure and temporary.

## Implementation Plan (Code Changes)

### Protocol Updates

- Add a `ClientMessage` enum to `game_server/src/interface_adapters/protocol.rs`
  with a `Join` variant.
- Include `guest_id` and `display_name` in the `Join` payload.
- Keep `PlayerInput` unchanged to avoid input handling churn.

### Server Changes

- Add a simple in-memory guest store (no persistence).
- Store `guest_id` -> `display_name` for future lookups.
- Require a `Join` message before accepting `PlayerInput`.
- Bind `guest_id` to the connection context for logging and lookup.
- Keep game loop logic the same (player identity is still `player_id`).

### Client Changes (Godot)

- Generate a `guest_id` once and store it in `localStorage`.
- Store and reuse `display_name` from the UI input.
- Send a `Join` message immediately after WebSocket connection opens.
- Keep input messages as they are, sent after `Join`.

### UI Notes

- The existing username input will populate `display_name`.
- No new UI is required for this phase.

### Compatibility Notes

- This change introduces a new handshake step; old clients will be rejected.
- No database or persistence is required.
