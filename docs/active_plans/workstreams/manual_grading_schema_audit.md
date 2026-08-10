# Manual grading schema audit

Status: historical audit concluded 2026-08-08. This document preserves the
schema evidence and risks considered for the former manual-grading package. It
is not an active implementation plan and authorizes no migration edit.

## Historical conclusion

The audit established that mixed automatic/manual scoring required one mutable,
tenant-owned current decision per attempt while preserving immutable submission
evidence and current-score projections. It rejected an append-only grade-history
design for that package.

The former pre-data baseline decision was resolved by the recorded implementation
and review artifacts:

- [manual_grading_schema_implementation.md](manual_grading_schema_implementation.md)
  records the selected `submission_evaluation` and `manual_grade_receipt` shape.
- [manual_grading_schema_review.md](manual_grading_schema_review.md) records the
  schema-slice review evidence and its limits.
- The accepted database-evolution policy now governs any schema work through
  [database_schema_evolution_plan.md](../decisions/database_schema_evolution_plan.md).

The former six-file consolidation decision is concluded historical context, not
a standing instruction. Any future durable-schema change follows the
forward-only policy in the database-evolution plan.

## Historical evidence and findings

| Topic                        | Evidence observed during the audit                                                               | Finding retained from the audit                                                                                               |
| ---------------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| Pending manual work          | `submission_evaluation` and `question_attempt` contained `needs_manual_grading` state.           | A pending state alone did not provide accountable manual decision identity, replay protection, or revision conflict handling. |
| Current scoring              | PostgreSQL scoring accepted `graded` evaluations and held a run pending for a manual evaluation. | Mixed assignments required an effective current decision without replacing immutable submission evidence.                     |
| Current state versus history | One current evaluation and one current score row already existed per attempt.                    | The proposed design kept one current manual decision and rejected an old-score or grade-event history.                        |
| Sensitive action retries     | Support actions used stable action identity and minimal audit evidence.                          | Manual actions needed the same identity/fingerprint discipline to distinguish exact replay from a conflicting reuse.          |
| Tenant and role boundary     | Operational tables used forced RLS, tenant policies, and limited application grants.             | Any learner record needed equivalent default-deny RLS, course binding, and retention treatment.                               |
| Retention lifecycle          | Retention enumerated, fenced, deleted, and checked learner activity records.                     | A manual record omitted from any lifecycle registration could leave protected learner data behind.                            |
| Analytics boundary           | Analysis was designed as derived, course-local, and non-blocking.                                | A manual decision must not put learner text or scores into jobs or overload global statistics.                                |
| Job payload boundary         | Existing recalculation jobs were generation-fenced and identity-only.                            | A manual update required the existing assignment/generation shape, not response or feedback content in a payload.             |

## Historical contract findings

The audit's bounded contract was:

- Preserve `submission_evaluation` as current automatic evidence and retain at
  most one current human decision for an attempt.
- Bind the decision to server-derived tenant and course identity, an accountable
  actor and timestamp, a positive revision, and a stable action identity with a
  request fingerprint.
- Store only bounded structured feedback operationally; keep long commentary,
  media, and annotated work out of hot-path JSON.
- Use forced RLS, tenant-leading keys, narrowly scoped application and retention
  grants, course-binding protection, retention fencing, purge ordering, and
  residual checks.
- Treat recalculation as a generation-fenced current-projection operation. A
  pending manual item held its run incomplete; a completed manual decision
  supplied current scoring evidence alongside automatic items.

These findings are historical design evidence. The implementation artifacts
named above, rather than this audit, describe the selected schema shape.

## Historical validation evidence

The audit identified the following as the meaningful evidence for that former
package:

- Fresh PostgreSQL migration apply, second no-op apply, ledger verification,
  and real-role RLS/grant/trigger inspection.
- Automatic-only and mixed automatic/manual scoring fixtures, including partial
  and zero credit, exact replay, conflicting action reuse, stale revision, and
  generation supersession behavior.
- Retention archive/delete and post-archive fence checks covering every manual
  learner record.
- Representative `EXPLAIN (ANALYZE, BUFFERS)` evidence only when a newly added
  gradebook or purge index changed a measured query path.

The audit did not itself execute those package gates and must not be read as
evidence that an unrecorded code path passed them.

## Release mapping

| Historical requirement                                                 | Current disposition                                                                                                                                                                                                                                                   |
| ---------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Server-authoritative automatic grading and current summary projections | Covered by the accepted platform architecture and by the grading behavior in the current release packages, especially [release_completion_plan.md](../active/release_completion_plan.md)'s WP-RC3 and WP-RC5 grading paths. This audit adds no separate release work. |
| Manual item decision, instructor workflow, or manual-review UI         | No WP-RC package owns a new manual-grading feature. It is outside the current version 1 release route unless the release decision ledger is amended before implementation.                                                                                            |
| Assignment-level manual override                                       | Explicitly separate from a manual item evaluation in the historical audit. No current release package assigns it; it is a post-v1 product decision rather than an open task here.                                                                                     |
| Course-local item analysis after rescoring                             | Not a current WP-RC package. It remains outside the version 1 route and must not be inferred from this audit.                                                                                                                                                         |
| Baseline consolidation and migration choice                            | Already-resolved historical decision; the database-evolution plan is the authoritative policy.                                                                                                                                                                        |

## Preserved risks

- A force-submitted attempt could lack a response/evaluation. A manual workflow
  must not fabricate response evidence or an automatic result for that state.
- SQL roles alone do not establish instructor authority; the authenticated
  server boundary remains responsible for course authorization.
- Unbounded operational JSON remains a general hardening concern. The audit's
  bounded-feedback finding does not resolve unrelated existing payloads.

Artifact: `docs/active_plans/workstreams/manual_grading_schema_audit.md`
