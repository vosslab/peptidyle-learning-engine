# Manual grading PostgreSQL book-grounded design review

## Status and decision

**Ready to freeze a one-row current evaluation plus a minimal idempotency
receipt.**  This is a read-only design review.  It recommends no production
edit and keeps the next package limited to manual item evaluation and mixed
automatic/manual completion; course-local item analysis remains the following
package.

The current six-file SQLx epoch has no durable data, so its owner may still
consolidate a consistent baseline change.  The recorded checkpoint nevertheless
describes it as an audited six-file boundary.  A forward migration is therefore
the safer implementation default; editing the baseline is acceptable only if
the package redoes the clean-apply, no-op, status, and verify evidence.  Once
durable data exists, a forward migration is mandatory.

The authoritative database-evolution plan wins over the older implementation
plan where they disagree: `submission_evaluation` is mutable current state;
raw submissions are immutable evidence; score and assignment-summary rows are
replaceable projections; and no score/evaluation history table is added.

## Recommendation

Use these two relations, rather than a `manual_grade_current` override table:

1. Extend the existing tenant-owned `submission_evaluation` row so a real
   response can be pending manual review and then become its current final
   evaluation.
2. Add a small tenant-owned `manual_grade_receipt` relation for replay safety
   and minimal audit evidence.  It records an action identity and a digest,
   not a past grade, rubric, response, or answer.

This is the smallest normalized model for the current workload.  A manual
item grade is not a second independent fact beside an automatic evaluation:
it is the current normalized evaluation consumed by the existing scoring
generation pipeline.  A separate one-to-one manual-decision table would make
every scoring read choose an "effective" grade and creates two mutable truth
sources for one attempt.  Its only advantage would be preserving an automatic
result after a manual correction, which is precisely the obsolete grade-history
behavior the accepted plan rejects.  The plan's optional separate manual
**assignment override** remains a different, explicitly unscheduled feature;
it must not be smuggled into this item-evaluation model.

This conclusion reconciles the audits: the schema audit proposed a separate
manual-current row, while the Store-contract, server/API, PostgreSQL Store, and
spec-reconciliation audits converge on replacing the sole current evaluation.
The latter matches the accepted plan's explicit designation of
`submission_evaluation` as the current normalized evaluation/manual status.

### Exact proposed relational shape

Retain the current composite primary key:

```text
submission_evaluation PRIMARY KEY (tenant_id, attempt_id)
```

Add:

```text
evaluation_revision bigint NOT NULL DEFAULT 1
  CHECK (evaluation_revision > 0)
```

Keep `tenant_id`, `attempt_id`, `submission_id`, `course_id`, `payload`,
`payload_sha256`, and `evaluated_at` non-null.  `submission_id` stays
non-null because the supported manual operation grades a response-bearing
submission.  A support force-submit remains deliberately response-less and
creates neither submission nor evaluation; it is cleared/recovered through the
existing support workflow, not manually scored by inventing evidence.

Make only `credit_fraction` and `correct` nullable, then replace the current
status-only check with named checks equivalent to:

```sql
CHECK (credit_fraction IS NULL OR credit_fraction BETWEEN -1000 AND 1000),
CHECK (
  (grading_status = 'needs_manual_grading'
   AND credit_fraction IS NULL AND correct IS NULL)
  OR
  (grading_status IN ('graded', 'exempt')
   AND credit_fraction IS NOT NULL AND correct IS NOT NULL)
)
```

The manual-pending row has a fixed non-secret payload marker (for example,
`{}`) and its normal checksum; it contains no fabricated numeric result.  The
manual command writes a normalized credit and the server-derived `correct`
field, switches status to `graded`, increments `evaluation_revision`, and
updates `evaluated_at`.  No client supplies point values, assignment mode,
correctness, tenant, course, or result payload.

`manual_grade_receipt` should have the following non-null columns:

```text
tenant_id uuid
manual_grade_action_id uuid
attempt_id uuid
actor_id uuid
request_sha256 char(64)
expected_evaluation_revision bigint
resulting_evaluation_revision bigint
scoring_generation bigint
course_id uuid
occurred_at timestamptz default transaction_timestamp()
PRIMARY KEY (tenant_id, manual_grade_action_id)
```

Use check constraints for positive revisions/generation and the fixed digest
length; use a tenant-leading foreign key to the current evaluation where the
existing partition/key layout permits it, otherwise have the transactional
attempt/evaluation lookup establish that ownership.  The receipt's primary key
is intentionally action-first rather than `(attempt_id, action_id)`: reusing
an idempotency key for a different actor, attempt, or request must conflict,
not become a second successful action.  On conflict, compare every immutable
request identity field and digest before replaying the recorded revision and
generation.  Do not add a unique key involving nullable components: PostgreSQL
unique constraints allow multiple nulls, so such a key is not an idempotency
proof.

The receipt is minimal audit evidence, not grade history.  It permits exact
retry identification while the current evaluation remains the only mutable
grade.  If the project requires the generic `audit_event` as well, write a
single bounded event in the same transaction, with action/actor/attempt and
digest identity only; never copy the numeric value or protected response.

## What the local PostgreSQL books establish

The following are database principles; applying them to manual grading is the
project-specific inference stated above.

| Local source and section | Establishes | Application here |
| --- | --- | --- |
| `PostgreSQL_16.0_Documentation-2023.md`, **5.4.1 Check Constraints** and **5.4.2 Not-Null Constraints** | A multi-column `CHECK` can express row-local state shape, but `CHECK` passes on NULL; `NOT NULL` is needed where absence is forbidden. | Use a conditional check plus nullable result columns so pending has neither credit nor correctness, while terminal rows require both. |
| `PostgreSQL_16.0_Documentation-2023.md`, **5.4.3 Unique Constraints**; `PostgreSQL_Query_Optimization_Ultimate_Guide_to_Efficient_Queries-2024.md`, **Unique Indexes and Constraints** | Composite primary keys are unique and non-null; ordinary unique constraints may admit NULLs. | Make the one current row and one action identity explicit with non-null tenant-leading keys; do not attempt a nullable composite UPSERT identity. |
| `PostgreSQL_16.0_Documentation-2023.md`, **5.8 Row Security Policies** | RLS complements privileges; no policy is default-deny; table owners normally bypass unless `FORCE ROW LEVEL SECURITY` is set. | Apply forced RLS, the existing tenant setting policy, and least-privilege grants to the receipt just as to evaluation. |
| `PostgreSQL_16.0_Documentation-2023.md`, **13.2.1 Read Committed Isolation Level**, **13.3.2 Row-Level Locks**, **13.3.4 Deadlocks**, **13.3.5 Advisory Locks**, and **13.4.2 Enforcing Consistency with Explicit Blocking Locks** | Read Committed re-evaluates a concurrently updated target; row/advisory locks can serialize a bounded operation; inconsistent lock ordering can deadlock. | Hold the established assignment advisory lock, then locks in one fixed order, then an action-id lock/receipt lookup and the evaluation row. Keep the transaction short. |
| `PostgreSQL_16.0_Documentation-2023.md`, **INSERT: ON CONFLICT Clause** | A matching unique arbiter supports atomic insert-or-update, but a conflict still locks the candidate row and the statement is deterministic only for one proposed change per target row. | Use the receipt key as the idempotency arbiter; verify its digest/identity before replay. Do not let `ON CONFLICT` turn a different request into a silent overwrite. |
| `PostgreSQL_Mistakes_and_How_to_Avoid_Them-2025.md`, **6.5 Allowing long-running transactions** | Idle/long transactions can retain locks and obstruct other work. | Validate the HTTP body before beginning the transaction; do no rubric rendering, external I/O, or worker drain while holding manual-grade locks. |
| `PostgreSQL_Query_Optimization_Ultimate_Guide_to_Efficient_Queries-2024.md`, **Understanding Execution Plans** and **Unique Indexes and Constraints** | Planner choice depends on workload/statistics; primary/unique constraints already create useful unique indexes; foreign keys do not automatically index child lookup paths. | Reuse the evaluation and receipt primary-key access paths. Add an index only after the actual manual queue/purge query is fixed and measured. |

## Transaction, lock, and RLS contract

Under the project application's authenticated tenant transaction context:

1. Validate request format and canonical decimal before `BEGIN`.
2. Set tenant context through the existing `begin_tenant` path.
3. Resolve tenant-scoped attempt/run/enrollment/assignment only to obtain the
   existing assignment advisory-lock key; acquire it first.  For a future
   batch command, sort `(tenant_id, assignment_id)` before acquiring any.
4. Lock the attempt, run, enrollment, assignment, and evaluation in the same
   established order.  Recheck persisted direct-course-instructor authority,
   response-bearing submitted state, pending status, and expected revision.
5. Serialize the action identity, inspect the receipt, and replay only when
   actor/attempt/digest/revision all match.  Mismatches are conflicts.
6. Replace the sole evaluation, advance assignment scoring generation, mark
   recalculating, enqueue/reuse only the matching generation-fenced scoring
   job, write receipt/audit, and commit.

The project-specific lock ordering above is required because the manual grade
touches both an assignment-level generation and an attempt-level evaluation.
The database documents the lock primitives, not this application order.  The
existing policy-writer order is the compatibility anchor; the implementation
must not introduce a reverse evaluation-then-assignment path.

`submission_evaluation` already has forced RLS and an app/retention grant
shape.  `manual_grade_receipt` needs the same: enable and force RLS; tenant
`USING` and `WITH CHECK` policies tied to `ple_current_tenant()`; application
read/insert access only as needed; retention broker select/delete only; no
student, statistics broker, worker, or browser direct access.  Register it in
the course-binding trigger, retention fence, purge sequence, residual checks,
and retention-broker policies.  RLS is a backstop; application queries still
include `tenant_id` predicates and derive course/actor from persisted rows.

## Index decision

No new manual-evaluation index is justified yet.  The operational mutation and
replay each have equality predicates on existing/new primary keys:

| Query shape | Index used | Decision |
| --- | --- | --- |
| evaluation by tenant and attempt | existing `submission_evaluation_pkey` | Retain; it is exact. |
| replay by tenant and action id | `manual_grade_receipt` primary key | Add as the required integrity/index structure. |
| assignment generation/current-score publication | existing assignment/staging/job indexes | Retain; no manual-specific duplicate. |
| future instructor queue or retention scan by course/status/time | not yet an implemented query contract | Do not pre-add `(tenant_id, course_id, grading_status, evaluated_at)` or a partial index. Measure when the queue projection exists. |

Before accepting a later queue/purge index, use a representative mixed course
and capture the same `EXPLAIN (ANALYZE, BUFFERS)` query before and after.  The
fixed-access mutation does not need a ritual planner assertion, and a planner
text match must not be a permanent test.

## Retention and concurrency oracles

Permanent behavior tests should prove outcomes, not row counts or internal SQL:

- A submitted manual-review response has exactly one current pending
  evaluation with null credit/correct and no current score; no fabricated zero
  result is exposed.
- A mixed run holds its final selection while any active item is pending; after
  a manual grade, the generation-fenced worker publishes a current combined
  projection using automatic and manual credit once each.
- Exact receipt replay returns its recorded outcome without a second
  generation/job; changed actor, target, digest, or expected revision conflicts.
- Two grade attempts with the same expected revision serialize: one wins, the
  other observes a revision/state conflict; no lost update or partial score is
  published.  Interleave a prepared score worker with a manual correction and
  require the stale generation to be superseded.
- Foreign tenant/course, student, and non-instructor callers get the existing
  non-enumerating absence result.  A real-role PostgreSQL disposable test
  confirms forced-RLS/privilege denial.
- Course retention fences new receipt/evaluation writes, deletes receipt before
  its parent evidence, and finds no retained learner record afterward.

One-time PostgreSQL 17 evidence: clean migration/no-op/status/verify; real
RLS/role fixture; deterministic concurrent manual-grade race; and an
`EXPLAIN (ANALYZE, BUFFERS)` comparison only if a new non-PK queue/purge index
is proposed.  Keep course-local item analysis and production worker draining
out of this transaction/package.

## Remaining decisions/risk

- The pending evaluation payload marker must be defined as a non-secret,
  checksum-bearing fixed representation.  Do not add rubric text or manual
  comments without a separately bounded disclosure and retention contract.
- If product direction later requires manual grading of force-submitted,
  response-less work, that is a new policy decision.  It cannot reuse this
  response-evaluation model without deliberately defining what evidence is
  being evaluated.
- A dedicated instructor queue can later justify a course/status/time index,
  but only with its actual ordering, cardinality, and plan evidence.

## Audit execution

- Read repository direction, current checkpoint, database-evolution plan, and
  all seven prior manual-grading audit artifacts.
- Read the PostgreSQL expert workflow, topic index, testing oracle, local-book
  route, and reference survey.
- Read the cited local PostgreSQL 16 documentation and task-book passages.
- `git diff --check` (after this artifact): required final package hygiene.
- Targeted Markdown link validation: run after artifact creation if available.
