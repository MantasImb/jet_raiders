use crate::interface_adapters::handlers::guest::{guest_init, guest_login};
use crate::interface_adapters::state::AppState;
use axum::{Router, routing::post};
use std::sync::Arc;

pub fn app(state: Arc<AppState>) -> Router {
    // Wire the HTTP routes to their handlers.
    Router::new()
        .route("/guest/init", post(guest_init))
        .route("/guest/login", post(guest_login))
        .with_state(state)
}
