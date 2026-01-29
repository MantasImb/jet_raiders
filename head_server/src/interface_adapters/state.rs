use crate::domain::AuthProvider;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    // We use Arc<dyn Trait> to hold any implementation (dependency injection).
    pub auth: Arc<dyn AuthProvider>,
}
