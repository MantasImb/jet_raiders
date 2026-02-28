# Jet Raiders - Physics Model

This document defines the minimal physics and movement logic used in Jet Raiders.
The server will implement these rules to update the game state authoritatively.

## 1. Coordinate System

- **2D Cartesian Plane**: Standard 2D space.
- **Forward Direction**: The "Up" direction (negative Y) is considered forward,
  consistent with Godot's 2D rotation.
- **Rotation**: Measured in radians. Clockwise is positive.

## 2. Player Movement Physics

The player movement is **not** Newtonian (no drifting/sliding). It mimics a
"throttle-based" flight model where velocity is always aligned with the ship's
forward direction.

### Player State Variables

- `position` (Vec2): Current world coordinates (x, y).
- `rotation` (float): Current facing angle in radians.
- `throttle` (float): A value between `0.0` and `1.0` representing current
  engine power.

### Constants (Configurable)

- `MAX_SPEED`: Maximum pixels per second (default `150.0`).
- `TURN_RATE`: Rotation speed in radians per second (default `3.0`).
- `THROTTLE_RATE`: How fast throttle increases/decreases per second (default `2.0`).

### Client Input Packet (Per Tick)

Client-to-server inputs should match `game_client/CLIENT_NETWORKING.md`.

```json
{
  "thrust": 1.0,
  "turn": 0.0,
  "shoot": true
}
```

For movement, the server uses:

- `thrust` (f32): throttle change input in `[-1.0, 1.0]` (from Godot `Input.get_axis`).
- `turn` (f32): turn input in `[-1.0, 1.0]`.

### Update Logic (Per Tick)

1. **Rotation**:

   ```rust
   rotation += input.turn * TURN_RATE * delta_time;
   ```

2. **Throttle**:

   ```rust
   throttle += input.thrust * THROTTLE_RATE * delta_time;
   throttle = clamp(throttle, 0.0, 1.0);
   ```

3. **Velocity Calculation**:
   - The velocity vector is derived purely from rotation and throttle.
   - With **0 rad = up / -Y** and **clockwise-positive rotation** (Godot 2D), the forward vector is:
     - `direction = Vec2::new(sin(rotation), -cos(rotation))`
   - `velocity = direction * throttle * MAX_SPEED`
4. **Position Update**:

   ```rust
   position += velocity * delta_time;
   ```

## 3. World Boundaries (Wrapping)

The game world is a torus (loops around).

- **Logic**: If a player moves past a border, they teleport to the opposite side.
- **Server implementation**: currently hardcoded in
  `game_server/src/use_cases/game.rs` (then passed into `MovementConfig`).
- **Client rendering note**: because wrapping is a teleport, client-side
  interpolation should **snap** on large jumps (otherwise the ship will lerp
  across the whole screen).

- **Implementation**:

  ```rust
  if position.x < MIN_X { position.x = MAX_X; }
  else if position.x > MAX_X { position.x = MIN_X; }

  if position.y < MIN_Y { position.y = MAX_Y; }
  else if position.y > MAX_Y { position.y = MIN_Y; }
  ```

## 4. Projectile Physics

Projectiles follow simple linear motion.

- **Client rendering note**: projectiles are spawned client-side when first observed
  in a server snapshot; snap to the first received position/rotation before enabling
  interpolation, otherwise they may appear to spawn at `(0, 0)` and slide to the
  shooter.

### Projectile State Variables

- `position` (Vec2)
- `velocity` (Vec2): Calculated at spawn time.
- `life_time` (float): Seconds remaining until destruction.

### Update Logic

1. **Movement**:

   ```rust
   position += velocity * delta_time;
   ```

2. **Lifetime**:

   ```rust
   life_time -= delta_time;
   if life_time <= 0.0 { destroy(); }
   ```

3. **World boundary behavior**: projectiles do **not** wrap. They continue along
   their velocity until despawn (TTL expiry) or collision hit.

## 5. Collision Detection

We will use simplified **Circle-Circle** collision detection.

### Shapes

- **Player**: Modeled as a Circle with radius `R_PLAYER` (approx 20-30px).
- **Projectile**: Modeled as a Point or small Circle `R_PROJ` (approx 5px).

### Logic

Check every projectile against every player (excluding the projectile's owner).

```rust
let distance_squared = (player.pos - proj.pos).length_squared();
let radius_sum = R_PLAYER + R_PROJ;

if distance_squared < (radius_sum * radius_sum) {
    // Hit detected
    handle_damage(player, proj);
}
```
