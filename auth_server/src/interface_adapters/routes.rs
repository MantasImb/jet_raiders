use crate::interface_adapters::handlers::{guest_login, logout, verify_token};
use crate::interface_adapters::state::AppState;
use axum::{routing::post, Router};

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/auth/guest", post(guest_login))
        .route("/auth/verify-token", post(verify_token))
        .route("/auth/logout", post(logout))
        .with_state(state)
}
