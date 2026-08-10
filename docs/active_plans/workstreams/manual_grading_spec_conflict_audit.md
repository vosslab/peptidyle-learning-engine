# Manual grading specification reconciliation

> **Historical audit.** This concluded reconciliation is retained as evidence, not current task
> direction. Current authority is the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

**Status:** ready for implementation. This is an audit artifact only; it makes
no production-code change.

The next package is **manual grading and mixed automatic/manual assignment
behavior**. The accepted database-evolution plan and the checkpoint status
override the older implementation-plan passages where the two describe
incompatible score-history models. The work remains server-authoritative,
tenant-scoped, and question-agnostic.

## Precedence and reconciliation

| Topic                                           | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Decision for this package                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Current package order                           | `docs/active_plans/partial_commit_status.md:100-105` puts manual/mixed grading first and item analysis second. `AGENTS.md:5-10` requires documented dependency order.                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Implement manual/mixed grading now. Do not move course-local item analysis into this package.                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Grade history versus current state              | The older plan requires a new immutable grade event on regrade (`docs/active_plans/implementation_plan.md:667-678`) and includes `grade_event` in its scale table (`docs/active_plans/implementation_plan.md:700-706`). The accepted database decision instead requires current `submission_evaluation`, `attempt_score_current`, and summary rows and explicitly forbids old grade projections/`grade_event` append history (`docs/active_plans/decisions/database_schema_evolution_plan.md:229-256`). The checkpoint says the baseline and current-only scoring are complete (`docs/active_plans/partial_commit_status.md:12-30`). | The database-evolution design wins: replace the current evaluation and recompute current projections. Do not add `grade_event`, a score-revision table, obsolete computed grades, or assignment-version history. The submitted raw response remains immutable protected evidence.                                                                                                                                                                                                                                                                 |
| Manual evaluation versus an assignment override | The database decision calls manual override **optional separate current state** and says rescoring must not silently delete it (`docs/active_plans/decisions/database_schema_evolution_plan.md:229-237`). It separately identifies `submission_evaluation` as the location of current normalized evaluation and manual-grading status.                                                                                                                                                                                                                                                                                               | Treat these as different concepts. A manual item evaluation is required now: an authorized instructor supplies a server-validated current normalized credit result for one submitted response. A final assignment-grade override, if exposed in this package, must be a distinct current-state record with an explicit clear command; it must never masquerade as a submission evaluation. The package should implement the data/Store contract for that separate override if it exposes the feature, rather than overloading computed summaries. |
| Mixed automatic/manual behavior                 | The schema already models `graded`, `needs_manual_grading`, and `exempt` evaluations (`schemas/migrations/2026080804_activity_feedback.sql:173-191`) and matching question-attempt states (`schemas/migrations/2026080804_activity_feedback.sql:209-228`). Current PostgreSQL summary formation already refuses to finalize a run while a non-cleared evaluation is `needs_manual_grading` (`crates/learning-data-access/src/postgres.rs:892-901`).                                                                                                                                                                                  | An assignment may contain any mixture of automatically gradeable and manual-review items. Automatic items are scored immediately by the server. A manually pending item leaves the run/assignment visibly awaiting grading and prevents an automatic final selection until every includable item is resolved, exempt, or cleared. One manual result must trigger the existing generation-fenced recalculation path, never a browser-side total.                                                                                                   |
| Question-agnostic contract                      | The grading layer declares file upload manual review explicitly (`crates/grading/src/checker.rs:39-41`, `crates/grading/src/checker.rs:174-176`), while the design places question-type-specific structures in the private problem payload (`docs/active_plans/decisions/database_schema_evolution_plan.md:89-100`).                                                                                                                                                                                                                                                                                                                 | The manual-grade command accepts generic normalized credit and a bounded, server-owned status/reason contract; it must not make the Store/API depend on a particular question family, rubric schema, or answer-key shape. Backend-specific rubric interpretation stays inside the server-only grading boundary.                                                                                                                                                                                                                                   |
| Answer secrecy and educational-record tenancy   | Human guidance keeps answers, keys, and correctness decisions server-only; student/course records are tenant-owned (`docs/HUMAN_GUIDANCE.md:48-51`). The database plan requires tenant-leading private keys and RLS, with context from the authenticated server session (`docs/active_plans/decisions/database_schema_evolution_plan.md:341-347`).                                                                                                                                                                                                                                                                                   | Manual-grade routes and Store commands derive tenant and instructor authority from the authenticated session; no tenant, enrollment, correct answer, rubric, or grade is trusted from the browser. Evaluation, override, audit, and feedback records remain tenant-owned. Shared published version data is only read by pinned immutable reference.                                                                                                                                                                                               |
| Published content immutability                  | Published corrections make a new version (`docs/active_plans/decisions/database_schema_evolution_plan.md:89-109`); assignment items pin problem/version IDs (`docs/active_plans/decisions/database_schema_evolution_plan.md:168-197`).                                                                                                                                                                                                                                                                                                                                                                                               | Manual grading changes the tenant-owned current evaluation of delivered evidence, never a published problem, version payload, answer key, rubric, or pinned run item. A content correction remains publish-a-new-version or assignment Delete and Regrade.                                                                                                                                                                                                                                                                                        |
| Item analysis                                   | Analysis is a rerunnable, tenant-owned projection that never blocks grading (`docs/active_plans/decisions/database_schema_evolution_plan.md:304-319`), and the checkpoint explicitly schedules it after manual grading (`docs/active_plans/partial_commit_status.md:100-104`).                                                                                                                                                                                                                                                                                                                                                       | Out of scope except for preserving a clean recalculation hook/boundary. Do not add course-local analytics tables, workers, or grade-path waits here. The following package owns recalculation of analysis and its incomplete-manual/recent-rescoring flags.                                                                                                                                                                                                                                                                                       |

## Resolved package scope

The implementation package should add an explicit, idempotent, authorized
manual-grade operation across the domain, Store trait, memory and PostgreSQL
backends, server route, generated browser contract, fixtures, and focused
cross-backend behavior tests.

Its authoritative transition is:

```text
submitted raw response (immutable tenant evidence)
  -> current submission_evaluation: needs_manual_grading
  -> instructor's server-validated current manual evaluation
  -> scoring generation increments and current projections are recalculated
  -> complete mixed run can become eligible for the existing grade policy
```

The operation must use a stable action/idempotency identity, optimistic or
equivalent serialization against concurrent instructor actions, and a minimal
audit record that identifies the protected evidence rather than copying its
response, prior computed value, rubric, or answer key. A retry returns the
same current outcome; a conflicting action cannot silently replace a different
manual grade. The existing retry-safe support-action pattern is the closest
local precedent (`crates/learning-data-access/src/lib.rs:1312-1388`).

The source schema currently has a single current evaluation per tenant/attempt
(`schemas/migrations/2026080804_activity_feedback.sql:173-191`,
`schemas/migrations/2026080804_activity_feedback.sql:320-324`) and one
current attempt-score row (`schemas/migrations/2026080804_activity_feedback.sql:183-197`,
`schemas/migrations/2026080804_activity_feedback.sql:323-324`). Core
manual item grading fits those records and needs no history table. The
checkpoint remains pre-data; if an explicit final-assignment override needs a
new table, amend the appropriate one of the six initial-epoch migrations and
rerun the fresh-baseline evidence. Once durable data exists, that same change
must instead be a forward migration (`docs/active_plans/partial_commit_status.md:66-87`).

The implementation must distinguish:

- **Manual item grade:** replace the current `submission_evaluation` for a
  submitted manual-review item with a normalized credit result and `graded`
  status, then use the current scoring-generation pipeline. This is required.

- **Final assignment override:** an explicit instructor choice that is
  separate current state from the computed student assignment summary. If the
  package exposes it, use a tenant/course/enrollment/assignment-scoped current
  record and an explicit clear action; current rescoring updates computation
  beneath it but leaves the explicit choice intact. It is not a substitute for
  resolving a pending manual item.

## Explicit non-scope

- Do not restore `grade_event`, score revisions, old computed-score retention,
  cloned assignment revisions, or historic grade display.
- Do not change published problem versions, draft/public identity rules,
  catalog visibility, source payloads, answer keys, or grading implementations
  to support a manual grade.
- Do not make manual grading a special case for file uploads, native questions,
  QTI, H5P, or any other backend; the boundary is generic normalized credit.
- Do not add item-analysis computation, course-local analysis display, catalog
  statistics, or analytics-worker waiting. Existing global de-identified
  question-statistics infrastructure is distinct from the next course-local
  projection (`schemas/migrations/2026080805_operations_analytics.sql:710-759`).
- Do not start a partial production queue-drain registry; that remains the
  documented later MOD-WORKER package (`docs/active_plans/partial_commit_status.md:60-64`).

## Acceptance checklist for the implementation owner

- [ ] A server-only, tenant-scoped manual-grade command can grade a submitted
      `needs_manual_grading` response with bounded normalized credit, without
      receiving answer keys or a browser-computed result.
- [ ] The command is authorized for the course instructor, rejects a foreign
      tenant/student/attempt and an inappropriate attempt state, and cannot
      use a caller-selected tenant context.
- [ ] Both MemoryStore and PostgreSQL implement identical state, conflict, and
      exact-retry behavior; a focused conformance test covers them.
- [ ] A manual grade replaces only current evaluation/projection state and
      starts the existing current-generation recalculation; no grade-history
      table, obsolete computed value, or answer-bearing audit payload is
      created.
- [ ] A mixed assignment proves automatic items grade normally, a pending
      manual item prevents premature final grade selection, and resolving it
      produces the correct current score under normal, full-credit,
      extra-credit, and excluded-item semantics where applicable.
- [ ] Recalculation is generation-fenced: a newer scoring change or concurrent
      submission cannot publish a stale/manual result, and instructor retries
      remain idempotent.
- [ ] Any final assignment override is separate, tenant-owned current state;
      recalculation preserves it until an explicit authorized clear. If not
      exposed, no API or UI claim implies final-grade override exists.
- [ ] Student/instructor projections disclose only the permitted current status
      and feedback. They never leak an answer key, private rubric, raw
      response to an unauthorized user, or another tenant's education record.
- [ ] Published problem/version records and delivered run references are
      unchanged by manual grading; the proof includes a pinned-version case.
- [ ] Item-analysis work remains absent from the synchronous grade path; the
      next package receives a clear post-rescoring trigger rather than a
      coupled history model.
- [ ] Run the narrow manual/mixed Store tests first, then all documented
      package gates: formatting, strict Clippy, workspace tests, generated
      TypeScript/type/lint/test gates, PostgreSQL behavior/conformance tests,
      `./check_codebase.sh`, and `pytest tests/` as required by the database
      plan (`docs/active_plans/decisions/database_schema_evolution_plan.md:463-489`).

## One implementation choice made for reversible progress

Use a generic manual-evaluation operation first and keep final grade override a
separate optional current-state capability. This satisfies the explicit mixed
grading requirement without tying the platform to any question type or
reintroducing historical score storage. It also leaves a clean, additive
forward-migration seam if the final-override UI is deliberately scheduled
after the core manual-review flow.
