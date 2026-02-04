# Communication Considerations (Game Server)

This note captures security and routing considerations for the game server's
public WebSocket interface and the internal HTTP routes intended for the head
service.

## Goals

- Allow public WebSocket connections from game clients.
- Restrict internal HTTP routes so only the head service can access them.
- Keep request and error handling consistent across adapters.

## Current Behavior

- Internal HTTP handlers live in `interface_adapters/net/internal.rs`.
- Lobby creation requires the head service to provide a `lobby_id`.
- Internal routes return JSON error payloads using `ErrorResponse`.

## Threat Model (High Level)

- Any external user can reach the game server's HTTP and WebSocket ports.
- Internal routes such as lobby creation can be abused if not authenticated.
- The WebSocket endpoint is intentionally public and must be resilient against
  abuse (rate limiting, payload validation, and circuit breakers).

## Recommended Protections for Internal Routes

1) **Shared secret + HMAC**  
   - Head service signs requests (method + path + body + timestamp).  
   - Game server verifies the signature and rejects unsigned or stale requests.  
   - Use a short replay window (for example, 30 seconds) and require a nonce.

2) **mTLS between services**  
   - Terminate TLS at the game server and require a client certificate.  
   - Only the head service possesses a valid client cert.  
   - Works best with a service mesh or a trusted internal PKI.

3) **Network-level isolation**  
   - Put internal routes on a private listener or internal load balancer.  
   - Restrict ingress with firewall rules (security groups, VPC, etc.).  
   - This can be paired with app-layer auth for defense in depth.

4) **Dedicated internal port**  
   - Expose internal routes on a separate port bound to a private interface.  
   - Keep the public WebSocket endpoint on a public interface.

## Recommended Runtime Checks

- Enforce an `X-Request-Id` and log it for all internal routes.  
- Validate `Content-Type` and size limits for JSON payloads.  
- Return structured JSON errors for all internal routes.

## Suggested Implementation Approach

- Add an internal auth middleware in `interface_adapters/net/internal.rs`.  
- Configure a shared secret in `frameworks/config.rs`.  
- Apply the middleware to internal routes only.  
- Keep the WebSocket route open and unauthenticated.

## Open Questions

- Should internal routes be exposed on a separate port or share the public one?  
- Do we prefer HMAC or mTLS in the current deployment environment?
