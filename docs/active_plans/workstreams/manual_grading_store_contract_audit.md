# Manual grading Store contract audit

## Scope and conclusion

This audit covers the first remaining database-evolution package: manual
grading and assignments that contain both automatically graded and
manually graded questions. It does not change production code.

The package needs a new Store contract and a forward migration. The current
implementation has a useful pending state, but not a manual-grade mutation:

- `AttemptStatus::NeedsManualGrading` exists, and an authorized
  force-submit can put an attempt in that state without fabricating a
  response, evaluation, or score. [question_model activity:248-279;
  store lib:3336-3345; partial status:31-42]
- `submission_evaluation` has a current-only status column and
  `attempt_score_current` has one current row per attempt, but neither has a
  revision token nor an API to write, revise, or remove a manual grade.
  [activity migration:166-195, 320-324]
- Ordinary submission always writes `graded` and an attempt score. [postgres:
  8817-8867; memory:6439-6625]
- Rescoring stages only `graded` evaluations and holds back a run that has a
  `needs_manual_grading` evaluation. [postgres:814-902] MemoryStore has no
  independent current-evaluation map at all; it derives eligibility from a
  submitted `QuestionAttempt.result`. [memory:194-235, 735-759, 663-845]

Therefore no existing method can implement instructor-authorized manual
grade entry, revision, or removal without bypassing the Store's tenant,
authorization, idempotency, and generation boundaries.

## Required contract

Keep the distinction between immutable evidence and mutable current grading:

| Owner | Current responsibility |
| --- | --- |
| `submission` and `question_attempt` | Protected raw response when one exists; issued identity, timing, and provenance. Never overwrite these for manual grading. |
| `submission_evaluation` | One current normalized evaluation and its grading state for an attempt. A manual edit replaces this current value; it does not append a score history. |
| `attempt_score_current` and `student_assignment_summary` | Rebuildable current projections, published only by the existing scoring-generation worker. |
| `audit_event` | Minimal, idempotent evidence that an authorized person performed a manual-grade action. It must not retain previous/current numeric grades or response bytes. |

The public `QuestionAttempt` remains browser-safe and must not receive rubric
text, answer keys, or grading implementation details. `AttemptResult` already
contains only a normalized result and has the server-only boundary documented
in its model. [question_model activity:266-279, 321-353; grading key:1-43]

### Proposed Rust types and Store methods

Add the following types in `crates/learning-data-access/src/lib.rs`; they are Store command
types, not `question_model` types. The target is the native `store` library
for its existing native targets; this is a `Result`/ownership API change, not
a Wasm surface.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManualGradeActionId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvaluationRevision(u64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ManualGrade {
    pub result: AttemptResult,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ManualGradeRecord {
    pub tenant: TenantId,
    pub attempt: QuestionAttemptId,
    pub revision: EvaluationRevision,
    pub result: Option<AttemptResult>,
    pub grading_status: ManualGradingStatus,
    pub occurred_at: ActivityTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualGradingStatus {
    NeedsManualGrading,
    Graded,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ManualGradeChange {
    Set(ManualGrade),
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChangeManualGradeCommand {
    pub action: ManualGradeActionId,
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub expected_revision: EvaluationRevision,
    pub change: ManualGradeChange,
}

#[async_trait]
pub trait ManualGradingStore: Send + Sync {
    async fn get_manual_grade_for_edit(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ManualGradeRecord>, StoreError>;

    async fn change_manual_grade(
        &self,
        context: TenantContext,
        command: ChangeManualGradeCommand,
    ) -> Result<ManualGradeRecord, StoreError>;
}
```

`ManualGradeActionId` follows `AttemptSupportActionId`, including a trusted
generator, UUID accessor, and exact-action idempotency behavior. [store
lib:1312-1387; store conformance:4193-4270] `EvaluationRevision` is scoped to
the evaluation row, not `AssignmentRevision`: changing one learner's grade
must not invalidate an instructor editing assignment timing or point values.

`expected_revision` is required for *both* `Set` and `Remove`. It avoids the
otherwise silent last-writer-wins failure when two authorized graders review
the same submission. The returned record owns its values; callers neither
borrow state across an `async` boundary nor receive a mutable Store view.

### Mutation semantics

1. Load the attempt, run, enrollment, assignment, and current course role in
   the same transaction/MemoryStore lock. Require the active `TenantContext`,
   an accessible course, and a persisted direct instructor role; a foreign
   tenant must remain indistinguishable from absence, matching support actions.
   [memory:7210-7246; postgres:7880-7921]
2. Permit only a terminal, non-cleared/non-exempt attempt that is either
   `NeedsManualGrading` or has an existing manually editable evaluation. Do
   not manufacture a `submission` for a force-submitted attempt; the status
   checkpoint explicitly requires zero submission/evaluation/score rows for
   that case before manual grading. [partial status:39-42, 81-83]
3. `Set` validates `AttemptResult` with the existing normalized-credit
   validation, changes only the current evaluation to `graded`, increments
   `EvaluationRevision`, records one minimal audit receipt, increments the
   assignment `scoring_generation`, marks it `Recalculating`, and enqueues one
   tenant-scoped recalculation job in the same transaction.
4. `Remove` validates the same revision, removes the current manual result
   while leaving the raw response, attempt provenance, and submitted time
   untouched. It does not restore a superseded automatic score, because that
   would require a prohibited scoring-history record. Its current evaluation
   becomes `needs_manual_grading`, it increments the evaluation revision, and
   uses the same generation/job transaction. Repeated exact action returns the
   stored receipt; reuse of an action ID for a different attempt, revision, or
   change is `Conflict`.
5. A worker stages only `graded` current evaluations. A completed run with
   any active `needs_manual_grading` item receives no current run/assignment
   score; mixed assignments publish exactly when their last pending manual
   item becomes `graded`. Existing atomic staging, lease, and generation
   checks remain the only publication path. [store lib:226-259; postgres:
   814-902, 1050-1165; memory:663-845]

This contract deliberately does **not** add an assignment-summary override.
The schema plan says that an optional manual override is a separate current
state from computed values. [database plan:229-237] That is a distinct
gradebook policy feature; treating a manual item evaluation as an assignment
override would make mixed-question completeness unobservable and would blur
the current computed/override boundary.

## Required forward schema change

The accepted six-file baseline has already been exercised. Do not edit it;
add a new forward SQLx migration. [partial status:85-87]

The migration must make a current pending/manual evaluation representable for
both response-bearing and force-submitted attempts:

- add `evaluation_revision bigint NOT NULL DEFAULT 1 CHECK
  (evaluation_revision > 0)` to `submission_evaluation`;
- permit a current `needs_manual_grading` evaluation to have no response
  submission and no normalized result. The present `submission_id`,
  `credit_fraction`, `correct`, `payload`, and `payload_sha256` are all NOT
  NULL, so it cannot model the documented force-submit path. [activity
  migration:166-178; partial status:39-42]
- replace the permissive status check with a shape check: `graded` requires a
  complete normalized result; `needs_manual_grading` has no current numeric
  result; an optional submission link is allowed only when real raw evidence
  exists. Preserve the existing bounded `credit_fraction` check when present.
- retain exactly one `(tenant_id, attempt_id)` evaluation row and add a
  current-write audit receipt keyed by `(tenant_id, manual_grade_action_id)`.
  The receipt stores actor, attempt, action kind, expected/resulting revision,
  and time; it stores no grade values, response JSON, rubric, or old grade.
  It is the idempotency record, not a grade history table.
- give the receipt `course_id`, forced RLS, app/retention grants, the existing
  course-binding and retention-fence triggers, purge ordering, residual
  assertions, and tenant policy. Retention already fences and purges the
  evaluation/current-score tables; the new receipt must join that same
  security class. [retention migration:431-489, 1323-1329, 1564-1573,
  2625-2634, 2833-2842]

Do not introduce `grade_event`, `scoring_revision`, append-only evaluation
versions, or an old-score table. The plan explicitly rejects those histories.
[database plan:229-237, 479-480]

## MemoryStore parity changes

Add current-only structures rather than encoding manual revisions into a
`QuestionAttempt` copy:

```rust
manual_evaluations: BTreeMap<(TenantId, QuestionAttemptId), MemoryEvaluation>,
manual_grade_actions: BTreeMap<(TenantId, ManualGradeActionId), ManualGradeReceipt>,
```

`MemoryEvaluation` owns `Option<AttemptResult>`, status, revision, and current
timestamp. `projected_attempt` should project only browser-safe current result
and its pending/graded presentation from this record. The existing separate
`attempt_current` projection is already the correct pattern. [memory:
223-235, 314-341, 7335-7355]

Both the submission path and external-tool completion path must initialize
the same current-evaluation representation. `build_memory_assignment_scoring`
must iterate evaluations, not just `state.submissions`, so a force-submitted
manual grade can score while raw evidence remains absent. It must also model
the PostgreSQL complete-run holdback; the current MemoryStore skips missing
results and can otherwise score a partially manual run differently.
[memory:735-759, 801-845; postgres:873-902]

Retention cleanup must remove both maps and receipts with the other
course-scoped learner records. Any test-only inspection helper must remain
feature-gated, as the current memory backend does for legacy state helpers.

## Behavior-focused conformance tests

Add one shared `exercise_manual_grading_store` to
`crates/learning-data-access/tests/conformance.rs`, called for both MemoryStore and
PostgresStore. The existing conformance fixture and force-submit assertions
provide the nearest reusable setup. [store conformance:859-1690,
4185-4520]

Required permanent cases:

1. An instructor force-submits an active attempt. Before grading it has no
   response/evaluation/current-score. The instructor sets a valid manual
   grade; raw response remains absent, the current evaluation is graded, and
   the scoring worker produces exactly one current score.
2. A mixed two-item completed run has one automatic grade and one pending
   manual grade. It has no published current assignment score while pending;
   after the manual item is set and the rescoring job commits, its numerator
   and denominator contain both items exactly once.
3. A manual grade revision with the current `EvaluationRevision` changes the
   current score only after generation-fenced staging commits. An old revision
   returns `Conflict` and leaves the grade, generation, and queue unchanged.
4. Exact replay of a `ManualGradeActionId` returns the original receipt and
   creates no second job. Reusing it for another grade, expected revision, or
   attempt returns `Conflict`.
5. Removing a manual grade returns the evaluation to pending, hides current
   gradebook score after the new generation starts, and preserves the raw
   submitted response where one existed. It creates no score/evaluation
   history.
6. Student, staff, and instructor from another course/tenant cannot read or
   mutate the target; PostgreSQL acceptance verifies the same through forced
   RLS. Course archive rejects the mutation, and retention purge removes the
   evaluation action receipt with the course records.
7. A stale scoring worker that prepared before a manual change cannot publish;
   it is superseded or conflicts, then a re-stage of the current generation
   produces the correct mixed score. This extends the existing concurrent
   submission characterization rather than a sleep-based race test.
8. Manual pending/graded state never exposes `grading::AnswerKey`, rubric
   text, or grading implementation across browser-model generation or Wasm
   closure checks.

One-time PostgreSQL acceptance additionally inspects schema shape, RLS/grants,
trigger/purge coverage, and verifies there are no history tables. Keep those
as migration/package probes rather than brittle permanent test assertions.

## Compatibility consumers and sequencing

1. `crates/question_model`: expose only a browser-safe current manual-grading
   state if the UI needs to distinguish pending from graded. Do not expose the
   Store command or a rubric.
2. `crates/learning-data-access`: define the contract, forward migration, MemoryStore and
   PostgresStore implementations, current score staging, retention cleanup,
   conformance fixture, and feature-gated PostgreSQL acceptance.
3. `crates/server`: make instructor routes obtain the current record then send
   `ChangeManualGradeCommand`; the server creates action IDs and never accepts
   a browser-supplied actor/tenant. Keep the student response endpoint on its
   existing server grading path.
4. Browser/TypeScript contracts: add instructor-visible pending/completeness
   state only after the server contract exists. Student gradebook UI must show
   recalculating/pending rather than stale aggregate grades.
5. The next package, course-local item analysis, consumes the resulting
   current evaluation/status and scoring completion signal; it must not be
   started until this package establishes those semantics. [partial status:
   100-104; database plan:304-319]

## Assumptions and risks to resolve in implementation

- The plan requires a separate optional manual **assignment** override but
  does not yet specify its target, visibility, revision, or selection-policy
  semantics. This audit intentionally scopes the immediate package to manual
  **item evaluation**. Record any product decision for summary overrides
  separately before adding one.
- A manually graded force-submit has no student submission by design. The
  forward migration therefore cannot keep `submission_evaluation.submission_id`
  mandatory without adding a fabricated record, which is prohibited.
- `AttemptStatus::NeedsManualGrading` currently says the response "awaits" a
  grade. Do not overload it as the sole completed-manual signal. The separate
  current evaluation status is the authoritative grading completeness source;
  decide whether the browser-safe attempt projection needs a new explicit
  evaluation-state field during server/UI implementation.
- The post-baseline migration is safe only because the status document says
  the epoch is pre-data. Once applied to any real data environment, this
  package's migration must be treated as immutable too.

## Validation route

Narrow iteration: `cargo test -p learning-data-access --test conformance manual_grading`
and the exact PostgreSQL Store acceptance fixture. Completion gate: focused
PostgreSQL behavior tests, `cargo fmt --check`, strict workspace Clippy,
workspace tests/doctests, TypeScript generation/checks if public projections
change, `./check_codebase.sh`, and `pytest tests/`, matching the plan's
required package gate. [database plan:467-490]
