use crate::interface_adapters::handlers::queue::enqueue;
use crate::interface_adapters::state::AppState;
use axum::{Router, routing::post};
use std::sync::Arc;

// Build the HTTP router for matchmaking endpoints.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/matchmaking/queue", post(enqueue))
        .with_state(state)
}
