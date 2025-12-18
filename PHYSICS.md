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

- `MAX_SPEED`: Maximum pixels per second (e.g., 150.0).
- `TURN_RATE`: Rotation speed in radians per second (e.g., 2.5).
- `THROTTLE_RATE`: How fast throttle increases/decreases per second (e.g., 1.0).

### Update Logic (Per Tick)

1. **Rotation**:

   ```rust
   rotation += input.turn_axis * TURN_RATE * delta_time;
   ```

2. **Throttle**:

   ```rust
   throttle += input.throttle_axis * THROTTLE_RATE * delta_time;
   throttle = clamp(throttle, 0.0, 1.0);
   ```

3. **Velocity Calculation**:
   - The velocity vector is derived purely from rotation and throttle.
   - `direction = Vec2::new(-sin(rotation), -cos(rotation))` (Assuming 0 is Up/North).
   - `velocity = direction * throttle * MAX_SPEED`
4. **Position Update**:

   ```rust
   position += velocity * delta_time;
   ```

## 3. World Boundaries (Wrapping)

The game world is a torus (loops around).

- **Logic**: If a player moves past a border, they teleport to the opposite side.
- **Implementation**:

  ```rust
  if position.x < MIN_X { position.x = MAX_X; }
  else if position.x > MAX_X { position.x = MIN_X; }

  if position.y < MIN_Y { position.y = MAX_Y; }
  else if position.y > MAX_Y { position.y = MIN_Y; }
  ```

## 4. Projectile Physics

Projectiles follow simple linear motion.

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

3. **Wrapping**: Projectiles should also wrap around the screen to match player
   expectation (or be destroyed at border, depending on design choice. Currently:
   **Wraps** based on gameplay feel, though simple destruction is easier to start).
   _Correction based on `Projectile.gd`: The Godot script does NOT wrap projectiles.
   They fly until `QueueFree` timer expires. We will replicate this._

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
