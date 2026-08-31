# WP-W2 receipt-authoritative completion review

## Verdict

**ACCEPTED after exact-successor repair.** The null-receipt completion branch
and nonnull successor branch now both respect the immutable submission receipt.
The production fixture proves that a stale/same attempt and every mismatched
public successor descriptor preserve the submitted Position 1 response and
show the recoverable next-question action.

## Accepted findings

- The native retry lifecycle contract is sound: the server records an immutable
  predecessor-to-successor link and its native regression asserts the returned
  `nextIssued.id` is the independently listed retry attempt.
- `receiptNext === null` now calls `machine.complete`, shows the summary, and
  loads the summary without a run-screen read. The production completion
  fixture verifies zero fresh-run reads for pending, allowed, and closed summary
  policy.
- A nonnull receipt reaches `runtime.client.getRunScreen` inside
  `machine.advance`, so fetch failures enter the existing `advanceFailed`
  recovery path and `retryAdvance` can retry without resubmitting.
- A same-attempt fresh screen throws `ApiProtocolError` instead of being
  interpreted as terminal completion. This avoids correctness inference from a
  stale active screen.
- The implementation does not inspect correctness, response content, score,
  feedback body, answer material, report state, credential, or other private
  server data.

## Exact-successor repair review

- `matchesIssuedSuccessor` compares the exact public receipt binding: attempt
  ID, run, assignment position, question revision, seed, deadline, and rendered
  question hash. A fresh screen must pass that predicate before it can become a
  machine successor.
- The client reads the fresh screen inside `machine.advance`; a mismatch throws
  `ApiProtocolError`, producing the existing `advanceFailed` recovery rather
  than a completion or replacement question.
- `setScreen` is now outside the loader and follows the successful-advance
  guard, so failed receipt validation leaves the prior Position 1 screen and
  its checked response intact.
- The production component fixture covers `same`, wrong ID, run, position,
  version, seed, deadline, and hash. Every case visibly exposes `Retry next
question`, retains Position 1, and retains the selected radio. This closes
  both prior behavioral-test findings without inspecting answers or private
  state.

## Validation

- PASS: focused Prettier, TypeScript lint compile, ESLint, and `git diff --check`.
- PASS: `node --import tsx --test tests/test_question_attempt_state.mjs tests/test_frontend_contract.mjs tests/test_run_page_recovery.mjs` (41 passed).
- PASS: `npx playwright test tests/playwright/run_prefetch_route.spec.ts tests/playwright/run_completion_summary.spec.ts tests/playwright/frontend_contract.spec.ts` (32 passed).

The exact-successor and same-attempt recovery cases are now covered by the
production-component fixture and the receipt-authority contract is closed.
