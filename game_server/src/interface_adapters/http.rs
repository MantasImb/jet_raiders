// Shared HTTP response types for consistent API error payloads.

#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    // Human-readable error string for consistent JSON error responses.
    pub error: String,
}
