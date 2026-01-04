# Importing Logic with Clean Architecture

> Note: For the authoritative "where does this logic go?" rules, see
> `CLEAN_ARCHITECTURE_GUIDELINES.md`.

Use this guide to move logic out of `server/src/main.rs` into the modules defined in
`ARCHITECTURE.md` while keeping a clean architecture separation.

## Clean architecture boundaries for this project

- **Entities (Core):** Domain data and rules in `state.rs` and `systems/`.
- **Use cases (Application):** Game loop orchestration in `game.rs` and lobby control in
  `lobby.rs`.
- **Interface adapters:** WebSocket handling, protocol translation, and channel plumbing in
  `net.rs` and `protocol.rs`.
- **Frameworks/Drivers:** Axum setup, config loading, and runtime bootstrap in `main.rs` and
  `config.rs`.
- Dependency rule: outer layers depend inward only; core never imports web, Axum, or Tokio
  specifics.

## Migration steps from `main.rs`

1. **Identify responsibilities:** Split bootstrapping, routing, lobby management, game loop
   control, and message encoding/decoding.
2. **Create boundaries:** Define traits or structs for lobby management and networking so the
   game loop only consumes typed channels and domain data.
3. **Move orchestration:** Put WebSocket route handlers and lobby routing into `net.rs`;
   move lobby registry, spawn logic, and lifecycle handling into `lobby.rs`.
4. **Centralize domain state:** Keep `GameState`, `Player`, and `Projectile` in `state.rs` and
   ensure systems consume only these types.
5. **Systemize logic:** Place movement and combat updates in `systems/movement.rs` and
   `systems/combat.rs`; expose `update` functions that operate on `GameState`.
6. **Game loop isolation:** In `game.rs`, run the tick loop that drains input, invokes systems,
   and returns snapshots; it should not know about Axum or sockets.
7. **Protocol clarity:** Keep serialization enums and DTOs in `protocol.rs`; `net.rs` should be
   the only layer translating between wire messages and domain inputs.
8. **Bootstrap only:** Leave `main.rs` with runtime setup, router construction, config loading,
   channel wiring, and task spawning—no game logic.

## Wiring checklist

- Does `main.rs` only compose components and start the server?
- Do systems depend only on domain types, never on networking or tasks?
- Is the protocol layer the only place that touches serde and message schemas?
- Are lobby and game loop ownership/lifetimes clear, with channels passed in from the
  bootstrap layer?

## Verification

- Run existing tests and a manual smoke test (connect, join lobby, move, shoot).
- If a module now imports a framework type, reconsider the dependency direction.
