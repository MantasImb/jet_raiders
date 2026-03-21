use crate::interface_adapters::handlers::guest::{guest_init, guest_login};
use crate::interface_adapters::handlers::matchmaking::{
    cancel_matchmaking, enter_matchmaking, poll_matchmaking,
};
use crate::interface_adapters::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

pub fn app(state: Arc<AppState>) -> Router {
    // Wire the HTTP routes to their handlers.
    Router::new()
        .route("/guest/init", post(guest_init))
        .route("/guest/login", post(guest_login))
        .route("/matchmaking/queue", post(enter_matchmaking))
        .route(
            "/matchmaking/queue/{ticket_id}",
            get(poll_matchmaking).delete(cancel_matchmaking),
        )
        .with_state(state)
}
