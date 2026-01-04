use crate::protocol::{GameEvent, WorldUpdate};
use crate::state::{EntityState, ServerState};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, watch};

pub async fn world_task(
    mut input_rx: mpsc::Receiver<GameEvent>,
    world_tx: broadcast::Sender<WorldUpdate>,
    server_state_tx: watch::Sender<ServerState>,
) {
    let mut tick: u64 = 0;
    let mut entities: Vec<EntityState> = Vec::new();

    let _ = server_state_tx.send(ServerState::MatchStarting { in_seconds: 3 });
    tokio::time::sleep(Duration::from_secs(3)).await;
    let _ = server_state_tx.send(ServerState::MatchRunning);

    // Set tick rate to 60 Hz (approx 16ms), but for debugging lets keep this at 1000ms
    let mut interval = tokio::time::interval(Duration::from_millis(1000));

    loop {
        // Wait for next tick
        interval.tick().await;

        // Process all pending inputs/events
        while let Ok(ev) = input_rx.try_recv() {
            match ev {
                GameEvent::Join { player_id } => {
                    println!("Logic: Player {} joined", player_id);
                    // Spawn player at random position
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_micros();
                    let x = ((now % 800) as f32) - 400.0;
                    let y = ((now % 460) as f32) - 230.0;
                    entities.push(EntityState {
                        id: player_id,
                        x,
                        y,
                        rot: 0.0,
                    });
                }
                GameEvent::Leave { player_id } => {
                    println!("Logic: Player {} left", player_id);
                    entities.retain(|e| e.id != player_id);
                }
                GameEvent::Input { player_id, input } => {
                    // Apply input to correct entity
                    if let Some(e) = entities.iter_mut().find(|e| e.id == player_id) {
                        e.x += input.dx;
                        e.y += input.dy;
                        e.rot += input.drot;
                        // println!("Applied input for {}", player_id);
                    }
                }
            }
        }

        tick += 1;
        // Broadcast new state
        let _ = world_tx.send(WorldUpdate {
            tick,
            entities: entities.clone(),
        });
    }
}
