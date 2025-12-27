use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{Router, response::IntoResponse, routing::get};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

// TODO: Why is there a need for shared state?
struct AppState {
    // You can add shared state here if needed
    tx: broadcast::Sender<String>,
}

/// TODO: Understand the app_state required in this case
/// TODO: Understand the broadcast and the channel

#[tokio::main]
async fn main() {
    // Create a broadcast channel for sending messages to all connected clients
    let (tx, _rx) = broadcast::channel(100);

    let app_state = Arc::new(AppState { tx: tx.clone() });

    // Spawn a background task that sends a message every second
    // TODO: Does this spawn a new thread and what are tasks
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            // Send message to all active subscibers
            let _ = tx.send("Server tick".to_string());
        }
    });

    // Build our application with a route
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    // Define the address to run our server on
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    // Run the server
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

/// The handler for the HTTP request (this gets called when the HTTP request lands at the start
/// of websocket negotiation). We upgrade the request to a WebSocket connection.
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}
/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    println!("Client connected");
    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                if let Ok(msg) = msg {
                    if let Message::Text(text) = msg {
                        println!("Received: {}", text);
                        // Echo back
                        if socket.send(Message::Text(format!("Echo: {}", text).into())).await.is_err() { break;}
                    }
                } else {
                    // Client disconnected
                    break;
                }
            }
            // Handle broadcast messages from the server
            Ok(msg) = rx.recv() => {
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        }
    }
}
