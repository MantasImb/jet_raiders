# Jet Raiders - Abilities & Extensibility Plan

This document outlines the architecture for adding player abilities (weapons,
defense, movement) to the game. The goal is to design the core data structures
now to support these features later without major refactoring.

## 1. Concept: Ability Slots

To keep the system flexible but structured, every player will have defined
"Slots" for abilities.

- **Primary Weapon**: The main attack (Standard Fire, Laser, Spread Shot).
  Triggered by `Space`.
- **Secondary Ability**: A special move (Dash, Invisibility, Shield).
  Triggered by `Shift` or `Q`.

## 2. Data Structure Design

We will avoid hardcoding specific abilities into the `Player` struct. Instead,
we will use `Enums` and a generic `AbilityState` struct.

### The `AbilityType` Enum

This defines _what_ ability is equipped.

```rust
#[derive(Clone, Copy, PartialEq)]
pub enum AbilityType {
    // Weapons
    StandardGun,
    LaserBeam,
    ScatterShot,

    // Utility
    None,
    Dash,
    Invisibility,
    EnergyShield,
}
```

### The `Cooldown` Struct

Every ability needs to track when it can be used again.

```rust
pub struct Cooldown {
    pub ready_at: f64,    // Server timestamp when ready
    pub duration: f64,    // Total cooldown time in seconds
}

impl Cooldown {
    pub fn is_ready(&self, current_time: f64) -> bool {
        current_time >= self.ready_at
    }

    pub fn trigger(&mut self, current_time: f64) {
        self.ready_at = current_time + self.duration;
    }
}
```

### Updating the `Player` Struct

The `Player` struct in `state.rs` will include these new fields.

```rust
pub struct Player {
    // ... existing fields ...

    // Primary Weapon
    pub weapon_type: AbilityType,
    pub weapon_cooldown: Cooldown,

    // Secondary Ability
    pub secondary_type: AbilityType,
    pub secondary_cooldown: Cooldown,
    pub secondary_active_until: f64, // For duration-based effects (e.g., Shield)

    // Status Effects (Flags)
    pub is_invisible: bool,
    pub is_shielded: bool,
}
```

## 3. Implementation Logic

Logic for abilities will be handled in separate systems to keep `combat.rs`
clean.

### New System: `abilities.rs`

This system runs _before_ movement and combat.

1. **Check Input**: Did the player press the Ability Key?
2. **Check Cooldown**: Is `secondary_cooldown.is_ready()`?
3. **Activate**:
    - **Dash**: Add a sudden burst to `velocity` vector.
    - **Invisibility**: Set `is_invisible = true` and `secondary_active_until`.
    - **Shield**: Set `is_shielded = true` and `secondary_active_until`.
4. **Manage Durations**:
    - If `current_time > secondary_active_until`, disable effects (set flags
      to false).

### Updates to `combat.rs`

- **Shooting**: Switch on `weapon_type` to decide what projectile to spawn
  (fast & weak, or slow & strong).
- **Damage**:
  - If `target.is_shielded`, ignore damage.
  - If `target.is_invisible`, projectiles might still hit (lucky shot), or
    pass through depending on design.

## 4. Network Protocol Updates

The client needs to know about these states to render them (e.g., draw a
shield bubble, fade out the sprite).

### `PlayerState` (Sent to Client)

```rust
pub struct PlayerData {
    // ... pos, rot ...
    pub is_shielded: bool,
    pub is_invisible: bool, // Client might still receive position but alpha=0
}
```

_Note: For true competitive invisibility, the server should NOT send the
position of invisible players to enemies. For this project, sending it with a
flag is acceptable for simplicity._

## 5. Stats & Progression

To support leveling up and diverse builds, we will separate **Base Stats** from
**Modifiers**. This allows for complex upgrades (e.g., "3x Damage but 0.5x
Fire Rate").

### The `PlayerStats` Struct

We collect all gameplay variables into a single struct.

```rust
pub struct PlayerStats {
    // Offensive
    pub damage_mult: f32,       // Default: 1.0
    pub fire_rate_mult: f32,    // Default: 1.0 (Higher is faster or wait time multiplier?)
                                // Recommendation: "Cooldown Multiplier" (0.5 = twice as fast)

    pub heat_dissipation_mult: f32, // Default: 1.0
    pub heat_capacity_mult: f32,    // Default: 1.0

    // Defensive / Movement
    pub max_health_mult: f32,   // Default: 1.0
    pub speed_mult: f32,        // Default: 1.0
}
```

### Applying Upgrades

When a player chooses an upgrade, we simply modify these values.

- **Upgrade: "Heavy Barrel"**

  ```rust
  stats.damage_mult *= 3.0;
  stats.fire_rate_mult *= 2.0; // Increases cooldown (slower fire)
  ```

- **Upgrade: "Cooling System"**

  ```rust
  stats.heat_dissipation_mult *= 1.5;
  ```

### Calculation in Systems

Systems (`combat.rs`, `movement.rs`) will use these modifiers to calculate
effective values on the fly.

```rust
let effective_damage = BASE_DAMAGE * player.stats.damage_mult;
let effective_cooldown = BASE_WEAPON_COOLDOWN * player.stats.fire_rate_mult;
```
