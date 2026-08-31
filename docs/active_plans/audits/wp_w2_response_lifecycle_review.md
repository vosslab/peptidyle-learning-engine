# WP-W2 response lifecycle review

## Verdict

**ACCEPTED.** `QuestionResponseControl` is now keyed by the durable server-issued
attempt ID. A successor attempt therefore receives a fresh local response
controller, while recovery and edits of the same attempt keep their mounted
controller and local selection.

## Findings

- The keyed `Show` derives its identity only from
  `currentState()?.context.attemptId`. `machine.advance` changes that context
  only after it has accepted a matching server-issued envelope, so a retry with
  the same response kind cannot inherit a prior choice.
- Offline/network/session recovery, renderer recovery, and ordinary same-attempt
  editing preserve the context ID. They do not recreate the keyed subtree; the
  question response control's local selection and controller therefore remain available.
  The attempt-state behavior suite independently pins the buffered-response and
  original-idempotency-key recovery contracts.
- `currentEnvelope()` remains reactive and supplies the new attempt definition
  after `machine.advance`. `initialResponse` reads the current machine state,
  while the event callback for an external tool closes over the same keyed
  attempt ID. There is no mixed old-attempt request path.
- The new production `RunPage` fixture observes a checked first radio, advances
  via a distinct issued attempt, then requires the first retry radio to be
  unchecked. It is DOM behavior rather than an implementation mock. Existing
  frontend recovery coverage confirms saved-entry editing and cleanup.
- The new static source assertion is deliberately narrow and aligns with the
  repo's existing contract-test style. The behavioral fixture, not that regex,
  is the material regression proof.
- The change exposes no response, score, grading, feedback body, credential,
  report, storage, or server-private field. It changes only component lifetime
  at a public attempt-identity boundary.

## Validation

- PASS: focused Prettier, TypeScript lint compile, and ESLint for `RunPage` and
  the focused Playwright fixtures.
- PASS: `node --import tsx --test tests/test_frontend_contract.mjs` (19 passed).
- PASS: `node --import tsx --test tests/test_question_attempt_state.mjs tests/test_frontend_contract.mjs tests/test_run_page_recovery.mjs` (40 passed).
- PASS: `npx playwright test tests/playwright/run_prefetch_route.spec.ts tests/playwright/run_completion_summary.spec.ts tests/playwright/frontend_contract.spec.ts` (24 passed).
- PASS: `git diff --check`.

The retry fixture's title says the active selection persists, though its direct
DOM assertion primarily proves the important new-attempt reset. Same-attempt
preservation is established by the stable keyed identity and the focused
attempt-state/frontend-recovery tests; a future fixture may make that one
same-attempt transition explicit, but no correctness or privacy defect was
found.
