# WP-W2 frontend refresh review

## Verdict

**ACCEPTED.** The W2 fallback now reads the server's current run screen through
the uncached API client after a submission. It does not alter initial routing,
the bound prefetch fast path, or the same-attempt completion behavior.

## Findings

- `RunPage` still obtains its initial route screen through
  `runtime.queries.runScreen(runId)`. Only the post-Continue fallback uses
  `runtime.client.getRunScreen(screen().run.id)`, so router loading and its
  query cache remain the initial-navigation boundary.
- A fully bound prefetched successor still calls `machine.advance` directly and
  makes no next-screen request. The existing production-component fixture
  verifies that fast path while also retaining its twelve-asset cap.
- An unbound, mismatched, or unavailable prefetch falls back to the fresh
  client screen. The new fixture deliberately makes the router query throw
  after mount and proves Continue advances to Position 2 without invoking it.
  This is a direct test of the stale-query regression rather than a mock of the
  implementation function.
- The same-attempt response still completes the state machine and loads the
  summary. The production `RunPage` fixture covers pending, allowed, and closed
  summary policy, asserts the stale router query is unused, and confirms the
  fresh-practice action appears only after an allowed summary.
- The neutral `Run complete` heading before summary policy arrives avoids a
  premature fresh-practice promise. The later headings and buttons depend only
  on the public `practiceAllowed` summary field.
- The changed implementation and tests neither add answer-bearing output nor
  expose report, score, correctness, feedback-body, credential, storage, or
  private-server state. The fixture's response token is transport scaffolding
  for the existing production component; assertions observe only rendered
  controls/headings and request-path behavior.

## Validation

- PASS: `npx prettier --check src/pages/run_page.tsx tests/playwright/run_prefetch_route.spec.ts tests/playwright/run_completion_summary.spec.ts tests/test_frontend_contract.mjs`
- PASS: `npx tsc --noEmit -p tsconfig.lint.json`
- PASS: focused ESLint on the changed component and the two Playwright fixtures.
- PASS: `node --import tsx --test tests/test_frontend_contract.mjs` (18 passed).
- PASS: `npx playwright test tests/playwright/run_prefetch_route.spec.ts tests/playwright/run_completion_summary.spec.ts` (12 passed).
- PASS: `git diff --check`.

The plain Node test launcher does not resolve this repository's extensionless
TypeScript fixture import; the project-correct `node --import tsx` invocation
above passed. This is environmental invocation detail, not a frontend failure.
