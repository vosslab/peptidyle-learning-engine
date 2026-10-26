# Authentication consolidation and Question processing investigation

Status: completed and verified. Updated 2026-10-26.

## Goal and context

Remove repeated authentication plumbing while preserving authorization, and investigate the first
Question processing failure at its source. Follow [Human Guidance](../../HUMAN_GUIDANCE.md) and
[the pilot scope](../../FALL_2026_PILOT.md). Native JSON and WeBWorK are the current Backends;
iMathAS remains required future work and is outside this change.

An incorrect Student answer is a successful grading operation, usually producing zero or partial
credit. A processing failure means PLE cannot retrieve the source, obtain a grading result, or
commit that result. Temporary infrastructure outages are expected operational failures. Incorrect
grading, lost Student work, application processing errors, and failures to recover correctly after
service restoration are release-blocking defects.

### Investigation evidence and limits

- Source inspection found 39 ordinary PostgreSQL transaction/session setup copies, including
  32 equivalent 25-line helpers. Two other callers set special transaction isolation first.
  `sessions.rs` resolves and returns the session itself and has a different responsibility.
- There are 28 identical cookie-header helpers and one different implementation in
  `support_capability.rs`. The latter accepts UTF-8 bytes where the common helper uses
  `HeaderValue::to_str`; that difference must be resolved deliberately.
- The expiry query repeatedly selects the oldest 100 unfinished Attempts. A one-time SQLite
  model using the source candidate query confirmed that 100 persistent processing failures can
  exclude later work. With 99 failures, only one healthy Attempt advances per batch.
- That model does not demonstrate a real Question processing failure, its likelihood, or a
  PostgreSQL/runtime defect. The user's priority is preventing and diagnosing the first failure.
  The backlog scenario does not justify a cursor, queue, retry-state table, or scheduling redesign.
- Current code evaluates saved responses before committing submission. The worker retries
  unfinished expired Attempts and logs the failure stage and error category. Those observations
  are not connected proof of preserved work or successful recovery.

## Approach

### 1. Consolidate cookie parsing

Expose the existing `auth.rs` helper as `pub(crate)` and use it from the 28 identical callers.
Use the common strict header conversion for `support_capability.rs` after a focused check of
ordinary session cookies, repeated headers, invalid header bytes, and duplicate session tokens.
Preserve the existing token parser's rejection of multiple valid session tokens. Keep role checks,
route-specific responses, and Sysadmin access confirmation in their current owners.

### 2. Consolidate database session installation

Use the existing PostgreSQL connection module for two small functions: install the authenticated
application session into an existing transaction, and begin an ordinary transaction then install
that session. Replace the 39 ordinary copies with the latter. Design and cleanup inform each other:
remove every authentication-only forwarding method and call the shared function directly. Retain
methods with distinct domain work, including publication timeout and transaction isolation setup.

The Watch inbox and Instructor Student View keep their local transaction creation and their first
`SET TRANSACTION` statement, then call the shared session installer. Preserve repeatable-read and
read-only behavior. Leave session-record lookup and service-role transactions with their owners.
Keep bound token parameters, missing-session rejection, transaction-local roles, rollback behavior,
and the existing database authorization procedures. Add no role-selection flags or generic Store
framework.

### 3. Investigate one Question processing failure

Use the existing disposable connected harness and actual native JSON/WeBWorK paths. First exercise
a valid correct response, a valid wrong response, and an unanswered Question through submission;
verify their expected stored outcomes. Then use temporary fault injection to exercise one renderer
failure and one failed database commit, followed by restoration and a retry of the same Attempt.
Check saved responses, absence of a fabricated zero score or partial submission, final result,
duplicate-submit behavior, Student feedback, and the existing operator diagnostic.

Trace every unexpected result to its owning source retrieval, grading, commit, or response handler.
Fix a demonstrated defect there and keep the corresponding narrow regression test. Incorrect
grading, lost Student work, application errors, and failed recovery block release; retries alone
do not repair those defects.
Record infrastructure-failure behavior separately from defects in valid Question processing.
If no defect is demonstrated, close the original backlog finding as an unproven priority and make
no scheduling change. Automatic server shutdown is not specified by this plan.

## File scope

- `crates/server/src/auth.rs` and current cookie-helper callers: one shared parser and imports.
- `crates/learning-data-access/src/postgres/connection.rs` and the session-setup callers: shared
  session installation, preserving the two special transaction boundaries and session lookup.
- `crates/server/src/assessment_delivery/`, `crates/server/src/worker.rs`, the matching PostgreSQL
  adapters, and `schemas/base_schema/50_functions/assessment_attempt_finalization.sql`: investigation
  targets; production edits only for demonstrated defects. Inspect matching SQL wrappers/grants
  if a necessary correction changes a procedure contract.
- Existing connected tests under `tests/e2e/` and owning Rust modules: behavioral regressions only.
  Use `tests/_temp/` for one-time probes and remove them after recording the evidence.
- `docs/CHANGELOG.md` and this plan: record each bounded completion and its verification limits.
  Update generated schema documentation only if its canonical SQL source changes.

## Completion checks

- One-time call-site/diff review confirms the duplicate setup is removed and domain authorization
  remains intact. Do not add permanent tests that pin helper counts or file layout.
- Connected checks prove valid sessions work, absent/revoked sessions fail, transaction-local
  identity does not leak through pool reuse, and the special isolation/read-only settings remain.
  Reuse existing authorization coverage; add only missing meaningful cases.
- The single-Question investigation records actual results, root cause where found, and whether
  saved work and grading recover correctly. Fault-injection results are labeled separately from
  normal-path failures. Failed integrity/authorization checks block completion and require repair.
- Run `source source_me.sh && ./launchers/run_fast_checks.sh` after the consolidation, then
  `source source_me.sh && bash tests/e2e/e2e_database_baseline.sh` and the focused connected
  processing checks. Run affected real-stack authentication journeys for the shared auth changes.
- Keep separate milestones for cookie consolidation, database consolidation, and processing
  investigation. No unrelated dead-code cleanup, iMathAS changes, or new infrastructure.

## Implementation evidence

- Cookie consolidation complete: one owner replaces 29 implementations. The added permanent test
  covers multiple Cookie headers, duplicate session credentials, and strict header conversion.
- Database consolidation complete: 41 installation sites share the connection owner; all 38
  authentication-only forwarding methods are removed. The 39 ordinary sites include one inline
  setup. Publication timeout and the two special transaction-isolation operations remain intact.
  Rust source has a net reduction of 1,258 lines, including the new regression test.
- Full offline gate passed: Rust formatting, compilation, strict Clippy, tests, WASM target,
  TypeScript/Node checks, and 8,949 Python checks. The updated application image was rebuilt.
- Temporary connected Rust probe passed through the actual helper with a one-connection pool:
  valid session accepted; unknown and revoked sessions rejected; identity and role cleared after
  transaction reuse; repeatable-read and read-only settings preserved. Its source was removed.
- Existing real-stack `auth_authorization` and `sysadmin_student_data_access` journeys passed.
  Initial runs encountered a Chromium sandbox restriction and stale local certificate pin;
  sandbox escalation and the existing gateway-trust refresh resolved those test-environment issues.
- Temporary real-stack processing probe passed. Pausing the renderer produced HTTP 503 after its
  configured 15-second timeout. A trigger scoped to the new test Attempt injected a commit failure
  (HTTP 412). Both preserved the saved-response fingerprint, recorded no submission or grading
  result, and showed the Student a retry message. Removing the faults allowed the same Attempt
  to finish; the correct native answer earned full credit and WeBWorK returned a grading result.
  Repeated submission left the result unchanged. Separate wrong-answer and unanswered cases
  finished normally with zero credit or no backend grading result, respectively.
- The renderer was resumed and the scoped trigger/function removed in cleanup; catalog inspection
  confirmed no injection trigger remained. Temporary browser source was removed. Existing API
  telemetry recorded the 503 and 412 with request IDs and elapsed time, without response content.
- No valid-Question processing defect was reproduced. Close the backlog proposal without a
  scheduling change. Temporary infrastructure failures recovered correctly in these bounded cases.
- Exclusive database baseline passed. The Live Demo was restarted, reseeded, and reported ready
  at `https://localhost:8314/sign-in`.
