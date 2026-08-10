# Manual grading Store contract audit

Status: historical audit concluded 2026-08-08. This document records the
Store-contract analysis used by the former manual-grading package. It is not an
active implementation directive and does not claim completion of any code or
test not recorded by its owning implementation evidence.

## Historical conclusion

At the time of this audit, the inspected Store boundary had a useful pending
state but no instructor-authorized, revisioned manual-grade mutation. The audit
therefore defined a current-state, tenant-scoped contract for manual item
evaluation and mixed automatic/manual scoring.

Subsequent historical implementation and review evidence lives in:

- [manual_grading_core_implementation.md](manual_grading_core_implementation.md)
- [manual_grading_core_review.md](manual_grading_core_review.md)
- [manual_grading_postgres_implementation.md](manual_grading_postgres_implementation.md)
- [manual_grading_schema_implementation.md](manual_grading_schema_implementation.md)

Those records, where applicable, supersede this audit's proposed types and
forward-schema sketch. The current release authority is
[release_completion_plan.md](../active/release_completion_plan.md), not this
historical audit.

## Evidence retained from the audit

| Area                     | Historical observation                                                                                    | Contract consequence identified                                                                                 |
| ------------------------ | --------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Attempt state            | `AttemptStatus::NeedsManualGrading` could represent a submitted or force-submitted pending attempt.       | Pending presentation did not itself authorize a grade write or provide a completed-decision record.             |
| Current projections      | `submission_evaluation` and `attempt_score_current` were current-only rows.                               | A manual correction belonged in mutable current state, not in a score-history table.                            |
| Submission path          | Ordinary submission produced a graded result and score; scoring held back a run with pending manual work. | Mixed runs needed one effective final decision per includable item before a completed projection could publish. |
| Memory/PostgreSQL parity | The MemoryStore's historical eligibility model differed from PostgreSQL's current-evaluation model.       | Both backends required equivalent observable pending, revision, replay, and generation behavior.                |
| Security boundary        | Store support actions already derived tenant and course access and avoided foreign-record enumeration.    | A manual command required server-derived authority, exact idempotency, and non-enumerating denial behavior.     |

## Historical contract requirements

The audit required an instructor-only Store operation with these properties:

- A stable action identity, request fingerprint, actor, attempt identity, and
  expected positive evaluation revision.
- Exact action replay returning the prior result; action reuse with changed
  target, revision, actor, or decision producing a conflict.
- A current manual result that replaced the current manual evaluation without
  changing immutable response, attempt provenance, or submission timing.
- Server-derived tenant and direct-course-instructor authority, with foreign
  tenant or unauthorized access indistinguishable from absence where the Store
  already used that boundary.
- A single transaction/lock boundary that updated current evaluation state,
  advanced scoring generation, marked recalculation, emitted one fenced job,
  and wrote minimal audit evidence without grade values or response bytes.
- Current-score publication only through the established generation-fenced
  worker path. A pending manual item left the run incomplete; a resolved item
  participated with automatic items in the resulting current score.
- Browser-safe projections only. Answer keys, rubrics, raw grading details,
  Store commands, and manual authority stayed outside the learner/Wasm surface.

The audit also separated a manual item evaluation from a final assignment
override. It found that treating an item decision as an assignment override
would obscure mixed-item completeness and blur current computed/override
ownership.

## Historical test evidence sought

The former package's behavior matrix covered:

- Manual-only and mixed two-item runs, including pending presentation before
  publication and exact current values after a generation-fenced commit.
- Correction, removal/return-to-pending, stale revision, stale worker, exact
  replay, and conflicting action-ID reuse.
- Instructor, student, wrong-course, and foreign-tenant denial behavior;
  course archive and retention cleanup; PostgreSQL forced-RLS acceptance.
- Browser/Wasm boundary checks showing that no answer key, rubric, grading
  implementation, or learner-usable manual mutation crossed the public model.

The audit was a specification of evidence, not proof that every case had run.
Implementation and review workstreams are the only sources for recorded gate
results.

## Resolved and out-of-v1 mapping

| Historical concern                                                                   | Current disposition                                                                                                                                                                       |
| ------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposed Rust types and `ManualGradingStore` API                                     | Superseded as an audit proposal by the historical implementation artifacts listed above. No type change is directed by this document.                                                     |
| Forward migration proposal                                                           | Superseded by the historical schema implementation/review records and governed by [database_schema_evolution_plan.md](../decisions/database_schema_evolution_plan.md).                    |
| Manual item grading and mixed-run behavior                                           | No current WP-RC package owns new manual-grading delivery. It is outside the current version 1 release route unless the release decision ledger is amended before implementation.         |
| Instructor review DTO, route, and UI                                                 | No current WP-RC package assigns this feature. It is post-v1 product scope, not an open dependency for the active release plan.                                                           |
| Assignment-summary override                                                          | The audit identified it as a separate product policy. No current release package assigns it; it is an explicit post-v1 decision.                                                          |
| Course-local item analysis                                                           | The historical audit kept it separate from scoring. It is not assigned to the current WP-RC sequence and is outside the version 1 route.                                                  |
| Server-only grading, answer secrecy, current score summaries, and generation fencing | These remain active architecture constraints under [implementation_plan.md](../implementation_plan.md) and the release plan's grading packages; this audit introduces no additional work. |

## Preserved limitations

- A response-less force-submit must not acquire fabricated submission evidence.
- A manual mutation requires an authenticated server authority check; database
  role grants alone are insufficient.
- Historical code/test observations in this audit describe the inspected state
  on 2026-08-08 and are not a statement about the present working tree.

Artifact: `docs/active_plans/workstreams/manual_grading_store_contract_audit.md`
