# Solana Web3 Login Options for Jet Raiders Web Platform

## Overview

This document outlines options for implementing Solana-compatible web3 login
for the Jet Raiders web platform. The goal is to authenticate a player using a
Web3Auth-backed wallet experience and convey the verified identity to an
in-browser game client for server-authoritative gameplay.

Database design is out of scope. However, important storage considerations are
highlighted where they impact auth, security, or player identity management.

## Goals and Constraints

- **Wallet-based login**: Use a wallet signature to prove control of a
  blockchain address.
- **Browser-based game**: The game runs in the browser and must receive a
  verified identity or session token to authenticate to the game server.
- **Server-authoritative**: Server remains the source of truth for player
  identity and session validity.
- **Unified login**: Prefer a single Web3Auth integration rather than multiple
  wallet-specific flows.
- **Security**: Prevent replay attacks, spoofed identities, and session theft.

## Wallet Support Landscape (Web3Auth-First)

Web3Auth provides a unified login layer for Solana-compatible wallets and
embedded key management. The focus is a single integration path rather than
separate flows for Phantom, Solflare, or MetaMask.

### Web3Auth as the Primary Login

- Web3Auth handles wallet connection and key management behind a unified UI.
- Users can onboard with social/email logins or connect external wallets.
- The app receives a Solana-compatible key for message signing.

### External Wallets (Optional via Web3Auth)

- Phantom and Solflare can still be exposed through Web3Auth if desired.
- MetaMask requires a Solana-compatible bridge (such as a Snap) and should be
  treated as optional unless a stable Solana path is confirmed.

## High-Level Architecture Options

### Option A: Frontend-First Auth + Backend Session (Recommended)

1. Frontend requests a nonce from the backend.
2. Wallet signs a structured login message containing the nonce and domain.
3. Frontend sends the address + signature to the backend.
4. Backend verifies signature and issues a session token (cookie or JWT).
5. Game client uses the session token during WebSocket join/handshake.

Pros:

- Strong server control of identity and session lifecycle.
- Works well with server-authoritative game architecture.

Cons:

- Requires backend auth endpoints and signature verification.

### Option B: Direct Wallet-to-Game Server Auth

1. Frontend obtains nonce directly from game server.
2. Wallet signs the login message.
3. Game client sends signature to game server during `Join`.
4. Game server validates and establishes player identity.

Pros:

- Fewer moving parts for smaller stacks.

Cons:

- Game server takes on auth complexity.
- Harder to reuse sessions across web UI and game.

### Option C: Session Broker (Auth Service)

1. Dedicated auth service validates signatures and issues short-lived tokens.
2. Web UI and game server both validate tokens.

Pros:

- Clear separation of concerns.
- Scales to multiple services.

Cons:

- Extra infrastructure and deployment overhead.

## Recommended Flow (Option A, Web3Auth-First)

### 1) Nonce Retrieval

- Frontend requests a login nonce from the backend.
- Backend generates a cryptographically secure nonce and stores it with:
  - Wallet address (optional pre-binding if user selected wallet).
  - Issued-at timestamp.
  - Expiry timestamp (short, e.g., 5 minutes).
  - One-time use flag.

### 2) Wallet Signature (via Web3Auth)

- Frontend constructs a structured message including:
  - Domain or origin.
  - Wallet address.
  - Nonce.
  - Issued-at timestamp.
  - Optional statement: "Sign in to Jet Raiders".

Example message:

```
Jet Raiders wants you to sign in with your Solana account:
<address>

Domain: jet-raiders.example
Nonce: <nonce>
Issued At: <timestamp>
```

- Web3Auth signs the message using the Solana-compatible key.

### 3) Signature Verification

- Backend verifies:
  - Message matches expected format and domain.
  - Signature is valid for the claimed address.
  - Nonce is unused and within expiry window.

### 4) Session Issuance

- Backend creates a session token and associates it with the wallet address.
- Token is returned as:
  - HTTP-only secure cookie (recommended), or
  - Short-lived JWT passed to the client.

### 5) Game Join

- The in-browser game sends a `Join` message that includes:
  - Session token (or a short-lived auth ticket derived from the session).
- Game server validates the token and binds the player identity to the
  connection.

## Message Format and Standards

### Sign-In With Solana (SIWS)

- SIWS is a draft standard similar to SIWE (Sign-In With Ethereum).
- It defines a canonical message format and verification rules.
- If available in your stack, SIWS improves interoperability and security
  audits.

### Solana Wallet Standard

- Aims to unify wallet detection and connection interfaces.
- Use it to avoid vendor-specific integration issues.

## Frontend Integration Details

### Web3Auth Strategy

- Use Web3Auth React components for login and key management.
- Configure Solana chain parameters and the expected network.
- Expose a single login button to avoid multiple wallet-specific flows.

### Game Client Integration

- The web UI can run alongside the game canvas or in the same page.
- Communicate auth success to the game via:
  - JavaScript bridge into WebAssembly/Godot.
  - `postMessage` if the game is in an iframe.
- Pass only short-lived tokens, never raw private data.

## Backend Verification Details

### Signature Validation

- Use a Solana SDK to verify signatures.
- Verify the message bytes exactly as signed.
- Reject mismatched domains or stale timestamps.

### Session Management

- Keep sessions short and rotate tokens on refresh.
- Bind the session to a wallet address and optional device fingerprint.
- Allow explicit logout, which invalidates the session on the server.

## Database Considerations (Out of Scope)

While database design is out of scope, plan for storing:

- Wallet address as the primary identity key.
- Session records (token hash, expiry, device metadata).
- Player profile data linked to the wallet.
- Optional mapping of multiple wallets per user.

Ensure sensitive data is encrypted at rest and access is logged.

## Security Considerations

- **Nonce reuse**: Reject any nonce that has been used once.
- **Replay attacks**: Enforce short nonce expiry and verify timestamps.
- **Phishing**: Display clear sign-in statement and domain binding.
- **Token theft**: Use HTTP-only cookies when possible and enforce TLS.
- **Cross-site attacks**: Use CSRF protection for session endpoints.

## UX Considerations

- Provide clear user feedback when signature requests appear.
- Explain that signing is free and does not broadcast a transaction.
- Handle wallet disconnects and retries gracefully.
- Offer a fallback guest mode if desired.

## Operational Considerations

- Rate limit auth endpoints to prevent abuse.
- Monitor failed login attempts for suspicious patterns.
- Log auth events with correlation IDs for debugging.
- Use structured logging to tie wallet identity to game session IDs.

## Future Enhancements

- Add support for hardware wallets via Wallet Adapter.
- Introduce multi-factor auth for high-value accounts.
- Add optional OAuth link for non-crypto users.
- Expand to other chains if gameplay or marketplace features evolve.

## Summary

A wallet-signature-based login is the standard, secure way to authenticate
Solana users in a browser-based game. The recommended approach is a backend
session system that validates signatures, issues short-lived tokens, and
binds them to game connections during the join handshake.
