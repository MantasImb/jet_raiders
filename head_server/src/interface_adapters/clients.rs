use crate::domain::{AuthGuestRequest, AuthGuestResponse, AuthProvider};
use async_trait::async_trait;
use reqwest::Client;

// The clients defined here are for reqwest clients to communicate with external services.
// Thin wrapper around reqwest for auth service calls.
#[derive(Clone)]
pub struct AuthClient {
    http: Client,
    pub base_url: String,
}

impl AuthClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl AuthProvider for AuthClient {
    async fn create_guest_session(
        &self,
        req: AuthGuestRequest,
    ) -> Result<AuthGuestResponse, Box<dyn std::error::Error>> {
        // Compose the auth URL and POST the guest payload.
        let url = format!("{}/auth/guest", self.base_url);
        let res = self.http.post(url).json(&req).send().await?;
        // TODO: Check `res.status()` and return a clear error on non-2xx responses.

        // Parse the auth response into our DTO.
        res.json::<AuthGuestResponse>().await.map_err(|e| e.into())
    }
}
