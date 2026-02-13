# SOLID Principles in Jet Raiders (In Depth)

This document explains each SOLID principle and maps it to concrete patterns in
this repository. The goal is to help you connect theory to the code you have
right now, and also identify where a principle can be strengthened.

## How to read this guide

For each principle, you will see:

1. **What the principle means** in practical engineering terms.
2. **Where it appears today** in this codebase.
3. **Where it is missing or weak**.
4. **How to implement or improve it** here.
5. **Why that helps** in day-to-day development.

---

## S — Single Responsibility Principle (SRP)

### Core idea

A module should have one primary reason to change. In practice, that means each
module should focus on one concern (domain logic, transport, orchestration,
persistence, etc.) rather than combining unrelated concerns.

### Where SRP is already used in this codebase

#### 1) Auth domain ports vs adapter implementations

- `auth_server/src/domain/ports.rs` defines narrow interfaces (`SessionStore`,
  `Clock`).
- `auth_server/src/interface_adapters/state.rs` provides concrete adapters
  (`InMemorySessionStore`, `SystemClock`).
- `auth_server/src/use_cases/*` consumes the abstractions and focuses on auth
  behavior.

This is good SRP because auth behavior changes (token policy, validation) do not
require changing how time is read or where sessions are stored.

#### 2) Game domain systems split by behavior

- `game_server/src/domain/systems/ship_movement.rs` handles only movement.
- `game_server/src/domain/systems/projectiles.rs` handles only projectile
  lifecycle and collisions.
- `game_server/src/use_cases/game.rs` orchestrates the tick loop and calls these
  systems.

This keeps movement tuning changes isolated from projectile/collision changes.

#### 3) Matchmaking domain and use-case separation

- `matchmaking_server/src/domain/queue.rs` defines queue entities and id
  builders.
- `matchmaking_server/src/use_cases/matchmaker.rs` defines the matching workflow
  and outcomes.

Even in an MVP, this split helps prevent transport concerns from leaking into
core matchmaking behavior.

### Where SRP can be improved

- `game_server/src/use_cases/game.rs` currently owns many responsibilities:
  lobby events, respawn logic, timing, world update composition, and use-case
  orchestration.
- `head_server/src/domain/auth.rs` includes `serde` DTO coupling in the domain
  file (noted by its own comment).

### Implementation direction (concrete)

1. Extract responsibilities from `world_task` into focused helpers:
   - `spawn_player(...)`
   - `apply_respawn(...)`
   - `drain_input_events(...)`
   - `build_world_update(...)`
2. Move transport DTO serialization types away from domain contracts in
   `head_server` into interface adapter protocol modules.

### Why this helps

- Smaller files are easier to reason about and test.
- Changes become safer because fewer unrelated behaviors sit in one function.
- New contributors can find where to edit faster.

---

## O — Open/Closed Principle (OCP)

### Core idea

Software should be open for extension but closed for modification. You should be
able to add a new behavior by adding new code, not by repeatedly editing stable
core logic.

### Where OCP is already used in this codebase

#### 1) Auth storage and time are extension points

Use cases are generic over traits:

- `GuestLoginUseCase<C, S>` in `auth_server/src/use_cases/guest_login.rs`
- `VerifyTokenUseCase<C, S>` in `auth_server/src/use_cases/verify_token.rs`
- `LogoutUseCase<S>` in `auth_server/src/use_cases/logout.rs`

Because these depend on `Clock` and `SessionStore` traits, you can add:

- Redis-backed `SessionStore`
- SQL-backed `SessionStore`
- deterministic `Clock` for tests

without rewriting auth business rules.

#### 2) Head server auth client abstraction

`head_server/src/domain/auth.rs` defines `AuthProvider`, and
`head_server/src/interface_adapters/clients.rs` implements it with `AuthClient`.
You can add a second provider (for example, a mock provider or alternate auth
backend) by implementing the trait.

### Where OCP can be improved

- `matchmaking_server/src/use_cases/matchmaker.rs` hardcodes region-only
  matching logic.
- `game_server/src/use_cases/game.rs` hardcodes some match-flow policy in one
  loop.

### Implementation direction (concrete)

1. Introduce a `MatchPolicy` trait in matchmaking domain:
   - `find_opponent(queue, request) -> Option<index>`
2. Keep current region policy as default implementation.
3. Add alternative policies (region+skill window, latency-aware policy) by new
   implementations instead of editing the base matchmaker each time.

### Why this helps

- Feature growth (ranked mode, region fallback) does not destabilize existing
  code.
- Product experiments become cheap: add implementation, wire it, evaluate.

---

## L — Liskov Substitution Principle (LSP)

### Core idea

Subtypes (or trait implementations) must be usable through the base abstraction
without breaking expected behavior. If a use case expects `SessionStore`, any
`SessionStore` should behave correctly under that contract.

### Where LSP is already used in this codebase

#### 1) SessionStore substitution in auth

Auth use cases call `insert/get/remove` through `SessionStore`. The in-memory
adapter in `auth_server/src/interface_adapters/state.rs` matches that contract,
so use cases do not care about concrete type.

#### 2) Clock substitution in auth

`SystemClock` is one implementation of `Clock`. A fake/test clock can replace it
while preserving semantics (`now_epoch_seconds`).

#### 3) AuthProvider substitution in head server

Handlers call `create_guest_session` through `AuthProvider`, so another
implementation can be swapped in if it obeys the same request/response
expectations.

### Common LSP risks to watch for

- Returning different error semantics between store implementations.
- Silent behavior differences, such as one store auto-expiring tokens and
  another not.
- Stronger preconditions in one implementation (for example, rejecting token
  formats another accepts).

### Implementation direction (concrete)

Define contract tests that every `SessionStore` implementation must pass:

- insert then get returns the same session
- remove returns true only when item existed
- get on missing token returns `Ok(None)`

This turns LSP from assumption into enforceable behavior.

### Why this helps

- Swapping adapters no longer causes surprising runtime behavior.
- Production migrations (in-memory to Redis/SQL) become lower risk.

---

## I — Interface Segregation Principle (ISP)

### Core idea

Clients should not depend on methods they do not use. Prefer small, focused
interfaces over one large "do everything" interface.

### Where ISP is already used in this codebase

#### 1) Auth ports are intentionally narrow

`SessionStore` only exposes session operations, while `Clock` only exposes time.
Auth use cases consume only what they need.

#### 2) Focused provider contract in head server

`AuthProvider` has one method for guest session creation, matching the specific
responsibility needed by the handler.

### Where ISP can be improved

As more features are added, there is a risk that `AuthProvider` grows into a
large multi-method interface (guest login, refresh, revoke, profile, etc.) that
forces consumers to depend on too much.

### Implementation direction (concrete)

Split interfaces by use-case context when growth appears:

- `GuestSessionIssuer`
- `TokenVerifierClient`
- `SessionRevokerClient`

Each handler then depends on exactly one narrow contract.

### Why this helps

- Smaller interfaces are easier to implement and mock.
- Changes in one API area do not break unrelated consumers.

---

## D — Dependency Inversion Principle (DIP)

### Core idea

High-level modules (business/use-case logic) should not depend on low-level
modules (framework details). Both should depend on abstractions.

### Where DIP is already used in this codebase

#### 1) Auth use cases depend on abstract ports

- High-level: `auth_server/src/use_cases/*`
- Abstractions: `auth_server/src/domain/ports.rs`
- Low-level implementations: `auth_server/src/interface_adapters/state.rs`

This is a direct DIP implementation.

#### 2) Head handler depends on domain abstraction

- High-level handler: `head_server/src/interface_adapters/handlers/guest.rs`
- Abstraction: `head_server/src/domain/auth.rs` (`AuthProvider`)
- Low-level detail: `head_server/src/interface_adapters/clients.rs`
  (`AuthClient` using `reqwest`)

The handler is protected from `reqwest`-specific details.

### Where DIP can be improved

- `head_server/src/domain/auth.rs` still carries `serde` dependency
  for request and response DTOs, which blurs boundary ownership.
- Some game orchestration is still tightly coupled in a single function, making
  policy injection harder.

### Implementation direction (concrete)

1. Move serialization-focused DTOs to interface adapter protocol modules.
2. Keep domain traits and domain models serialization-agnostic when possible.
3. Inject policies/services (e.g., match end conditions, spawn policy) via
   traits into use-case orchestration.

### Why this helps

- Domain logic becomes easier to reuse and test without HTTP/JSON framework
  baggage.
- Infrastructure changes (`reqwest` replacement, protocol updates) have reduced
  blast radius.

---

## Cross-principle observations (how SOLID works together here)

1. **DIP enables OCP**:
   because use cases depend on traits, new adapters can be added without editing
   business logic.
2. **SRP supports ISP**:
   when modules stay focused, interfaces naturally stay small and coherent.
3. **ISP supports LSP**:
   narrow contracts are easier to substitute correctly than broad contracts with
   many hidden assumptions.

Together, these patterns are visible mostly in auth and partially in head/game.
Auth currently shows the strongest SOLID maturity in this repository.

---

## Practical roadmap for this repository

If you want to strengthen SOLID incrementally (without large rewrites), this is
an effective order:

1. Add contract tests for `SessionStore` (LSP + DIP).
2. Split `world_task` helpers in game server (SRP).
3. Introduce matchmaking policy strategy trait (OCP + DIP).
4. Keep head domain contracts serialization-light (DIP + SRP).
5. Preemptively split provider traits if auth API surface grows (ISP).

This sequence gives immediate quality gains while staying compatible with the
current architecture and momentum.

---

## Final takeaway

The codebase already demonstrates meaningful SOLID usage, especially around
port-and-adapter boundaries in `auth_server` and abstraction use in
`head_server`. The main opportunity is to apply the same rigor to larger
orchestration areas (`game_server` and future matchmaking policies). Doing that
will improve maintainability, testing confidence, and iteration speed as the
project grows.
