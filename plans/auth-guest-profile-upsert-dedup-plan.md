# Plan: Auth Guest Profile Upsert Deduplication

## Objective

Remove duplicated guest-profile persistence logic in auth HTTP handlers while
preserving current behavior and clean-architecture boundaries.

## Problem Summary

`guest_init` and `guest_login` in
`auth_server/src/interface_adapters/handlers.rs` both:

- derive `metadata_json` with the same fallback
- construct `PostgresGuestProfileStore`
- run best-effort `upsert_guest_profile`
- log the same warning on failure

This duplication increases maintenance cost and raises drift risk when behavior
changes.

## Scope

In scope:

- deduplicate best-effort profile upsert logic in adapter handlers
- keep public HTTP behavior and status codes unchanged
- keep domain/use-case layers unchanged
- keep persistence as best-effort (non-blocking)

Out of scope:

- schema changes
- auth flow semantics changes
- retries, backoff, or error policy redesign
- moving persistence into use cases/domain

## Constraints

- Follow `CLEAN_ARCHITECTURE_GUIDELINES.md`.
- Keep runtime/bootstrap/config concerns in frameworks/adapters.
- Do not introduce domain dependency on transport DTOs.

## Phases

### Phase 1: Characterization and Test Guardrails

Add or confirm adapter-level tests that lock in current observable behavior for:

- success paths of `POST /auth/guest/init` and `POST /auth/guest`
- upsert failure remains non-fatal to token issuance

Acceptance criteria:

- [ ] Existing route/use-case contracts remain green.
- [ ] Failure to upsert still does not fail auth responses.

### Phase 2: Extract Shared Upsert Path

Refactor duplicated handler code into one shared adapter helper with explicit
inputs:

- `guest_id` string form
- `display_name`
- `metadata_json`
- DB handle

Acceptance criteria:

- [ ] `guest_init` and `guest_login` call shared helper.
- [ ] Warning log behavior on upsert failure stays equivalent.
- [ ] No domain/use-case boundary violations introduced.

### Phase 3: Cleanup and Documentation

Finalize with small readability improvements and document intent inline.

Acceptance criteria:

- [ ] Handler code is shorter and easier to scan.
- [ ] Comments explain best-effort persistence rationale once (not duplicated).
- [ ] Full `cargo test -p auth_server` passes.

## Risks

- accidental behavior drift (turning best-effort into hard failure)
- subtle response changes through refactor side effects

## Verification

- targeted route tests for guest init/login behavior
- full `cargo test -p auth_server`
- manual smoke: `POST /auth/guest/init` and `POST /auth/guest` still succeed
  when profile upsert fails
