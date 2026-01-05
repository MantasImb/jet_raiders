use crate::protocol::{GameEvent, PlayerInput, ServerMessage, WorldUpdate};
use crate::state::{AppState, ServerState};
use crate::utils::rng::rand_id;

use axum::{
    Error,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::SinkExt;
use std::sync::Arc;
use tokio::sync::watch::Receiver;
use tokio::sync::{broadcast, mpsc, watch};

#[allow(dead_code)]
#[derive(Debug)]
enum NetError {
    // TODO: Either remove this enum (if we keep handling errors ad-hoc) or use it consistently
    // TODO: throughout the connection lifecycle so callers can react based on category.
    // Future plans to make separate Ws errors for bootstrap and loop errors
    // This is currently not used by the handler loop
    Ws(axum::Error),
    Serialization(serde_json::Error),
    InputClosed,
    WorldUpdatesClosed,
    ServerStateClosed,
    // For future lag handling: when this happens, send latest GameState snapshot for resync
    // WorldUpdatesLagged(u64)
}

impl From<axum::Error> for NetError {
    fn from(e: axum::Error) -> Self {
        NetError::Ws(e)
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut ctx = match bootstrap_connection(&mut socket, &state).await {
        Ok(ctx) => ctx,
        Err(e) => {
            // TODO: Use tracing with structured fields (e.g. conn_id, player_id if assigned).
            // TODO: Consider sending a close frame / error message to the client before returning.
            eprintln!("Failed to bootstrap connection");
            eprintln!("{:?}", e);
            return;
        }
    };

    // Main Client Loop
    let _err = run_client_loop(&mut socket, &mut ctx).await;
}

async fn send_message(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), NetError> {
    // Serialize message safely; log JSON errors instead of panicking
    // TODO: Consider reducing per-message allocations (e.g. reuse buffers) if this becomes hot.
    let txt = serde_json::to_string(msg).map_err(NetError::Serialization)?;
    socket
        .send(Message::Text(txt.into()))
        .await
        .map_err(NetError::Ws)
}

struct ConnCtx {
    pub player_id: u64,
    pub input_tx: mpsc::Sender<GameEvent>,
    pub world_rx: broadcast::Receiver<WorldUpdate>,
    pub server_state_rx: watch::Receiver<ServerState>,
}

async fn bootstrap_connection(
    socket: &mut WebSocket,
    state: &AppState,
) -> Result<ConnCtx, NetError> {
    // Subscribe to updates *before* doing anything else (awaits) to not miss packets.
    let world_rx = state.world_tx.subscribe();
    let server_state_rx = state.server_state_tx.subscribe();

    // Handshake & ID Assignment
    // Generate a unique ID for this connection.
    // TODO: Ensure IDs are collision-free (or have the world task reject/rehash on collision).
    // TODO: If/when auth exists, bind player identity to auth/session instead of random IDs.
    let player_id = rand_id();

    // Send Identity Packet
    // Tell the client "This is who you are".
    let identity_msg = ServerMessage::Identity { player_id };
    send_message(socket, &identity_msg).await?;

    // Notify World Task
    // Tell the game loop to spawn a ship for this ID.
    // Join happens before initial state so the snapshot can include the newly spawned player.
    // If anything after Join fails, compensate with Leave to avoid "spawned but never connected".
    state
        .input_tx
        .send(GameEvent::Join { player_id })
        .await
        .map_err(|_| NetError::InputClosed)?;

    // Send Initial State
    // Keep in mind that we clone as soon as we borrow to avoid holding the lock. (especially
    // during an await)
    let initial_state = server_state_rx.borrow().clone();
    let state_msg = ServerMessage::GameState(initial_state);
    if let Err(e) = send_message(socket, &state_msg).await {
        state
            .input_tx
            .send(GameEvent::Leave { player_id })
            .await
            .map_err(|_| NetError::InputClosed)?; // InputClosed takes precedence
        return Err(e);
    }

    Ok(ConnCtx {
        player_id,
        world_rx,
        server_state_rx,
        input_tx: state.input_tx.clone(),
    })
}

enum LoopControl {
    Continue,
    Disconnect,
}

async fn run_client_loop(socket: &mut WebSocket, ctx: &mut ConnCtx) -> Result<(), NetError> {
    let player_id = ctx.player_id;
    let input_tx = &ctx.input_tx;
    let world_rx = &mut ctx.world_rx;
    let server_state_rx = &mut ctx.server_state_rx;

    let mut fatal: Option<NetError> = None;

    loop {
        // TODO: Add tracing spans per-connection and per-message counters (and sample logs).
        // disconnect becomes true on error
        let disconnect: bool = tokio::select! {
        // Incoming Message from Client
        incoming = socket.recv() => {
            match handle_incoming_ws(incoming, player_id, input_tx) {
                Ok(LoopControl::Continue) => false,
                Ok(LoopControl::Disconnect) => true,
                Err(e) => {
                    fatal = Some(e);
                    true
                }
            }
        }

        // Outgoing World Update
            world_msg = world_rx.recv() => {
                match world_msg {
                    Ok(update) => {
                        match forward_world_update(update, socket).await {
                            LoopControl::Continue => false,
                            LoopControl::Disconnect => true
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // TODO: Implement resync (e.g. send latest snapshot) instead of silently continuing.
                        // TODO: Rate-limit this log (can spam under load); switch to tracing metrics.
                        eprintln!("World_rx lagged. Missed {n} updates");
                        false
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        fatal = Some(NetError::WorldUpdatesClosed);
                        true
                    },
                }
        }

        // Outgoing Server State
        changed_state = server_state_rx.changed() => {
            match changed_state {
                Ok(()) => {
                    match forward_server_state(server_state_rx, socket).await {
                            LoopControl::Continue => false,
                            LoopControl::Disconnect => true
                        }
                    }
                    Err(e) => {
                        // TODO: `changed()` only errors when the sender is dropped; don't treat it as a generic error.
                        // TODO: Use tracing and include context about why we're shutting down the connection.
                        eprintln!("Server state closed: {:?}", e);
                        fatal = Some(NetError::ServerStateClosed);
                        true
                    }, //watch sender dropped
                    }
                }
            };
        if disconnect {
            if let Err(err) = socket.close().await.map_err(NetError::Ws) {
                eprintln!("{:?}", err);
            }
            break;
        }
    }
    if let Err(e) = disconnect_cleanup(ctx).await {
        eprintln!("Error cleaning up:{:?}", e);
        if fatal.is_none() {
            fatal = Some(e);
        }
    }

    if let Some(err) = fatal {
        Err(err)
    } else {
        Ok(())
    }
}

fn handle_incoming_ws(
    incoming: Option<Result<Message, Error>>,
    player_id: u64,
    input_tx: &mpsc::Sender<GameEvent>,
) -> Result<LoopControl, NetError> {
    match incoming {
        Some(Ok(msg)) => {
            match msg {
                Message::Text(text) => {
                    // Parse JSON Input with error reporting
                    // TODO: Validate/sanitize inputs (ranges, NaNs) before forwarding into the world task.
                    // TODO: Consider disconnecting or rate-limiting on repeated invalid JSON (anti-spam).
                    match serde_json::from_str::<PlayerInput>(&text) {
                        Ok(input) => {
                            // Forward input to World Task via Channel
                            match input_tx.try_send(GameEvent::Input { player_id, input }) {
                                Ok(()) => {}
                                Err(tokio::sync::mpsc::error::TrySendError::Full(evt)) => {
                                    println!("Could not send input_tx, mpsc full:");
                                    println!("{:?}", evt);
                                    // TODO: Needs upgraded implementation with sampling. (When tracing
                                    // is added)
                                    // TODO: The input is dropped on Full. Decide policy:
                                    // TODO: - drop newest vs drop oldest vs coalesce latest-per-player.
                                    // TODO: - optionally disconnect clients that consistently overload.
                                }
                                Err(tokio::sync::mpsc::error::TrySendError::Closed(_evt)) => {
                                    return Err(NetError::InputClosed);
                                }
                            };
                            Ok(LoopControl::Continue)
                        }
                        Err(e) => {
                            // TODO: Consider sending an error response (or closing) on malformed input.
                            // TODO: Avoid logging full payloads at high volume; use truncation/sampling.
                            eprintln!(
                                "Failed to parse input from player {}: {} (err: {})",
                                player_id, text, e
                            );
                            Ok(LoopControl::Continue)
                        }
                    }
                }
                Message::Close(_) => Ok(LoopControl::Disconnect),
                // Could be Ping/Pong or something else depending on the client, so continue
                other => {
                    // TODO: Explicitly handle Ping/Pong to keep connections healthy behind proxies.
                    // TODO: Decide if Binary is supported; otherwise close with a clear reason.
                    println!("Other non-text message from {}:", player_id);
                    println!("{:?}", other);
                    Ok(LoopControl::Continue)
                }
            }
        }
        Some(Err(e)) => {
            eprintln!("websocket recv error for {}: {}", player_id, e);
            Ok(LoopControl::Disconnect)
        }
        None => {
            eprintln!("websocket closed for {}", player_id);
            Ok(LoopControl::Disconnect)
        }
    }
}

async fn forward_world_update(world_msg: WorldUpdate, socket: &mut WebSocket) -> LoopControl {
    let msg = ServerMessage::WorldUpdate(world_msg);
    if send_message(socket, &msg).await.is_err() {
        LoopControl::Disconnect
    } else {
        LoopControl::Continue
    }
}

async fn forward_server_state(
    server_state_rx: &Receiver<ServerState>,
    socket: &mut WebSocket,
) -> LoopControl {
    let st = server_state_rx.borrow().clone();
    let msg = ServerMessage::GameState(st);
    if send_message(socket, &msg).await.is_err() {
        LoopControl::Disconnect
    } else {
        LoopControl::Continue
    }
}

async fn disconnect_cleanup(ctx: &mut ConnCtx) -> Result<(), NetError> {
    let player_id = ctx.player_id;
    ctx.input_tx
        .send(GameEvent::Leave { player_id })
        .await
        .map_err(|_| NetError::InputClosed)?;
    // TODO: Switch to tracing and include connection context (conn_id, session, lobby_id).
    println!("client {player_id} disconnected");
    Ok(())
}
