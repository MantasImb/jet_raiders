# Testing

## Purpose

This repository uses test-driven development to keep changes small, behavior-
focused, and well-bounded.

Most services here are internal-facing. They are called by other services and
clients in this repository and are maintained by the same team. That means the
test strategy should protect real behavior contracts, security boundaries, and
architecture seams without creating unnecessary test volume.

Tests are the main boundary contract for implementation work in this repository.
A good test suite should:

- Define observable behavior clearly.
- Constrain implementation scope.
- Catch regressions quickly.
- Preserve clean architecture boundaries.
- Stay readable and fast enough to support frequent iteration.

## Core rules

### 1. Test behavior through public interfaces

Tests should verify what callers can observe:

- Inputs and outputs.
- State transitions.
- Error semantics that are part of the contract.
- HTTP, WebSocket, or protocol responses.

Do not test private methods, internal call order, or internal collaborators when
the same behavior can be verified through a public interface.

### 2. Choose the narrowest boundary that proves the behavior

Pick the smallest public test surface that validates the requested change:

- Domain tests for business invariants.
- Use-case tests for orchestration and workflow rules.
- Adapter tests for DTO mapping, validation, and error translation.
- Framework tests for startup, routing, and wiring smoke checks only.

Do not drop lower into internals unless the behavior truly lives there and
cannot be proven at a stable public boundary.

### 3. Use vertical-slice TDD

Do not write all tests first and then all implementation.

Use an explicit red-green-refactor loop in small slices:

1. Add one failing test for one behavior.
2. Confirm it fails for the expected reason.
3. Implement the smallest change needed to make it pass.
4. Refactor only after returning to green.
5. Repeat for the next behavior.

This repository does not treat "RED" as "write the whole test file up front."
Write one test, make it pass, then continue.

### 4. Add negative tests to define what must not happen

For each feature or bug fix, include negative coverage where it helps define the
boundary of allowed behavior.

Examples:

- Invalid payload is rejected.
- Unauthorized or expired credentials are rejected.
- Disallowed state transitions are blocked.
- Unsupported methods return the expected routing behavior.

There is no fixed quota. Add enough negative coverage to protect the contract
and security boundary without duplicating the same guarantee many times.

### 5. Keep tests robust, specific, and actionable

A failing test should make it clear:

- What behavior failed.
- Under what conditions.
- At which layer or boundary.

Prefer descriptive names and precise assertions. A good test should read like a
small specification.

## Repository testing priorities

Keep strong coverage for:

- Domain and use-case invariants such as identity, session, and game-state
  rules.
- Security behavior in auth-sensitive flows such as invalid token rejection,
  expiry handling, and revocation behavior.
- Core adapter contracts such as status-code, payload, and protocol mapping for
  primary success and failure paths.
- Route and handler wiring smoke checks such as `404`, `405`, and expected
  method behavior.
- Regression coverage for production bugs.

`auth_server` is an explicit exception to the default lightweight stance. Auth
flows are security-sensitive and shared by multiple internal services, so
broader-than-default coverage is appropriate there.

## What not to over-test

Prefer not to add many tests for:

- Exact error message variants unless another service or client depends on the
  exact text.
- Repeated malformed-payload permutations when one representative extraction or
  validation failure proves the contract.
- Low-value micro-edge cases that do not affect business correctness, auth
  security, or service-to-service compatibility.
- Framework behavior in domain or use-case suites.
- Internal implementation details that would break under harmless refactors.

The goal is not maximum test count. The goal is strong protection for important
behavior with minimal noise.

## Mocking and test doubles

Mock only real external boundaries when necessary, such as:

- External APIs.
- Time and randomness.
- Filesystem interactions.
- Infrastructure boundaries that are impractical to exercise directly.

Do not mock:

- Your own modules or classes.
- Internal collaborators you control.
- Private implementation seams introduced only to make tests easier.

At boundaries under our control, prefer simple fakes or lightweight ports over
behavior-heavy mocks.

## Practical workflow for changes

When implementing a new feature:

1. Confirm the target behavior and the boundary where it belongs.
2. Pick the narrowest public test surface that proves that behavior.
3. Add one failing test for the first required behavior.
4. Implement minimally until that test passes.
5. Repeat with the next behavior, including negative-path coverage where needed.
6. Refactor only while green.
7. Run broader suites after targeted tests pass.

Recommended test run order:

1. Affected tests only.
2. Layer-local suite.
3. Service-level integration tests.
4. Repository-level checks.

For bug fixes:

1. Add a test that reproduces the bug.
2. Confirm it fails on the old behavior.
3. Implement the fix.
4. Keep the regression test in the suite.

## Test design guidelines

### Naming

Use names that describe behavior and context, not implementation.

Pattern:

- `when_<context>_and_<condition>_then_<outcome>`

Example:

- `when_lobby_id_missing_and_create_requested_then_returns_400`

### Structure

Prefer a consistent Arrange-Act-Assert flow:

- Arrange only what is required.
- Act on one behavior.
- Assert the contract precisely.

### Determinism

Use fixed clocks, stable IDs, and explicit seeds where applicable. Avoid
non-determinism that creates flaky or weak tests.

### Assertion style

Assert on contract-level outcomes:

- Returned values.
- Exposed state.
- Emitted domain events.
- HTTP or protocol responses.

Avoid asserting on internal call counts, call order, or internal storage
inspection unless that boundary is itself the public contract under test.

### Error contracts

Pin error semantics that are part of the contract, such as:

- Status codes.
- Stable error keys or variants.
- Error categories required by callers.

Do not default to pinning exact message strings unless another consumer
explicitly depends on them.

## Definition of done

A change is complete only when all are true:

- New behavior is covered by failing-first tests.
- Tests were added incrementally in vertical slices, not in a bulk prewritten
  batch.
- The implementation passes targeted tests before broader suites are run.
- Existing tests still pass without weakening assertions.
- No architecture boundary violations were introduced.
- Negative-path coverage exists where needed to constrain the behavior.
- Bug fixes include regression tests.
- Test names and failures are readable and actionable.

## Anti-patterns

Avoid:

- Implement first, write tests later.
- Writing all tests first and all code second.
- Over-mocking that hides real behavior contracts.
- Brittle assertions on incidental formatting or exact message text that is not
  contractual.
- Large mixed changes that combine feature work and unrelated refactors.
- Tests that verify framework behavior in lower-level suites.
- Refactoring while tests are red.

## Prompt template for Codex implementation

Use this template when requesting implementation work:

```text
Implement <feature> using TDD.

Requirements:
- Confirm the public behavior and test boundary first.
- Add one failing test at a time and show it fails for the expected reason.
- Keep logic in <target layer> only.
- Test behavior through public interfaces, not internals.
- Add negative-path coverage for <invalid/forbidden cases>.
- Implement the minimum needed to pass the current test.
- Pass targeted tests, then run broader suites.
- Summarize how the tests protect the behavior contract.
```
