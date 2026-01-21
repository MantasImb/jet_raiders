# Auth Service Architecture Plan

## Purpose

Define a clean architecture layout for the auth service that supports the
current guest flow and leaves a clear path for Web3 authentication. This plan
aligns with the project clean architecture guidelines and keeps dependencies
pointing inward.

## Architecture Overview

The auth service is split into four layers:

- **Entities (Domain)**: core identity and session models.
- **Use Cases (Application)**: orchestration for guest login, token issuance,
  token verification, and logout.
- **Interface Adapters**: HTTP request/response mapping, DTO validation, and
  conversions between API models and domain models.
- **Frameworks/Drivers**: Axum routing, runtime setup, configuration, and
  persistence implementations.

Dependencies only flow inward. Inner layers never import framework or transport
code.

## Directory Layout (Proposed)

```text
auth_server/
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md
└── src/
    ├── main.rs                # Frameworks/Drivers: bootstrap only.
    ├── config.rs              # Frameworks/Drivers: env/config loading.
    ├── routes.rs              # Interface Adapters: HTTP routing.
    ├── handlers/
    │   ├── mod.rs
    │   ├── guest.rs            # Interface Adapters: request mapping.
    │   ├── tokens.rs           # Interface Adapters: verify/logout.
    │   └── web3.rs             # Interface Adapters: nonce/verify.
    ├── dto/
    │   ├── mod.rs
    │   ├── guest.rs            # Interface Adapters: request/response DTOs.
    │   └── web3.rs             # Interface Adapters: request/response DTOs.
    ├── app/
    │   ├── mod.rs
    │   ├── guest_login.rs      # Use Cases: guest session issuance.
    │   ├── verify_token.rs     # Use Cases: token verification.
    │   └── logout.rs           # Use Cases: token revocation.
    ├── domain/
    │   ├── mod.rs
    │   ├── identity.rs         # Entities: GuestIdentity, WalletIdentity.
    │   └── session.rs          # Entities: Session, SessionToken.
    ├── ports/
    │   ├── mod.rs
    │   ├── session_store.rs    # Use Cases: trait for persistence.
    │   ├── token_signer.rs     # Use Cases: trait for token issuance.
    │   └── clock.rs            # Use Cases: time abstraction.
    ├── infrastructure/
    │   ├── mod.rs
    │   ├── in_memory_store.rs  # Frameworks/Drivers: MVP store.
    │   └── jwt_signer.rs       # Frameworks/Drivers: token implementation.
    └── logging/
        └── mod.rs              # Frameworks/Drivers: tracing setup.
```

## Layer Responsibilities

### Entities (Domain)

- Own the canonical identity and session data structures.
- Define invariants and helper methods for core domain state.
- Avoid any dependency on HTTP, Axum, or serialization frameworks.

### Use Cases (Application)

- Orchestrate guest session issuance, token verification, and logout.
- Coordinate with ports to store sessions and sign tokens.
- Return domain-focused results that adapters can map to HTTP responses.

### Interface Adapters

- Validate HTTP inputs and map them into domain or use case inputs.
- Convert use case outputs into DTOs.
- Contain all request/response structs and serialization.

### Frameworks/Drivers

- Configure Axum, routing, and middleware.
- Provide concrete implementations for persistence and token signing.
- Wire dependencies and start the server.

## Data Flow (Guest Login)

1. `POST /auth/guest` hits the handler in `handlers/guest.rs`.
2. The handler validates DTO fields and calls `app::guest_login`.
3. The use case stores session data via `SessionStore` and issues a token via
   `TokenSigner`.
4. The handler maps the use case output into the guest response DTO.

## Data Flow (Token Verification)

1. `POST /auth/verify-token` hits `handlers/tokens.rs`.
2. The handler validates the token payload and calls `app::verify_token`.
3. The use case checks session state and returns the associated identity.
4. The handler maps the identity to the response DTO.

## Web3 Readiness

- Add `app::nonce_issue` and `app::verify_signature` for Web3 login.
- Introduce `ports::nonce_store` for one-time nonce handling.
- Keep the token issuance and session model consistent across guest and Web3
  flows.

## Clean Architecture Guardrails

- Domain and use cases never import Axum, JSON, or runtime types.
- DTOs live only in the adapter layer and are not stored in domain models.
- `main.rs` should only wire the application together and start the server.
- Persistence and token implementations stay in `infrastructure/` and are
  accessed through ports.
