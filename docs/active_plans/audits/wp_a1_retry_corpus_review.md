# WP-A1 retry corpus review

## Scope

- Reviewer: independent TypeScript/API contract review.
- Reviewed: `tests/playwright/simulator/retry_corpus.ts`, its focused spec, and
  [WP-A1 workstream](../workstreams/wp_a1_retry_corpus.md).
- Decision: ACCEPTED OFFLINE after final re-review; real-stack acceptance remains pending.

## Type Safety

- PASS: The narrow request-factory seam uses `unknown` at JSON boundaries,
  narrows the publication response to UUID references, and returns only the
  problem/version plus an arrangement label. It exports no source, answer,
  credential, or cookie-bearing value.
- PASS: A fresh API context owns the instructor session, and disposal is
  attempted after every post-context path. Transport, JSON, and disposal
  exceptions become staged redacted errors. A failed or ambiguous publish is
  attempted once only.
- REQUIRED: `isStrongEtag` accepts `"revision-1"`, but the actual save route
  accepts and emits only quoted positive decimal revisions (at most signed
  64-bit maximum). Make the client accept exactly that wire form and change the
  successful fixture to `"1"`; add a rejection case for `"revision-1"`.
  Otherwise the focused test approves an `If-Match` value that the real publish
  route rejects.
- REQUIRED: The public-detail guard misses answer-shaped fields including
  `correctResponse`, `answerKey`, `gradingPayload`, `privateGrading`,
  `expectedValue`, and `checkerState`. Add their case-normalized names to the
  recursive forbidden-field set and a focused `correctResponse` failure test.
  A catalog detail is never allowed to contain those fields, so this guard must
  fail closed rather than approving an answer disclosure under a different
  field name.

## Module Boundaries

- PASS: This module uses only supported local-development endpoints:
  `POST /api/auth/login`, `PUT /api/workspaces/{workspace}/flat-question`,
  `POST /api/problems/{workspace}/flat-question-publish`, and the catalog
  detail GET. It performs no SQL, account, enrollment, course, membership, or
  cleanup action.
- PASS: The authoring document matches the maintained v1 single-choice source
  contract: private `correctChoice`, `maxAttempts: null`, `immediateFull`, and
  `untimed`. The source selects a named RNG stream and keeps the first response
  wrong and second response correct without exporting that information.
- PASS: Save uses the required flat-question media type; publish uses the save
  ETag exactly and institution scope; the actual server returns 200 for save
  and 201 for publication. The local-development login route is appropriate to
  this separately gated local runner.

## Compile-Time Errors

- PASS: `npx tsc --noEmit -p tsconfig.json` completed successfully.
- PASS: `npx eslint tests/playwright/simulator/retry_corpus.ts
tests/playwright/simulator/retry_corpus.spec.ts` completed successfully.
- PASS: Prettier, `git diff --check`, and the targeted ASCII/shebang/whitespace
  pytest checks completed successfully (1,924 passed).

## Type-Level Tests

- PASS: `npx playwright test tests/playwright/simulator/retry_corpus.spec.ts
--reporter=line` completed successfully: 5 passed.
- NOTE: `--project=chromium` is not valid for this repository's current
  Playwright configuration because it defines no named projects; the projectless
  command above is the applicable focused command.
- REQUIRED: Extend the existing behavioral tests with the exact numeric ETag
  and `correctResponse` fail-closed cases described above. No live-stack claim
  is made by this review.

## Verdict

WP-A1 is not accepted offline yet. The supported route sequence, private
arrangement boundary, deterministic retry source, error redaction, no-retry
publish behavior, and cleanup ownership are sound. Acceptance is blocked only
by the fail-closed public-detail contract gaps above; neither requires a
product change.

## Re-review update

- PASS: The repaired ETag parser now accepts exactly the server's quoted,
  positive-decimal, signed-64-bit revision grammar. The focused test covers
  negative, zero, oversized, and nonnumeric values, while its success path
  sends the server-emitted `"1"` form unchanged.
- PASS: The repaired recursive detail guard is nested and case-insensitive for
  the newly covered singular aliases, including `correctResponse`.
- REQUIRED: Offline acceptance remains blocked because the expanded denylist
  still permits `rubric` plus plural answer-bearing aliases such as
  `answerKeys`, `answers`, and `correctResponses`. The repository's own
  learner-run security tests explicitly prohibit `rubric`; the simulator guard
  must do the same. Add the normalized aliases and a nested mixed-case failure
  test, then repeat this review.

## Final re-review

- PASS: The recursive, case-insensitive denylist now rejects `rubric`,
  `answerKeys`, `answers`, and `correctResponses` as well as the singular
  aliases. The focused public-detail probe nests mixed-case
  `CorrectResponses`, `Rubric`, and `AnswerKeys`, and it fails closed at the
  public-inspection stage.
- PASS: The exact ETag validation remains aligned with the Rust parser: quoted
  positive decimal, no leading zero, and no value above `i64::MAX`; the exact
  returned string is forwarded as `If-Match`.
- PASS: `npx playwright test tests/playwright/simulator/retry_corpus.spec.ts
--reporter=line` reports 6 passed. `npx tsc --noEmit -p tsconfig.json`, ESLint,
  Prettier, and `git diff --check` pass. The focused ASCII, shebang, whitespace,
  and Markdown-link pytest gate reports 2,060 passed.

## Final verdict

WP-A1 is ACCEPTED OFFLINE. It creates only a fresh private workspace source,
uses the observed local supported-API route/media-type/status/ETag contracts,
redacts credentials and private source from results and errors, and never
retries publication. This is not real-stack acceptance: M3 integration remains
pending WP-A2 and a live runner invocation of the arrangement.
