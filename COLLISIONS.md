# Jet Raiders - Collisions

This document describes how collisions currently work in the server-authoritative
simulation.

## Overview

- Collisions are computed **server-side only** (Rust).
- Clients are “dumb terminals”: they render snapshots and do not perform
  authoritative hit detection.
- Current collision handling only logs hits and despawns the projectile (no health
  system yet).

## Collision shapes

We currently model collisions as **circle vs circle** in world space.

- Player circle radius: `PlayerTuning::radius`
- Projectile circle radius: `ProjectileTuning::radius`

Effective hit distance:

- `hit_radius = player_radius + projectile_radius`
- Hit occurs when squared distance `dx*dx + dy*dy <= hit_radius_sq`

## Where the code lives

All collision work is currently performed in the main world tick loop:

- `server/src/game.rs` (inside `world_task`)

The radii are sourced from tuning:

- `server/src/tuning/player.rs`
- `server/src/tuning/projectile.rs`

## Important behavior rules

### 1) Owner immunity

A projectile does not hit its owner:

```rust
if e.id == p.owner_id {
    continue;
}
```

### 2) Despawn-on-hit

When a hit is detected, we:

1. `println!` a hit message
2. mark the projectile for despawn by setting `p.ttl = 0.0`
3. remove it in the `retain` pass

### 3) Complexity

Collision checks are currently a naive nested loop:

- `O(P * E)` where `P = projectiles.len()`, `E = entities.len()`.

This is fine for small counts; later we can switch to spatial hashing / grid.

## Responsible snippets

### Projectile movement + TTL

File: `server/src/game.rs`

```rust
for p in &mut projectiles {
    p.x += p.vx * dt;
    p.y += p.vy * dt;
    p.ttl -= dt;
}
```

### Projectile vs player collision

File: `server/src/game.rs`

```rust
let hit_radius = player_radius + projectile_radius;
let hit_radius_sq = hit_radius * hit_radius;
for p in &mut projectiles {
    if p.ttl <= 0.0 {
        continue;
    }

    for e in &entities {
        if e.id == p.owner_id {
            continue;
        }

        let dx = e.x - p.x;
        let dy = e.y - p.y;
        if (dx * dx + dy * dy) <= hit_radius_sq {
            println!("Hit: player {} was hit by player {} projectile", e.id, p.owner_id);
            p.ttl = 0.0;
            break;
        }
    }
}

projectiles.retain(|p| p.ttl > 0.0);
```

### Player/Projectile radii

File: `server/src/tuning/player.rs`

```rust
pub struct PlayerTuning {
    // ...
    pub radius: f32,
}
```

File: `server/src/tuning/projectile.rs`

```rust
pub struct ProjectileTuning {
    // ...
    pub radius: f32,
}
```

## Notes / next steps

- We currently despawn projectiles on hit; later we can add a `GameEvent` (or a
  dedicated combat system) to apply damage and broadcast hit events.
- We do not yet handle projectile-projectile collisions or world collisions.
