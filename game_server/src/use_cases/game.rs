use super::types::{GameEvent, ServerState, WorldUpdate};
use crate::domain::systems::{projectiles, ship_movement};
use crate::domain::tuning::player::PlayerTuning;
use crate::domain::tuning::projectile::ProjectileTuning;
use crate::domain::{EntitySnapshot, PlayerInput, ProjectileSnapshot, SimEntity, SimProjectile};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, watch};
use tracing::info;

pub async fn world_task(
    mut input_rx: mpsc::Receiver<GameEvent>,
    world_tx: broadcast::Sender<WorldUpdate>,
    server_state_tx: watch::Sender<ServerState>,
    tick_interval: Duration,
    shutdown: Arc<tokio::sync::Notify>,
    match_time_limit: Duration,
) {
    let mut tick: u64 = 0;
    let mut entities: Vec<SimEntity> = Vec::new();
    let mut projectiles: Vec<SimProjectile> = Vec::new();
    let mut next_projectile_id: u64 = 1;

    let _ = server_state_tx.send(ServerState::MatchStarting { in_seconds: 3 });
    tokio::time::sleep(Duration::from_secs(3)).await;
    let _ = server_state_tx.send(ServerState::MatchRunning);

    // Drive the fixed-step game loop at the configured tick rate.
    let mut interval = tokio::time::interval(tick_interval);

    // World bounds for wrapping.
    let (min_x, max_x) = (-400.0, 400.0);
    let (min_y, max_y) = (-230.0, 230.0);

    // Projectile tuning (keep in sync with client `projectile.tscn` timer for now).
    let projectile_tuning = ProjectileTuning::default();
    let projectile_speed: f32 = projectile_tuning.speed;
    let projectile_ttl: f32 = projectile_tuning.life_time;
    let projectile_radius: f32 = projectile_tuning.radius;
    let projectile_damage: i32 = projectile_tuning.damage;
    let projectile_cooldown: f32 = 0.1;

    let player_tuning = PlayerTuning::default();
    let player_radius: f32 = player_tuning.radius;
    let player_max_hp: i32 = player_tuning.max_hp;
    let respawn_delay: f32 = player_tuning.respawn_seconds;

    // Track match duration for time-limit based end conditions.
    let mut match_elapsed = Duration::from_secs(0);
    let mut match_ended = false;

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                // Exit cleanly when the lobby is removed.
                break;
            }
            _ = interval.tick() => {
                if !match_ended && match_time_limit != Duration::from_secs(0) {
                    // Time limit is the current win condition; extend with other checks later.
                    match_elapsed += tick_interval;
                    if match_elapsed >= match_time_limit {
                        let _ = server_state_tx.send(ServerState::MatchEnded);
                        match_ended = true;
                    }
                }
            }
        }

        while let Ok(ev) = input_rx.try_recv() {
            match ev {
                GameEvent::Join { player_id } => {
                    info!(player_id, "player joined");
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_micros();
                    let x = ((now % 800) as f32) - 400.0;
                    let y = ((now % 460) as f32) - 230.0;
                    entities.push(SimEntity {
                        id: player_id,
                        x,
                        y,
                        rot: 0.0,
                        hp: player_max_hp,
                        alive: true,
                        respawn_timer: 0.0,
                        throttle: 0.0,
                        last_input: PlayerInput {
                            thrust: 0.0,
                            turn: 0.0,
                            shoot: false,
                        },
                        shoot_cooldown: 0.0,
                    });
                }
                GameEvent::Leave { player_id } => {
                    info!(player_id, "player left");
                    entities.retain(|e| e.id != player_id);
                    projectiles.retain(|p| p.owner_id != player_id);
                }
                GameEvent::Input { player_id, input } => {
                    if let Some(e) = entities.iter_mut().find(|e| e.id == player_id) {
                        e.last_input = input;
                    }
                }
            }
        }

        let dt = tick_interval.as_secs_f32();
        let cfg = ship_movement::MovementConfig {
            max_speed: player_tuning.max_speed,
            turn_rate: player_tuning.turn_rate,
            throttle_rate: player_tuning.throttle_rate,
            min_x,
            max_x,
            min_y,
            max_y,
        };

        for e in &mut entities {
            // Respawn logic.
            if !e.alive {
                e.respawn_timer -= dt;
                if e.respawn_timer <= 0.0 {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_micros();
                    e.x = ((now % 800) as f32) - 400.0;
                    e.y = ((now % 460) as f32) - 230.0;
                    e.rot = 0.0;
                    e.hp = player_max_hp;
                    e.alive = true;
                    e.respawn_timer = 0.0;
                    e.throttle = 0.0;
                    e.shoot_cooldown = 0.0;
                    e.last_input = PlayerInput {
                        thrust: 0.0,
                        turn: 0.0,
                        shoot: false,
                    };
                }
                continue;
            }

            // Ship movement.
            ship_movement::tick_entity(e, dt, cfg);
        }

        // Projectile simulation and collision resolution.
        projectiles::tick_projectiles(
            &mut entities,
            &mut projectiles,
            &mut next_projectile_id,
            dt,
            projectiles::ProjectileConfig {
                speed: projectile_speed,
                ttl: projectile_ttl,
                radius: projectile_radius,
                damage: projectile_damage,
                cooldown: projectile_cooldown,
                player_radius,
                respawn_delay,
            },
        );

        tick += 1;
        let entities_snapshot: Vec<EntitySnapshot> = entities
            .iter()
            .filter(|e| e.alive)
            .map(EntitySnapshot::from)
            .collect();
        let projectiles_snapshot: Vec<ProjectileSnapshot> =
            projectiles.iter().map(ProjectileSnapshot::from).collect();

        let _ = world_tx.send(WorldUpdate {
            tick,
            entities: entities_snapshot,
            projectiles: projectiles_snapshot,
        });
    }
}
