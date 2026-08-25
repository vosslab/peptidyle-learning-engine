# Manual grading schema implementation

> **Historical workstream record.** This package is retained as implementation evidence, not
> current task direction. Current authority is the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

## Scope and baseline decision

This is the approved pre-data baseline consolidation described by
`partial_commit_status.md`: only the activity/feedback and retention migrations
changed, and no seventh migration was added. The clean apply, no-op, status,
and verify validation remains package acceptance work after the PostgreSQL Store
consumer lands.

## Implemented shape

- `submission_evaluation` now represents the sole current evaluation for an
  attempt. Only `credit_fraction` and `correct` are nullable; its conditional
  check requires both to be null for `needs_manual_grading` and both to be
  present for `graded` or `exempt`. `evaluation_revision` is positive and
  begins at one.
- `manual_grade_receipt` is tenant-owned and keyed exactly by
  `(tenant_id, manual_grade_action_id)`. It retains only action identity,
  request digest, expected/resulting evaluation revisions, scoring generation,
  actor, attempt, occurrence time, and course. It has no grade, response,
  rubric, or evaluation payload columns.
- The receipt has a tenant-leading FK to the current evaluation and a course
  FK. Its revision/generation checks require positive values and exactly one
  evaluation-revision step.
- The receipt has forced RLS, the normal explicit tenant policy, application
  `SELECT, INSERT` only, and retention-broker `SELECT, DELETE` only. No
  student, worker, browser, or statistics grant was added.

## Retention integration

`manual_grade_receipt` is registered in the closed learner-record fence list,
receives the existing attempt-to-course binding trigger and learner-record
fence trigger, is purged before `submission_evaluation`, is checked by the
post-purge residual assertion, and has tenant-scoped retention-broker select
and delete policies. Consequently an archived course rejects new receipts,
and the parent evaluation cannot be purged while a receipt remains.

No manual-specific secondary index was added: the receipt primary key is the
replay access path, and no instructor-queue or course/time query has been
implemented or measured.

## Validation

- `git diff --check` - passed.
- `cargo tools database --help` - blocked before command parsing by the known,
  concurrent `GradeOutcome::NeedsManualGrading` non-exhaustive match in
  `crates/project-tools/src/fixtures.rs:454`.
- `./check_codebase.sh` - likewise stopped at its `tsgen` step on that same
  known project-tools compile error; it did not reach a SQL migration execution path.

There is no local `psql`, SQL parser, or SQL linter available for an offline
syntax oracle. The remaining syntax/privilege risk must therefore be resolved
by the planned disposable PostgreSQL 17 clean-apply/no-op/status/verify and
real-role retention/RLS fixture. The supplied SQLx type reference confirms
that the subsequent Store implementation should bind `rust_decimal::Decimal`
to PostgreSQL `NUMERIC` using SQLx's `rust_decimal` feature; this schema keeps
the existing `numeric(16,12)` representation.
