# Test-Driven Development Strategies for Codex Boundary Control

This document defines practical test-driven development (TDD) strategies for
using ChatGPT Codex to implement functionality safely, within clear boundaries,
and with strong regression protection.

## Purpose

When Codex is asked to implement a feature, tests should be the primary
boundary contract. A high-quality test suite:

- Constrains implementation scope.
- Makes acceptance criteria explicit.
- Detects regressions quickly.
- Reduces architecture drift and accidental coupling.

In this repository, those outcomes should also preserve clean architecture
boundaries.

## Core principles

### 1. Tests define behavior, not internal implementation

Write tests in terms of observable behavior:

- Inputs and outputs.
- State transitions.
- Error semantics.
- Protocol contracts.

Avoid asserting private internals unless no other option exists.

### 2. Start at the boundary where change is requested

Choose the narrowest boundary that validates the requested behavior:

- Domain tests for business invariants.
- Use-case tests for orchestration and workflow rules.
- Adapter tests for DTO mapping and transport error translation.
- Framework tests for wiring smoke checks only.

This aligns implementation pressure with clean architecture responsibilities.

### 3. Red -> Green -> Refactor must be explicit

For Codex-assisted work, keep the cycle visible:

1. Add or update failing tests that capture requested behavior.
2. Implement the smallest change needed to pass tests.
3. Refactor while keeping tests green.

Do not skip directly to implementation before a failing test exists.

### 4. Prevent scope creep with negative tests

For each feature, include at least one test proving what must *not* happen.
Examples:

- Invalid payload returns a precise validation error.
- Unauthorized actor is rejected.
- Disallowed state transition is blocked.

Negative tests are strong boundaries for Codex and prevent over-permissive
implementations.

### 5. Make regressions actionable

A failing test should immediately answer:

- What behavior failed?
- Under what conditions?
- At which boundary (domain/use case/adapter/framework)?

Use descriptive test names and specific assertions.

## Codex-oriented workflow

Use this workflow whenever requesting feature implementation from Codex.

### Step 1: Define behavior contract first

Write a short contract in the task prompt or issue:

- Supported scenarios.
- Rejected scenarios.
- Expected outputs and errors.
- Boundary/layer where logic belongs.

### Step 2: Author tests before implementation

Create or update tests that reflect the contract.

Guidance by layer:

- Domain: pure invariants, no framework dependencies.
- Use case: orchestration using interfaces and test doubles.
- Adapter: DTO conversion, validation, and error mapping.
- Framework: startup and dependency wiring only.

### Step 3: Run targeted tests and confirm failure

Before coding, run only affected tests to verify they fail for the right reason.
This prevents false confidence from stale or irrelevant tests.

### Step 4: Implement minimally

Ask Codex to satisfy only failing tests. Avoid unrelated refactors in the same
change unless needed to pass tests safely.

### Step 5: Expand confidence

After targeted tests pass, run broader suites in this order:

1. Layer-local tests.
2. Service-level integration tests.
3. Repository-level checks.

### Step 6: Capture regression tests for bug fixes

For every defect fix:

1. Add a test that reproduces the bug.
2. Confirm the new test fails on old behavior.
3. Implement fix.
4. Verify the regression test stays green.

## Test design patterns that constrain Codex

### Contract-first test naming

Use names that encode requirement + context + expected outcome.

Pattern:

- `when_<context>_and_<condition>_then_<outcome>`

Example:

- `when_lobby_id_missing_and_create_requested_then_returns_400`

### Arrange-Act-Assert consistency

Keep test structure uniform:

- Arrange: setup only what is needed.
- Act: execute one behavior.
- Assert: verify outcomes with precise checks.

This reduces ambiguity in Codex-generated changes.

### Deterministic data and clocks

Use stable IDs, fixed clocks, and explicit seeds where applicable. Non-
determinism invites flaky tests and weakens boundary enforcement.

### Single responsibility per test

Each test should validate one business rule or contract expectation. Fewer
compound assertions make failures easier to diagnose and repair.

### Error contract pinning

Pin error semantics explicitly:

- Status codes.
- Error keys/messages.
- Domain error variants.

This prevents silent breaking changes in client-facing behavior.

## Definition of done for Codex-driven changes

A feature is complete only when all are true:

- New behavior is covered by failing-first tests.
- Existing tests pass without weakening assertions.
- No architecture boundary violations are introduced.
- At least one negative-path test exists for the change.
- Bug fixes include regression tests.
- Test names and failures are readable and actionable.

## Prompt template for requesting Codex implementation

Use this template to force a test-first boundary:

```text
Implement <feature> using TDD.

Requirements:
- Add/adjust tests first and show they fail for expected reasons.
- Keep logic in <target layer> only.
- Do not change unrelated files.
- Add negative-path tests for <invalid/forbidden cases>.
- Pass targeted tests, then run broader suite.
- Summarize how tests prevent regressions.
```

## Anti-patterns to avoid

- "Implement first, write tests later."
- Over-mocking that hides real behavior contracts.
- Brittle assertions on incidental formatting.
- Large mixed commits that combine feature work and broad refactors.
- Tests that verify framework behavior in domain-level suites.

## Recommended adoption path

1. Start with one service and enforce failing-first PRs.
2. Add boundary-focused test templates per layer.
3. Require regression tests for every production bug.
4. Track flaky tests weekly and eliminate non-determinism.
5. Periodically review whether tests still match real contracts.

