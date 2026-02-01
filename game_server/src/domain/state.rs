// Domain-level simulation entities and input/snapshot types.

#[derive(Debug, Clone)]
pub struct EntitySnapshot {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
    pub hp: i32,
}

#[derive(Debug, Clone)]
pub struct ProjectileSnapshot {
    pub id: u64,
    pub owner_id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
}

#[derive(Debug, Clone)]
pub struct PlayerInput {
    pub thrust: f32,
    pub turn: f32,
    pub shoot: bool,
}

pub struct SimEntity {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,

    // Combat state.
    pub hp: i32,
    pub alive: bool,
    pub respawn_timer: f32,

    // Movement-only state (do not serialize to clients)
    pub throttle: f32,           // 0.0..=1.0
    pub last_input: PlayerInput, // last received input for this entity
    pub shoot_cooldown: f32,     // seconds until next allowed shot
}

pub struct SimProjectile {
    pub id: u64,
    pub owner_id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
    pub vx: f32,
    pub vy: f32,
    pub ttl: f32,
}

impl From<&SimEntity> for EntitySnapshot {
    fn from(e: &SimEntity) -> Self {
        Self {
            id: e.id,
            x: e.x,
            y: e.y,
            rot: e.rot,
            hp: e.hp,
        }
    }
}

impl From<&SimProjectile> for ProjectileSnapshot {
    fn from(p: &SimProjectile) -> Self {
        Self {
            id: p.id,
            owner_id: p.owner_id,
            x: p.x,
            y: p.y,
            rot: p.rot,
        }
    }
}
