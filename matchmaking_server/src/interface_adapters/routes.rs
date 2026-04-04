use crate::interface_adapters::handlers::health::health;
use crate::interface_adapters::handlers::queue::{cancel_ticket, enqueue, lookup_ticket};
use crate::interface_adapters::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

// Build the HTTP router for matchmaking endpoints.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/matchmaking/queue", post(enqueue))
        .route(
            "/matchmaking/queue/{ticket_id}",
            get(lookup_ticket).delete(cancel_ticket),
        )
        .with_state(state)
}
