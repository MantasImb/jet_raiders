# Jet Raiders - Collisions

This document describes how collisions currently work in the server-authoritative
simulation.

## Overview

- Collisions are computed **server-side only** (Rust).
- Clients are “dumb terminals”: they render snapshots and do not perform
  authoritative hit detection.
- Current collision handling applies damage on hit and despawns the projectile.

## Collision shapes

We currently model collisions as **circle vs circle** in world space.

- Player circle radius: `PlayerTuning::radius`
- Projectile circle radius: `ProjectileTuning::radius`

Effective hit distance:

- `hit_radius = player_radius + projectile_radius`
- Hit occurs when squared distance `dx*dx + dy*dy <= hit_radius_sq`

## Where the code lives

Collision resolution is implemented in the projectile system:

- `game_server/src/domain/systems/projectiles.rs` (`tick_projectiles`)

The world task calls this system every tick:

- `game_server/src/use_cases/game.rs`

The radii are sourced from tuning:

- `game_server/src/domain/tuning/player.rs`
- `game_server/src/domain/tuning/projectile.rs`

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

1. emit a structured `tracing::info!` hit event
2. mark the projectile for despawn by setting `p.ttl = 0.0`
3. remove it in the `retain` pass

### 3) Complexity

Collision checks are currently a naive nested loop:

- `O(P * E)` where `P = projectiles.len()`, `E = entities.len()`.

This is fine for small counts; later we can switch to spatial hashing / grid.

## Responsible snippets

### Projectile movement + TTL

File: `game_server/src/domain/systems/projectiles.rs`

```rust
for p in &mut projectiles {
    p.x += p.vx * dt;
    p.y += p.vy * dt;
    p.ttl -= dt;
}
```

### Projectile vs player collision

File: `game_server/src/domain/systems/projectiles.rs`

```rust
let hit_radius = player_radius + projectile_radius;
let hit_radius_sq = hit_radius * hit_radius;
for p in &mut projectiles {
    if p.ttl <= 0.0 {
        continue;
    }

    for e in entities.iter_mut() {
        if !e.alive {
            continue;
        }
        if e.id == p.owner_id {
            continue;
        }

        let dx = e.x - p.x;
        let dy = e.y - p.y;
        if (dx * dx + dy * dy) <= hit_radius_sq {
            e.hp -= projectile_damage;
            if e.hp <= 0 {
                e.hp = 0;
                e.alive = false;
                e.respawn_timer = respawn_delay;
                e.throttle = 0.0;
                e.shoot_cooldown = 0.0;
            }
            info!(
                victim_id = e.id,
                shooter_id = p.owner_id,
                projectile_id = p.id,
                victim_hp = e.hp,
                "player hit"
            );
            p.ttl = 0.0;
            break;
        }
    }
}

projectiles.retain(|p| p.ttl > 0.0);
```

### Player/Projectile radii

File: `game_server/src/domain/tuning/player.rs`

```rust
pub struct PlayerTuning {
    // ...
    pub radius: f32,
}
```

File: `game_server/src/domain/tuning/projectile.rs`

```rust
pub struct ProjectileTuning {
    // ...
    pub radius: f32,
}
```

## Notes / next steps

- We currently despawn projectiles on hit and apply damage server-side.
- When a player's HP reaches 0, their plane despawns and respawns after 1 second.
- We do not yet handle projectile-projectile collisions or world collisions.
