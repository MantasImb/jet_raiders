# Game Specification

This document describes the base gameplay mechanics of **Jet Raiders** from a
player’s perspective.

## Core objective

Fly your plane, shoot other players, and stay alive.

## Player controls and movement

- Your plane is always moving based on **throttle**.
- You can **turn** left/right to change your heading.
- The game uses fast, arcade-style movement intended for quick dogfights.

## Shooting

- You can fire a stream of projectiles while the weapon is off cooldown.
- Projectiles travel forward from the plane’s nose.
- Projectiles are server-authoritative: hits are determined by the server.

## Health and damage

- Each plane has **100 HP**.
- A projectile deals **30 damage** on hit.
- A hit reduces the victim’s HP immediately.

## Death and respawn

- When your HP reaches **0**, your plane is destroyed.
- Your plane will **respawn after 1 second** at a new position.
- After respawning, your HP is restored to full.

## Multiplayer and fairness (high-level)

- The server runs the authoritative simulation.
- Clients primarily render the latest server snapshots.
- Collision/hit detection is performed on the server to reduce cheating and
  desync.

## Terminology

- **HP**: Health points; when this reaches 0, you die.
- **Projectile**: A shot fired from a plane that can damage other players.

### Possible feature ideas
- [ ] A passive skill for the shooting ability where another player dies if they get hit by 3 shots from a player.