# UI/UX walkthrough plan reconciliation review

## Scope and result

Reviewed the binding walkthrough plan and its nonbinding evidence companion
independently against the active release plan, implementation status,
enrollment contract, launcher, native seed, and browser source. Result:
**CHANGES REQUIRED**. The reconciliation fixes the earlier stale six-family,
local-account-fallback, SQL-fixture, undefined-package, and line-limit
problems, but three execution-blocking dependencies remain ambiguous or
contradictory.

Both reviewed files are below the 1,000-line limit: the plan is 681 lines and
the evidence companion is 64 lines. Their headings follow the multi-workstream
plan shape, every listed package has one owner, package dependencies, outcome,
and evidence, and M5-M7 state a bounded maximum parallel-doer count.

## Findings

### High - WP-RC8 package acceptance is a circular prerequisite for M3b

- **Location:** `docs/active_plans/peptidyle-walkthrough-plan.md:194-200` and
  `:376-381`.
- **Evidence:** WP-RC8 is implemented and independently reviewed, but its
  package acceptance remains open. Its release-plan validation expressly
  includes the real email-authentication, invitation, and browser E2E evidence
  that M3b/WP-W10 is meant to produce. Requiring accepted WP-RC8 package gates
  before M3b therefore requires the result before its producer can run.
- **Fix:** Replace both dependencies with the already satisfied,
  independently-reviewed production-composition prerequisite, for example:
  `the implemented and independently reviewed WP-RC8 provider-free production
composition, not WP-RC8 package acceptance`. Keep the separate operator
  canonical-email-browser prerequisite as the onboarding preflight's runtime entry criterion.
  M3b/WP-W10 evidence can then contribute to, rather than wait for, final
  WP-RC8 acceptance.
- **Retest:** Have a fresh reviewer trace the revised dependency wording to
  `release_completion_plan.md` WP-RC8 validation and verify that no required
  M3b result is also an M3b entry condition.

### High - WP-A2 must reuse the launcher's seeded course and mastery assignment

- **Location:** `docs/active_plans/peptidyle-walkthrough-plan.md:179-190` and
  `:359-370`.
- **Evidence:** `launch_local_stack.sh` creates `containers/local-demo.json`
  after `cargo tools e2e-seed`. The native seed creates one course with the
  local instructor and student as members, one mastery assignment, and that
  student's enrollment. A new course would not carry that seeded student
  membership; the stated "one seeded student" rule would then fail without the
  prohibited enrollment fixture.
- **Fix:** Rename WP-A2 to reuse the launcher's seeded course and mastery
  assignment by reading the local demo manifest. It may arrange only later
  corpus and an exam-contrast assignment inside that seeded course through the
  supported API, and must verify the existing student receives access through
  the product's assignment-creation enrollment behavior. Do not create a
  course, membership, account, or enrollment. Update M3 deliverables and
  arrangement-report wording accordingly.
- **Retest:** Start the ordinary launcher, read the generated manifest, and
  prove the seeded student opens the reused mastery assignment and any later
  arranged contrast assignment through the browser without SQL or a manually
  created enrollment.

### High - Canonical email onboarding conflicts with the stated mailbox

non-goal

- **Location:** `docs/active_plans/peptidyle-walkthrough-plan.md:81-82`,
  `:199-203`, and `:378-381`.
- **Evidence:** The passwordless sign-in surface sends a one-time link and
  tells the user to open it in the browser. The enrollment contract requires a
  live operator-selected external SMTP provider test account for the canonical
  email-authentication ceremony. Excluding both external SMTP delivery and
  mailbox interaction makes a real J9/J10 canonical email account walk
  impossible, even though provider-system acceptance is rightly outside this
  simulator.
- **Fix:** State the boundary precisely: the simulator does not test SMTP
  provider implementation, deliverability, or mailbox-service behavior; it
  consumes an operator-provided test mailbox/account solely to complete the
  PLE email-authentication browser ceremony. The onboarding preflight must require that access,
  and WP-W10 must record the PLE browser outcome while redacting the challenge
  token and mailbox artifacts. Retain `BLOCKED` when the operator prerequisite
  is absent.
- **Retest:** A redacted preflight fixture distinguishes unavailable operator
  mailbox/provider access (`BLOCKED`) from a PLE sign-in failure (`FAIL`), and
  the canonical path never substitutes a copied invitation link or passkey.

### Low - `deferred` is outside the declared result vocabulary

- **Location:** `docs/active_plans/peptidyle-walkthrough-plan.md:644-645`.
- **Evidence:** WP-V2 declares only `PASS`, `BLOCKED`, `NOT_APPLICABLE`, and
  `FAIL`, while documentation close-out instructs progress tracking to mark a
  package "deferred." That invites a fifth, undefined terminal-looking state.
- **Fix:** Replace `deferred` with `NOT_APPLICABLE` when capability does not
  apply, `BLOCKED` when evidence or a dependency is missing, or an explicit
  nonterminal package-progress field such as `pending`.
- **Retest:** Add a report-schema assertion that terminal journey outcomes are
  exactly the four declared values and progress metadata cannot be mistaken for
  a journey outcome.

## Confirmed corrections

- The plan preserves arrangement versus walked-browser evidence and excludes
  local-file promotion, SQL enrollment, checksums, canonical enrollment JSON,
  and passkey fallback.
- It names the current eight response families: MC, MA, FIB, MULTI-FIB, NUM,
  MATCH, ORDER, and HOTSPOT.
- It removes the prior undefined WP-E1 dependency, uses the real roster route
  `/instructor/courses/:courseId/students`, keeps all-family claims behind RC4
  and secure-payload gates, and treats local-stack onboarding as `BLOCKED`.
- The evidence companion is correctly nonbinding and directs dispatchers back
  to active source-of-truth plans and current route source.

## Validation

- `wc -l docs/active_plans/peptidyle-walkthrough-plan.md docs/active_plans/decisions/peptidyle_walkthrough_evidence.md` - 681 and 64 lines.
- `source source_me.sh && python3 tests/check_ascii_compliance.py -i <each reviewed file>` - passed.
- `source source_me.sh && python3 -m pytest tests/test_markdown_links.py -q` - 136 passed.
- `git diff --check -- <both reviewed files>` - passed.
- Heading scan and outcome-vocabulary scan completed; the sole vocabulary inconsistency is the finding above.

## Acceptance decision

Do not dispatch M3 or M3b until the three High findings are corrected and a
fresh review verifies the revised dependency path. The lower-risk static and
documentation-quality portions are otherwise execution-ready.

## Re-review - 2026-08-10

### Result

**CHANGES REQUIRED - one residual Medium finding.** The three original High
findings and the Low vocabulary finding are corrected. The revised plan is
otherwise execution-ready after this one assignment-contract clarification.

### Confirmed fixes

- M3b and the onboarding preflight now depend on the implemented, independently reviewed
  provider-free WP-RC8 production composition, explicitly not final WP-RC8
  package acceptance. WP-W10 contributes the browser evidence needed by that
  later acceptance, so the dependency is noncircular.
- The plan now reads `containers/local-demo.json`, resolves its `assignmentId`
  through supported authenticated assignment or course reads, and reuses the
  seeded course. It prohibits creating a course, membership, enrollment,
  account, local identity, or SQL fixture.
- The onboarding preflight correctly treats an operator-selected provider, test mailbox, and
  delivered link as a runtime prerequisite. The simulator consumes that
  prerequisite to test the PLE browser ceremony, while explicitly excluding
  SMTP transport, deliverability, provider UI, and mailbox-service validation.
  It redacts mailbox artifacts and distinguishes unavailable operator access
  (`BLOCKED`) from a PLE sign-in failure (`FAIL`).
- Package-progress vocabulary now permits nonterminal `pending`; journey
  outcomes remain exactly `PASS`, `BLOCKED`, `NOT_APPLICABLE`, and `FAIL`.

### Medium - the seed is not sufficient for the retry/mastery journey

- **Location:** `docs/active_plans/peptidyle-walkthrough-plan.md:103-108`,
  `:198-199`, and `:382-393`.
- **Evidence:** The source seed's assignment policy is `AnswerAll`, `Highest`,
  `Unlimited`, and `NewSeeds`, but its sole native question has
  `max_attempts: Some(1)` and deferred feedback. It cannot provide the
  incorrect-then-correct retry evidence required by WP-W2. The plan says it
  reuses a "seeded mastery assignment" and creates later assignments including
  an exam contrast, but does not explicitly require a separate later
  retry-capable mastery assignment or name the required corpus policy.
- **Fix:** Call the reused seed a baseline seeded assignment, not the mastery
  retry fixture. Require WP-A1/WP-A2 to publish and arrange a later minimal
  mastery assignment in the reused seeded course whose corpus has unlimited
  attempts and immediate feedback, then make WP-W1/WP-W2 consume that
  arrangement. Keep the later exam contrast separate. The seed may still prove
  manifest resolution and basic course access.
- **Retest:** The focused browser evidence must show incorrect then correct on
  the later mastery assignment, while the arrangement report names its corpus
  and assignment separately from the launcher seed.

### Re-review validation

- `wc -l docs/active_plans/peptidyle-walkthrough-plan.md docs/active_plans/decisions/peptidyle_walkthrough_evidence.md` - both remain below 1,000 lines.
- `source source_me.sh && python3 tests/check_ascii_compliance.py -i <each reviewed file>` - passed.
- `source source_me.sh && python3 -m pytest tests/test_markdown_links.py -q` - 136 passed.
- `git diff --check -- <both reviewed files>` - passed.

## Final re-review - 2026-08-10

### Result

**ACCEPTED.** The residual retry-fixture finding is resolved without changing
the product or weakening the browser-evidence boundary.

### Verified resolution

- The plan now calls the launcher output a baseline native assignment rather
  than mastery coverage and accurately records that its single-attempt,
  delayed-feedback question cannot demonstrate an incorrect-then-correct
  retry.
- WP-A1 requires a separately published retry corpus with
  `max_attempts: None`, immediate full feedback, and untimed behavior.
- WP-A2 resolves the seeded course from manifest `assignmentId` through a
  supported authenticated read, then arranges distinct Mastery
  (`AllCorrect`, `Highest`, `Unlimited`, `NewSeeds`) and Exam assignments only
  within that course. The existing seeded student receives those assignments
  through product assignment-creation enrollment behavior; the simulator does
  not create enrollment.
- M3, WP-W1, and WP-W2 now consume the newly arranged retry-capable Mastery
  assignment. The baseline seed remains limited to manifest resolution and
  basic course-relationship arrangement evidence.
- The corrected noncircular WP-RC8 prerequisite, operator-mailbox boundary,
  and explicit package-progress versus journey-outcome vocabulary remain
  intact.

### Final validation

- `wc -l docs/active_plans/peptidyle-walkthrough-plan.md docs/active_plans/decisions/peptidyle_walkthrough_evidence.md` - 727 and 80 lines.
- `source source_me.sh && python3 tests/check_ascii_compliance.py -i <each reviewed file>` - passed.
- `source source_me.sh && python3 -m pytest tests/test_markdown_links.py -q` - 136 passed.
- `git diff --check -- <both reviewed files>` - passed.
- Stale dependency and vocabulary scan found no accepted-WP-RC8 dependency,
  `deferred` outcome state, six-family claim, or live `WP-E1` dependency.
