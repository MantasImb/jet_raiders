use axum::Json;

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: &'static str,
}

// Lightweight liveness endpoint for container smoke checks.
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_returns_ok_status() {
        let response = health().await;
        assert_eq!(response.0, HealthResponse { status: "ok" });
    }
}
