# Manual grading schema audit

Status: DONE_WITH_CONCERNS

Scope: schema-only audit for the next implementation package, "Add manual grading and mixed automatic/manual assignment behavior." This audit inspected `schemas/migrations/2026080804_activity_feedback.sql` and the dependent operations and retention migrations. It does not change production SQL.

## Decision

Edit the six-file pre-data baseline directly for this package. The authoritative status says the epoch still has no durable data and specifically permits editing these migrations until an environment accepts durable data; afterwards the same design must be introduced as a new forward-only migration. Evidence: `docs/active_plans/partial_commit_status.md:85-87`; the six-file ownership rule is `docs/active_plans/decisions/database_schema_evolution_plan.md:395-416`.

The bounded design is a one-row, tenant-owned current manual decision per question attempt. It must supplement rather than replace the immutable student submission and current automatic evaluation. Recalculation consumes the effective evaluation: automatic evaluation when no manual decision exists, otherwise the manual decision. This permits one assignment to contain automatically scored and manually scored attempts without introducing assignment versions, score history, or per-question-family schema.

## Evidence and gap table

| Requirement | Current evidence | Gap and consequence |
| --- | --- | --- |
| An attempt can await manual work | `schemas/migrations/2026080804_activity_feedback.sql:166-181` has one current evaluation with `needs_manual_grading`; `:199-217` has the matching attempt status. | There is no manual-decision identity, actor, optimistic revision, action idempotency, or status representing a completed manual decision. |
| Recalculation uses only automatic grades | `crates/learning-data-access/src/postgres.rs:820-860` accepts only `grading_status = 'graded'`; `:893-902` blocks a completed-run result when a current evaluation needs manual grading. | A manual grade cannot become score evidence, and mixed assignments never finish their summary until the SQL and Store contract change together. |
| Current state, not score history | One primary key per attempt exists for evaluation and score at `schemas/migrations/2026080804_activity_feedback.sql:320-324`; private generation staging exists at `schemas/migrations/2026080805_operations_analytics.sql:920-967`. | The new record must be mutable current state, not an append-only grade history. Minimal audit evidence belongs in `audit_event`, whose bounded action/target columns are at activity migration `:71-88`. |
| Retry-safe sensitive instructor action | Support actions already use a stable action identity and a minimal audit record in `crates/learning-data-access/src/postgres.rs:7850-7887`. | Manual grading needs its own exact action identity and request fingerprint; otherwise duplicate browser retries can cause ambiguous rewrites. |
| Tenant isolation and roles | The affected operational tables have forced RLS, tenant policies, and only `ple_app` write grants at activity migration `:505-560`. Plan requires separated application, grader, worker, analytics, and retention roles at `docs/active_plans/decisions/database_schema_evolution_plan.md:342-353`. | A new table requires the same forced RLS, default-deny posture, tenant policy, and narrowly justified grants. Do not give browser or statistics roles direct access. |
| Retention owns every learner record | Retention enumerates activity rows at `schemas/migrations/2026080806_retention.sql:441-450`, deletes evaluation and score rows at `:1323-1329`, checks residues at `:1567-1573`, fences current writes at `:2628-2632`, and grants its delete/select policies at `:2836-2842`. | Every new manual-decision row must be added to all five places, plus its course-binding trigger and retention broker policies, or privacy deletion will leave records behind. |
| Analytics is derived and does not delay grading | The plan requires course-local rerunnable analysis and flags for incomplete manual grading or recent rescoring at `docs/active_plans/decisions/database_schema_evolution_plan.md:304-319`. The current global aggregate is identity-free and catalog scoped at operations migration `:710-759` and `:1172-1189`. | Do not overload global `question_statistics_aggregate` with course records. This package only needs a future-safe recalculation hook; course-local item analysis is the next package. |
| Queue payloads remain bounded and identity-free of learner content | Worker job payloads are closed-shape and contain only identifiers/generations at operations migration `:796-870`; `recalculateAssignment` is already uniquely fenced by tenant/assignment/generation at `:1034-1036`. | Manual grade commit should atomically advance the assignment generation and insert/reuse that existing job shape, never put manual text, scores, student data, or responses in a job payload. |

## Recommended bounded schema contract

Add `public.manual_grade_current` to the activity/feedback migration, not a question-specific table and not a grade-event log. Recommended columns and invariants:

| Field | Contract |
| --- | --- |
| `tenant_id`, `attempt_id` | Composite primary key. `attempt_id` is also a foreign key to the tenant attempt/current submission evidence, with a course binding derived from the attempt rather than client supplied. |
| `credit_fraction`, `correct` | The current human decision, using the existing evaluation bounds and semantics. Permit partial credit; preserve the existing `-1000..1000` validation only if extra-credit semantics truly need it, otherwise tighten the manual path to `0..1` and let assignment scoring modes add extra credit. |
| `grader_id`, `graded_at` | Accountability and current-decision timestamp. The Store verifies the actor is an instructor for the course; the database must not trust a tenant id supplied by a client. |
| `revision` | Positive optimistic-concurrency revision. A write requires the expected revision, except initial creation. Concurrent graders get conflict, not last-writer-wins. |
| `action_id`, `request_sha256` | Stable UUID action id plus a 32-byte request fingerprint. A matching replay returns the existing decision; the same action id with different actor, attempt, or content is a conflict. This is the manual counterpart to the existing support-action retry discipline. |
| `feedback` and `feedback_sha256` | Optional bounded JSON object only for short structured rubric feedback. Enforce object shape and a small byte ceiling. Put long comments, files, annotated work, and media in the existing private object-store/delivery path and store object metadata rather than unbounded operational JSON. |
| `course_id` | Derived from the attempt for indexed gradebook/retention lookup; FK to the course. Its source must be validated by the existing or equivalent security-definer course-binding function. |

The table should have: `FORCE ROW LEVEL SECURITY`; a tenant policy using `ple_current_tenant()`; a `(tenant_id, course_id, attempt_id)` index for gradebook and purge paths; a unique `(tenant_id, action_id)` index; `ple_app` read/write only; and `ple_retention_broker` select/delete only. It should receive the same course bind and learner-record retention-fence triggers as `submission_evaluation`.

Keep `submission_evaluation` as the current automatic/authoritative-evaluation record, adding `manual_graded` to its status only if the Store needs a public single-row state marker. Prefer deriving effective state through a protected Store query joining `manual_grade_current`, so raw automatic evidence is not overwritten by a manual override. Either choice must maintain the invariant that an attempt has at most one current automatic evaluation and at most one current manual decision, with no old-score table.

Mixed behavior is data-driven: an automatic response writes `submission_evaluation`; an evaluator that returns manual-required writes its existing `needs_manual_grading` status; a human decision writes `manual_grade_current`. The assignment can therefore mix all three per delivered item. Do not add a rigid assignment-wide manual/automatic switch: adapters and question capabilities own whether a particular issued response can be graded automatically.

## Transaction and recalculation contract

Manual grade create/update must execute in one short transaction:

1. Set server-authenticated tenant context and lock the attempt/current evaluation and assignment in the same order used by other scoring changes.
2. Verify course instructor authority, submitted or auto-submitted non-cleared/non-exempt state, and that a manual-required evaluation exists when the adapter requires it.
3. Enforce action-id replay or expected-revision conflict; upsert the one current manual decision.
4. Advance `assignment.scoring_generation`, mark `recalculating`, and insert exactly one existing `recalculateAssignment` job for that generation.
5. Insert one minimal `audit_event` that names the action, actor, attempt, and revision but contains no response, manual text, or obsolete score.

The score staging query must calculate from the effective current decision and accept both automatic `graded` and manually completed outcomes. The completed-run summary must remain pending while any non-cleared/non-exempt delivered item has no effective final decision. This makes automatic items score immediately while a manual item blocks only the assignment run that needs it, as the current pending query already intends at `crates/learning-data-access/src/postgres.rs:871-902`.

Do not enqueue item analysis from this transaction unless the worker registry can claim it by family. The status documents that production claim filtering and complete worker composition are still incomplete at `docs/active_plans/partial_commit_status.md:60-64`. The next item-analysis package should add a separate generation-fenced, course-local work item and consume the same post-rescore completion boundary.

## Migration and rollback implications

- Pre-data baseline now: edit activity, operations only if an analytics-job shape/hook is added, and retention registration in the same six-file epoch. Re-run the empty database apply, second no-op, status, and verify evidence. No down migration is appropriate.
- After durable data: add one new forward migration. Use expand/backfill/switch/contract; never edit an applied checksum. Backfill only if an already-shipped state requires it, using batches and an explicit compatibility window.
- Rollback: application rollback must continue reading automatic evaluation when the new table is unused. Once manual decisions exist, use a compensating forward migration or disable new writes; never delete manual grade records merely to roll back code.
- Indexes above are justified by concrete read paths (course gradebook/purge and idempotent action lookup). Capture `EXPLAIN (ANALYZE, BUFFERS)` on production-shaped mixed assignments before accepting a new index, as required by the PostgreSQL skill evidence route.

## PostgreSQL oracles

Permanent behavior tests:

- Fresh PostgreSQL fixture applies all six migrations, re-applies as a no-op, and verifies ledger compatibility.
- An automatic-only item and a manual-required item in one assignment: automatic score is staged, run summary remains pending, a successful manual decision yields the expected current score and completed gradebook summary, and no score-history table/row remains.
- Partial manual credit, zero credit, and an assignment extra-credit mode use exact decimal values and recompute every affected attempt/retake through a new scoring generation.
- Same `action_id` and same request replay is idempotent; same action id with different content or actor conflicts; stale expected revision conflicts; concurrent manual writes leave one current decision and one current score per attempt.
- Manual feedback cannot exceed the chosen operational byte bound; large commentary is rejected or routed to object storage, never silently committed to operational JSON.
- Forced RLS fixtures show foreign tenant context sees/writes zero manual decisions, and student/statistics roles lack direct table privileges. A course instructor cannot grade an attempt outside that course.
- Retention fixture archives/deletes a course containing manual decisions, proves fence rejection after archive, removes manual records before their parent attempt/evaluation, and finds no residual learner records.
- Rescoring fixture records that manual decision causes the expected current generation replacement and that the job payload contains only assignment UUID plus generation.

One-time acceptance and operational probes:

- PostgreSQL 17 disposable-container migration rehearsal, including check of all grants, RLS policies, and trigger registrations; preserve the exact command/output with the package evidence.
- `EXPLAIN (ANALYZE, BUFFERS)` before/after captures for the gradebook/purge query on a representative mixed-assignment data set before retaining the recommended index.
- Once the worker registry is complete, run an end-to-end queue-drain proof that automatic grading does not wait for analytics and manual completion causes one generation-fenced rescore.
- Later, run the plan's real-role/RLS, partition-pruning, purge, backup/restore, and whole-system gates; the current status explicitly lists these as unfinished (`docs/active_plans/partial_commit_status.md:100-107`).

## Assumptions and risks

- Assumption: `submission_evaluation` is the preserved current automatic evidence and `manual_grade_current` is the current human override, rather than an append-only rubric/history feature. This matches the plan's immutable facts plus mutable current-state model at `docs/active_plans/decisions/database_schema_evolution_plan.md:23-31`.
- Assumption: the application already has authenticated course-instructor authorization reusable for manual-grade commands. The SQL role boundary alone is not sufficient authorization.
- Risk: force-submit currently transitions an attempt to `needs_manual_grading` without an evaluation or response (`docs/active_plans/partial_commit_status.md:39-42`). The manual command must explicitly distinguish "manual evaluation of a submitted response" from "support follow-up for a response-less force-submit," and must never invent a response or an automatic result.
- Risk: unbounded JSON is already visible in several existing operational payload columns. This package must not repeat it. A focused hardening decision for existing payloads is outside this bounded manual-grading schema audit, but the new manual feedback contract must be bounded from its first migration.
- Risk: course-local item analysis has no current table/projection. That is intentionally the next package, not a reason to distort the manual-grade table into analytics storage.

Artifact: `docs/active_plans/workstreams/manual_grading_schema_audit.md`
