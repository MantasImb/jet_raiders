# Guest Login Orchestrator

## Purpose

Centralize the guest login workflow so HTTP handlers remain thin and consistent.

## Responsibilities

- Validate display name rules (length, allowed characters, trimming).
- Create or update the guest profile in the head data store.
- Call the auth service to create or validate a guest session.
- Return a response payload with the session token and profile summary.

## Suggested Flow

1. Normalize and validate the display name.
2. Load or create the guest profile.
3. Upsert the profile with the validated name and metadata.
4. Call the auth service with the guest ID and display name.
5. Return the auth token and profile data to the client.

## Inputs

- `guest_id` (optional)
- `display_name`
- `metadata` (optional, future)

## Outputs

- `session_token`
- `guest_id`
- `display_name`

## Notes

- Keep transport-specific DTOs in interface adapters.
- Keep persistence and auth calls behind domain ports.
