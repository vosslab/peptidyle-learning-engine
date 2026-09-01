# UI walkthrough plan consistency audit

## Status

**DONE_WITH_CONCERNS.** The walkthrough plan has a sound browser-first goal, but its M1-M3b
dependency chain contains blocking stale requirements. Do not begin the documented M1 package
unchanged.

## Governing source

- `AGENTS.md` makes `docs/active_plans/implementation_plan.md` and the active release plan the
  source of truth. It requires one dependency-ordered task and its narrow gate before advancing.
- `docs/HUMAN_GUIDANCE.md:17-25` makes the implementation plan authoritative and the release plan
  the remaining decision-complete sequence.
- `docs/active_plans/active/release_completion_plan.md:419-479` and
  `docs/active_plans/implementation_status.md:324-382` establish WP-RC8 production account-provider
  composition and acceptance as the immediate remaining package. The local-file provider is a
  pre-deployment-only mode, not canonical account enrollment.
- `docs/HUMAN_GUIDANCE.md:142-146` establishes eight v2 flat families. The current source confirms
  all eight discriminants in `crates/adapters/ple/src/flat_question/v2.rs:62-99`.

## Blocking contradictions

### Stale canonical JSON and enrollment checksum foundation

- **Walkthrough passages:** M1 requires `fixtures/canonical_json.ts` and a fixed enrollment checksum
  at `peptidyle-walkthrough-plan.md:351-368`; WP-V1 repeats the Rust `BTreeMap` and SHA-256 contract
  at `:511-521`; M3 requires `fixtures/enrollment_fixture.ts` and an enrollment GET checksum proof at
  `:384-395`.
- **Contrary passage in the same plan:** `:134-141` explicitly deletes the SQL enrollment fixture,
  canonical JSON encoder, checksum reproduction, and read-back self-check because enrollment has a
  browser interface.
- **Current source:** `crates/server/src/run/routes.rs:71-72` exposes only GET routes for an existing
  enrollment; it is not an enrollment-creation interface and cannot validate a client-created
  enrollment payload.
- **Severity:** blocking. The first dependency is deliberately deleted by the plan itself and would
  create an implementation-detail checksum test rather than browser behavior.
- **Resolution:** remove enrollment canonical JSON, checksum, and `fixtures/enrollment_fixture.ts`
  from M1, WP-V1, M3, the risk register, and the baseline. Retain a small seeded decision module only
  if its public contract is deterministic allocation and report ordering, with a behavior-focused
  offline test. Enrollment is walked through J9/J10 or reported as unavailable; it is never arranged
  through a fabricated enrollment fixture.

### Walked enrollment conflicts with arranged enrollment and stale gap language

- **Walkthrough passages:** `:140-141` says only course and assignment creation remain arranged, and
  `:185-191` says browser roster action makes enrollment walked. M3 nevertheless requires additional
  students to be enrolled by arrangement at `:390-395`; WP-A3 permits extra students to be arranged
  through endpoints at `:633-646`; the full-run requirement still lists login and enrollment as
  arrangements at `:911-914`. The close-out changelog text repeats the nonexistent login/enrollment
  UI claim at `:951-953`.
- **Severity:** blocking for M3/M3b. The same action is assigned both coverage states, so the baseline
  cannot truthfully classify a run.
- **Resolution:** make the single coverage table authoritative: J9/J10 are walked only after the
  canonical identity path is accepted; course and assignment creation are arrangements until product
  UI exists. For scale-only accounts, distinguish Account Creation from enrollment, record it as
  arrangement, and never put it in a "walked enrollment" baseline. Update the report and changelog
  gap wording to say exactly which surfaces are absent at the time of release.

### Identity path is not a prerequisite; the documented fallback is not valid

- **Walkthrough passages:** the plan uses launcher local-file credentials as a session at `:171-181`,
  leaves redemption compatibility as a pre-M3b experiment, and makes a passkey fallback at `:618-629`.
  It calls M3b nonblocking in the open-question section at `:1004-1018`.
- **Governing contradiction:** the release plan states production composition still constructs local-file
  development authentication at `release_completion_plan.md:421-429`; implementation status calls
  production account-provider composition the immediate package at `implementation_status.md:366-382`.
  Source confirms the router constructs local development authentication at
  `crates/server/src/composition.rs:73-85`.
- **Severity:** blocking. A local-file session cannot be assumed to own a canonical account record, and
  registering a passkey cannot repair a missing email-account identity without first reaching the
  canonical registration/sign-in flow.
- **Resolution:** add accepted WP-RC8 production account-provider composition plus its browser
  acceptance as an explicit predecessor to M2 live mode and M3b. Replace the fallback with one
  narrow preflight: create a real canonical account, walk the invitation copy-link claim, and prove
  the roster readback. If it fails, the walkthrough reports its prerequisite as BLOCKED; it must not
  silently switch identity systems.

### Six-family corpus contradicts the eight-family release contract

- **Walkthrough passages:** the current-state claim names six families at `:193-205`; WP-A1 creates
  six implementations at `:558-571`; M6 and its risk record require all six at `:437-447` and
  `:939`; the resolved decision repeats six at `:989-992`.
- **Governing and source evidence:** human guidance requires MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER,
  and HOTSPOT at `HUMAN_GUIDANCE.md:138-146`; the changelog calls the v2 source all eight at
  `CHANGELOG.md:7-16`; source has `MultipleAnswer`, `FillIn`, `MultiFillIn`, and `Hotspot` in addition
  to matching, ordering, numeric, and single choice at `v2.rs:62-99`.
- **Severity:** blocking for WP-A1 and M6. A six-family `QuestionAuthor` cannot meet the released
  response contract or exercise multi-blank and hotspot accessibility.
- **Resolution:** rename the family table and interfaces to the eight canonical family names. Make M6
  depend on accepted WP-RC4 and the secure student-payload package before claiming all-family live
  coverage. If the walkthrough is intentionally scoped to a smaller temporary subset, name it as a
  subset and do not use "every v2 response family" or a release acceptance claim.

### Undefined dependency leaves the first student journey unreachable

- **Walkthrough passage:** WP-A3 depends on `WP-E1` at `:633-638`, but the plan defines no WP-E1 in
  its work-package inventory (`:509-816`) or dependency map.
- **Severity:** blocking. WP-W1 depends on WP-A3 at `:649-658`, so M4 has no executable predecessor.
- **Resolution:** replace `WP-E1` with a defined predecessor. The least invasive sequence is accepted
  WP-RC8 identity composition -> WP-O1/WP-O2 live preflight -> J9/J10 canonical invitation proof ->
  scale-account arrangement, if still needed -> WP-W1. Do not invent a new package merely to retain
  the obsolete SQL-era dependency.

## Nonblocking corrections before close-out

- **Release order:** `implementation_status.md:324-356` puts WP-RC8 closeout before WP-RC4/P1-P6,
  WP-RC5, RC7, and RC12. The walkthrough may remain an opt-in E2E package, but its "implementation
  may start" statement at `peptidyle-walkthrough-plan.md:1004-1006` must not bypass WP-RC8 or claim
  final all-family coverage before RC4/payload acceptance.
- **Source-file limit:** the untracked plan is already 1,037 lines. Once tracked, the line-limit test
  discovers tracked Markdown (`tests/test_source_file_line_limit.py:161-189`) and fails at 1,000
  (`:222-232`). The plan's proposed exception at `:944-948` needs an explicit manager decision; it
  conflicts with the owner rule to keep every source file below 1,000 lines (`HUMAN_GUIDANCE.md:12-13`).
  Prefer splitting the plan into an active plan plus a compact decision/evidence appendix before
  filing it. Do not add a new override as routine implementation work.
- **Permanent pytest scope:** WP-G1 is appropriate only as a fast static boundary test. Do not add
  seeded violation files or live/browser behavior to pytest: `PYTEST_STYLE.md:7-26` requires durable,
  offline, fixed-seed behavior and `:97-102` moves slow or filesystem-heavy work to E2E/Playwright.

## Dependency-correct first implementation slice

1. Complete the already-authoritative WP-RC8 production account-provider composition and its focused
   browser proof; do not modify the simulator first.
2. Reconcile the walkthrough plan into a sub-999-line active plan and remove the obsolete checksum,
   SQL, six-family, and old-gap statements above.
3. Implement one bounded simulator preflight: WP-O1/WP-O2 load the existing live gateway and validate
   inputs without starting the mock server. Its acceptance is a canonical-account sign-in plus an
   instructor-created copy-link invitation and student claim through visible browser controls. It
   produces no corpus, course, assignment, scale accounts, or score assertions.

This slice preserves the one-axis coverage model, makes identity a real prerequisite, and gives M3
one trustworthy state boundary from which later arrangements and journeys can proceed.

## Handoff

- Status: DONE_WITH_CONCERNS.
- Artifact: `docs/active_plans/audits/ui_walkthrough_plan_consistency_audit.md`.
- Changed files: this audit only; no production source or user-owned walkthrough-plan edits.
- Validation: run the single-file ASCII checker and focused Markdown check after writing this report.
- Residual risk: release-plan acceptance may change the canonical provider or flat-family closeout before
  the walkthrough begins; re-read the active release plan and implementation status immediately before
  dispatching the first implementation package.
