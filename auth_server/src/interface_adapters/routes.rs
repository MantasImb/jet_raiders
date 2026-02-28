use crate::interface_adapters::handlers::{guest_init, guest_login, logout, verify_token};
use crate::interface_adapters::state::AppState;
use axum::{routing::post, Router};

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/auth/guest/init", post(guest_init))
        .route("/auth/guest", post(guest_login))
        .route("/auth/verify-token", post(verify_token))
        .route("/auth/logout", post(logout))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Session;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use sqlx::postgres::PgPoolOptions;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    fn build_test_app() -> Router {
        build_test_app_with_sessions(HashMap::new())
    }

    fn build_test_app_with_sessions(seed_sessions: HashMap<String, Session>) -> Router {
        // Use a lazy pool because route contract tests should not require a
        // live database connection when the exercised path is DB-independent.
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/auth_test")
            .expect("expected lazy postgres pool");
        let state = AppState {
            sessions: Arc::new(Mutex::new(seed_sessions)),
            db,
        };

        app(state)
    }

    #[tokio::test]
    async fn when_guest_login_payload_has_zero_guest_id_then_returns_400_and_error_message() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/auth/guest")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"guest_id":0,"display_name":"Pilot","metadata":null}"#,
            ))
            .expect("expected request to build");

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("expected response body");
        let payload: Value = serde_json::from_slice(&body).expect("expected json body");
        assert_eq!(payload["message"], "guest_id is required");
    }

    #[tokio::test]
    async fn when_verify_token_is_invalid_then_returns_401_and_error_message() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/auth/verify-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"token":"missing-token"}"#))
            .expect("expected request to build");

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("expected response body");
        let payload: Value = serde_json::from_slice(&body).expect("expected json body");
        assert_eq!(payload["message"], "invalid session token");
    }

    #[tokio::test]
    async fn when_logout_token_is_unknown_then_returns_200_with_revoked_false() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/auth/logout")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"token":"unknown-token"}"#))
            .expect("expected request to build");

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("expected response body");
        let payload: Value = serde_json::from_slice(&body).expect("expected json body");
        assert_eq!(payload["revoked"], false);
    }

    #[tokio::test]
    async fn when_guest_route_is_called_with_get_then_returns_405() {
        let app = build_test_app();

        let request = Request::builder()
            .method("GET")
            .uri("/auth/guest")
            .body(Body::empty())
            .expect("expected request to build");

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn when_auth_route_does_not_exist_then_returns_404() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/auth/does-not-exist")
            .body(Body::empty())
            .expect("expected request to build");

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn when_guest_login_display_name_is_invalid_then_returns_400_and_error_message() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/auth/guest")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"guest_id":42,"display_name":"Pilot!","metadata":null}"#,
            ))
            .expect("expected request to build");

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("expected response body");
        let payload: Value = serde_json::from_slice(&body).expect("expected json body");
        assert_eq!(payload["message"], "invalid display_name");
    }

    #[tokio::test]
    async fn when_verify_token_session_is_expired_then_returns_401_and_error_message() {
        let mut sessions = HashMap::new();
        sessions.insert(
            "expired-token".to_string(),
            Session {
                guest_id: 42,
                display_name: "Pilot".to_string(),
                metadata: None,
                session_id: "session-1".to_string(),
                expires_at: 0,
            },
        );
        let app = build_test_app_with_sessions(sessions);

        let request = Request::builder()
            .method("POST")
            .uri("/auth/verify-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"token":"expired-token"}"#))
            .expect("expected request to build");

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("expected response body");
        let payload: Value = serde_json::from_slice(&body).expect("expected json body");
        assert_eq!(payload["message"], "session expired");
    }

    #[tokio::test]
    async fn when_logout_payload_is_missing_token_then_returns_422() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/auth/logout")
            .header("content-type", "application/json")
            .body(Body::from(r#"{}"#))
            .expect("expected request to build");

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn when_guest_payload_is_missing_required_fields_then_returns_422() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/auth/guest")
            .header("content-type", "application/json")
            .body(Body::from(r#"{}"#))
            .expect("expected request to build");

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn when_guest_init_display_name_is_invalid_then_returns_400_and_error_message() {
        let app = build_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/auth/guest/init")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"display_name":"xx","metadata":null}"#))
            .expect("expected request to build");

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("expected response body");
        let payload: Value = serde_json::from_slice(&body).expect("expected json body");
        assert_eq!(payload["message"], "invalid display_name");
    }
}
