use serde::{Deserialize, Serialize};
use serde_json::Value;

// Guest session record stored in memory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub guest_id: String,
    pub display_name: String,
    pub metadata: Option<Value>,
    pub session_id: String,
    pub expires_at: u64,
}
