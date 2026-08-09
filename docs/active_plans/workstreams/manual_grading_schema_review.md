# Manual grading schema review

## Verdict

**PASS for the pre-data baseline schema slice.** The two changed migrations
implement the frozen one-current-evaluation model and extend every required
retention/RLS surface. No production or schema file was changed during this
review.

## Evidence

- `submission_evaluation` permits null `credit_fraction` and `correct` only
  for `needs_manual_grading`; `graded` and `exempt` require both fields. Its
  credit bounds remain intact, and `evaluation_revision` defaults to one and
  must be positive. See
  [activity migration](../../../schemas/migrations/2026080804_activity_feedback.sql#L166-L184).
- `manual_grade_receipt` retains only tenant/action identity, attempt, actor,
  request digest, expected/resulting revisions, scoring generation, time, and
  course. It has no credit, response, rubric, result, or payload field. The
  revision/generation checks and the tenant-leading composite primary key are
  present. See
  [activity migration](../../../schemas/migrations/2026080804_activity_feedback.sql#L186-L203)
  and [its key](../../../schemas/migrations/2026080804_activity_feedback.sql#L345-L346).
- The receipt is constrained to the tenant-scoped current evaluation and
  course. Both the evaluation and receipt have forced RLS, explicit tenant
  policies, and least-privilege application/retention grants. See
  [foreign keys](../../../schemas/migrations/2026080804_activity_feedback.sql#L463-L470),
  [RLS](../../../schemas/migrations/2026080804_activity_feedback.sql#L536-L542),
  and [grants](../../../schemas/migrations/2026080804_activity_feedback.sql#L591-L595).
- Retention includes the receipt in the closed learner-record fence list,
  binds course ownership from its attempt, fences writes, purges it before its
  evaluation parent, checks it in the residual assertion, and gives the
  retention broker tenant-scoped select/delete policies. See
  [fence list](../../../schemas/migrations/2026080806_retention.sql#L440-L452),
  [purge order](../../../schemas/migrations/2026080806_retention.sql#L1324-L1338),
  [residual assertion](../../../schemas/migrations/2026080806_retention.sql#L1568-L1582),
  [triggers](../../../schemas/migrations/2026080806_retention.sql#L2637-L2643),
  and [broker policies](../../../schemas/migrations/2026080806_retention.sql#L2849-L2855).
- No manual-grade secondary index was added. The composite receipt primary key
  is sufficient for the implemented replay lookup; no queue/list workload has
  been measured to justify another index.

## Validation and limit

- `git diff --check` passed.
- Static inspection found balanced table, constraint, policy, trigger, and
  grant statements consistent with adjacent migration conventions.
- No local `psql`, PostgreSQL server, or SQL linter is available. The required
  PostgreSQL 17 disposable clean-apply/no-op/status/verify and real-role RLS/
  retention fixture remain the executable syntax and privilege oracle after
  the Postgres Store consumer lands. The known concurrent project-tools non-exhaustive
  `GradeOutcome::NeedsManualGrading` match prevents the broader repository
  gate before it reaches migration execution.

## Scope confirmation

The status document still declares this a pre-data baseline, so direct edits to
the six-file epoch are authorized. The next schema change after durable data is
accepted must be a forward migration.
