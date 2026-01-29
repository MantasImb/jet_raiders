// Domain-level errors for auth workflows.
#[derive(Debug)]
pub enum AuthError {
    InvalidGuestId,
    InvalidDisplayName,
    InvalidToken,
    SessionExpired,
    StorageFailure,
}
