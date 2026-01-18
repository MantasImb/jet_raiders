-- Guest profile storage for the simple auth flow.
CREATE TABLE IF NOT EXISTS guest_profiles (
    guest_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}'
);
