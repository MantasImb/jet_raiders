mod support;

// TODO: test the full response of the lobby creation request

#[tokio::test]
async fn test_lobby_creation() {
    let base_url = support::ensure_server();
    let client = reqwest::Client::new();
    let lobby_id = format!("test-{}", uuid::Uuid::new_v4());
    let payload = serde_json::json!({
        "lobby_id": lobby_id,
        "allowed_player_ids": []
    });

    let res = client
        .post(format!("{base_url}/lobbies"))
        .json(&payload)
        .send()
        .await
        .expect("request should succeed");

    assert_eq!(res.status(), reqwest::StatusCode::CREATED)
}
