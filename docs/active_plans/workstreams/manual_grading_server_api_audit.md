# Manual grading server/API audit

## Status

**Ready for implementation.** The current schema already distinguishes a pending manual evaluation
from a graded evaluation and the scoring worker excludes pending manual work from a completed run's
current result. The missing seam is an instructor-authorized, revision-checked Store operation and
the HTTP projection that invokes it. No production source was changed for this audit.

## Scope and evidence

The work package is "Add manual grading and mixed automatic/manual assignment behavior"
([partial_commit_status.md](../partial_commit_status.md):102). The database plan requires current
`submission_evaluation` records, no score history, a retryable generation-fenced recalculation, and
verification of mixed automatic/manual assignments
([database_schema_evolution_plan.md](../decisions/database_schema_evolution_plan.md):221-258,
479-483).

Existing foundations are real and should be extended rather than duplicated:

- `submission_evaluation` has one row per tenant/attempt, bounded numeric credit, and the closed
  states `graded`, `needs_manual_grading`, and `exempt`
  ([2026080804_activity_feedback.sql](../../../schemas/migrations/2026080804_activity_feedback.sql):166-181,
  320-324). It is tenant RLS-protected and the application role already has its current-state
  mutation grant (lines 505-507 and 556-557).
- A submitted manual item prevents the scoring worker from publishing that run as a current result;
  graded automatic items remain eligible for its staging query
  ([postgres.rs](../../../crates/learning-data-access/src/postgres.rs):815-902). This is the intended mixed
  assignment behavior, but there is no transition from pending to `graded`.
- `AttemptStatus::NeedsManualGrading` is a public attempt state
  ([activity.rs](../../../crates/question_model/src/activity.rs):251-262). Force-submit can create
  that state with no response or evaluation, deliberately without fabricating either
  ([lib.rs](../../../crates/learning-data-access/src/lib.rs):3336-3345;
  [conformance.rs](../../../crates/learning-data-access/tests/conformance.rs):4296-4304). Such a support-created
  record is not gradeable; clear remains the appropriate recovery action.
- Student submission is server-authenticated, checks an idempotency replay before touching the
  backend, validates the format, and commits only a server-produced result
  ([run.rs](../../../crates/server/src/run.rs):1454-1564). Its key parser is the established strict
  request-header seam (lines 2217-2224).
- Current routes intentionally authorize student-owned run operations with `owned_run`, while
  instructor reads use course membership only where explicitly requested
  ([run.rs](../../../crates/server/src/run.rs):1906-2004;
  [course.rs](../../../crates/server/src/course.rs):470-509). A coarse session `Instructor` role is
  not course authority ([auth.rs](../../../crates/server/src/auth.rs):148-158).
- Assignment updates demonstrate the local ETag pattern: a single quoted `If-Match` revision is
  required, malformed input is rejected, and a success returns the new ETag
  ([course.rs](../../../crates/server/src/course.rs):385-467, 565-607).
- All current run/auth errors go through `no_store`
  ([run.rs](../../../crates/server/src/run.rs):2246-2269;
  [auth.rs](../../../crates/server/src/auth.rs):439-456). The production composition merges only
  the existing run router ([composition.rs](../../../crates/server/src/composition.rs):539-605), so
  the route is not accidentally already mounted.

## Minimal HTTP contract

Add these two instructor-only routes to `crates/server/src/run.rs`, under its existing 64 KiB JSON
limit and route state. They operate on an attempt's **current evaluation**, never an immutable
problem version or an assignment revision.

| Method and path | Request | Success response | Purpose |
| --- | --- | --- | --- |
| `GET /api/attempts/{attemptId}/manual-grade` | no body | `200` `ManualGradeView` plus `ETag: "<evaluationRevision>"` | Loads the student evidence and the small current evaluation projection needed to grade it. |
| `PUT /api/attempts/{attemptId}/manual-grade` | `If-Match: "<evaluationRevision>"`; `Idempotency-Key`; JSON `{"creditFraction":"0.750000000000"}` | `200` `ManualGradeResult` plus the incremented evaluation ETag | Replaces only the current pending manual evaluation with the instructor's normalized credit and schedules current-score recalculation. |

Both responses, including every error, carry `Cache-Control: no-store`. The route accepts one
`Idempotency-Key` header using the existing bounded parser rules; it must not put an action UUID in
the URL or accept a browser-supplied tenant, actor, course, assignment, score, status, or answer key.

`creditFraction` is a canonical decimal string, not a JSON float. Accept the canonical
`-1000` through `1000` range with at most 12 fractional digits, normalize `-0` to `0`, and reject
exponents, whitespace, noncanonical leading zeroes, duplicate fields, and unknown fields. This
matches the `NUMERIC(16,12)`/credit check already declared in the schema (migration lines 166-179)
and avoids reintroducing a binary-float grade at this public mutation boundary. `correct` remains a
server-derived compatibility field (`creditFraction == "1"`); it is not an independently supplied
claim.

`ManualGradeView` should be deliberately small:

```json
{
  "attempt": {
    "id": "uuid",
    "status": "needs_manual_grading",
    "submittedAt": "timestamp",
    "response": { "...student response...": "..." }
  },
  "evaluation": {
    "status": "needs_manual_grading",
    "creditFraction": null,
    "revision": 1
  },
  "currentResult": { "state": "awaitingManualGrade" }
}
```

After the `PUT`, return the same view shape with `attempt.status: "submitted"`,
`evaluation.status: "graded"`, the canonical credit string, the new revision, and
`currentResult: {"state":"recalculating","generation":N}`. Do not return computed points from
the mutation: the durable score worker must be allowed to atomically replace all current attempt and
student summaries for generation `N`. A later gradebook/summary read is the authoritative current
score projection. The endpoint does not add narrative comments, a rubric, or a free-text grader
note; those require a separately bounded, retention-owned design.

## Authorization, non-enumeration, and state transitions

Resolve the session first. Then make one Store-owned authorization-and-load call that joins
attempt -> run -> enrollment -> assignment -> course while scoped to `TenantContext`; it must accept
only a direct course instructor or tenant administrator. Do **not** reuse `owned_run`, because it
correctly restricts student run actions to the enrollment owner. Do **not** authorize from
`SessionSubject.roles()` alone: that is coarse identity, whereas direct course membership is the
record authority.

For a missing, foreign-tenant, foreign-course, or non-instructor target, return the same
`404 {"error":"attempt not found"}` before exposing response, manual-pending status, or evaluation
revision. This follows the existing foreign enrollment behavior (run.rs:1906-2004) and direct
support-action conformance assertion (conformance.rs:4210-4224). A known student member of the same
course may receive `403`, but the route should prefer the stronger uniform 404 for all non-manager
attempt reads and writes; the UI already knows it is a restricted instructor surface.

The Store operation must be one short transaction:

1. Set the database tenant context and select the target evidence/evaluation row `FOR UPDATE`.
2. Derive direct-instructor-or-administrator authority from persisted course membership in that
   transaction; do not trust a prior route check as the durable authority.
3. Verify that a real submitted response and current evaluation both exist, evaluation status is
   `needs_manual_grading`, attempt status is `needs_manual_grading`, and the supplied evaluation
   revision matches. A force-submitted no-response attempt fails `409`, never manufactures a
   submission or evaluation.
4. First look up `(tenant_id, attempt_id, idempotency_key)`. An exact request fingerprint replays
   the stored result even if the caller's ETag is now old; a changed body, actor, or target under
   the same key is `409` and performs no write. Only a non-replay request evaluates `If-Match`.
5. Update the one current evaluation to `graded`, store the exact numeric credit and derived
   correctness, increment its revision, set the attempt to `submitted`, append only minimal audit
   evidence (`attempt.manual_grade`, actor, target, key hash/fingerprint, timestamp), increment the
   assignment scoring generation, mark it `recalculating`, and enqueue exactly one tenant-scoped
   recalculation job. Commit all of this atomically.

The existing support-action code is the appropriate design precedent for atomic authority,
idempotency, audit, and no fabricated evidence ([postgres.rs](../../../crates/learning-data-access/src/postgres.rs):7860-7970;
[memory.rs](../../../crates/learning-data-access/src/in_memory.rs):7190-7310), but the manual-grade command should be
separate: it owns an evaluation revision and an immutable receipt fingerprint, not an action that
closes or clears an attempt.

### Status and error matrix

| Situation | GET | PUT | Required result |
| --- | --- | --- | --- |
| No valid session | `401` | `401` | Generic authentication error, `no-store`. |
| Foreign tenant/course, outsider, or non-manager | `404` | `404` | `attempt not found`; never expose existence. |
| Authorized course instructor but unknown attempt | `404` | `404` | Same error shape. |
| Attempt has submitted response and pending evaluation | `200` + ETag | eligible | The only mutable manual-grade state. |
| Force-submitted with no response/evaluation | `200` support view may show pending state, but omit a gradeable evaluation ETag | `409` | `manual grade requires a submitted response`; use clear/support recovery, never fabricate evidence. |
| Already automatic/previously manually graded, cleared, exempt, or auto-submitted without pending evaluation | `200` read-only view where authorized | `409` | `manual grade is no longer pending`; no queue or audit mutation. |
| Missing `If-Match` | n/a | `428` | Exact local precondition convention. |
| Multiple/malformed/non-quoted/stale `If-Match` | n/a | `422` / `412` | `422` for syntax; `412` for a valid stale ETag, return current ETag only on the latter. |
| Missing/malformed key or strict body | n/a | `400` / `422` | No mutation; malformed identity header is `400`, invalid JSON/decimal/body is `422`. |
| Exact key + exact fingerprint replay | n/a | `200` | Stored `ManualGradeResult`, same committed revision/generation, no second job. |
| Same key but changed actor/attempt/body/If-Match fingerprint | n/a | `409` | Key is not reusable for another mutation. |
| Store/backend unavailable | `503` | `503` | Generic storage message; no partial score publication. |

The `412` distinction is intentional: an ETag is a current-evaluation compare-and-swap, whereas
`409` is a state-machine or idempotency-identity conflict. Existing assignment updates currently
map stale revisions to `409` (course.rs:446-466); this new route should use the standard
precondition result rather than make its API indistinguishable from a settled/non-pending attempt.

## Required source and schema touch points

1. Add a **forward** SQLx migration, not an edit to the six-file baseline: introduce
   `submission_evaluation.revision BIGINT NOT NULL DEFAULT 1 CHECK (revision > 0)` and a bounded
   `manual_grade_receipt` table keyed by `(tenant_id, attempt_id, idempotency_key)` with a request
   digest, actor, resulting revision/generation, timestamp, and no response/answer/grade payload.
   Give it forced RLS, tenant policy, application insert/select grants, retention cleanup, and an
   index for its primary replay lookup. The established baseline is documented as a coherent
   six-file checkpoint ([partial_commit_status.md](../partial_commit_status.md):7-18, 75-89); a
   new migration preserves SQLx checksum guarantees rather than silently modifying it.
2. In `crates/learning-data-access/src/lib.rs`, add typed `ManualGradeEvaluationCommand`,
   `ManualGradeEvaluation`, `ManualGradeView`, and a narrow `Store` method that returns the current
   result/replay. Keep the command's credit as a validated decimal domain type, not `f64`, and give
   private receipt/audit inputs custom redacted `Debug` like `SubmitQuestionAttemptCommand`
   (lib.rs:1280-1310).
3. Implement exactly the same transaction/state contract in
   `crates/learning-data-access/src/postgres.rs` and `crates/learning-data-access/src/in_memory.rs`. PostgreSQL must use RLS tenant
   setup plus `FOR UPDATE`; MemoryStore must make the whole transition under its write lock so
   conformance tests retain their meaning.
4. Add instructor DTOs/handlers/ETag parsing in `crates/server/src/run.rs` and mount them in the
   existing `router`, not a second server module. Add the route to the composition route-mount test
   (composition.rs:1496-1530). Do not add the private Store evaluation type to generated
   `question_model` browser contracts.
5. Reuse `crates/server/src/course.rs` only for the extracted/direct-course authorization helper
   if doing so does not make `run.rs` depend on a route module. Prefer a small Store ownership
   helper shared at the storage boundary over an HTTP-layer check-then-write split.

## Permanent behavior tests

Add deterministic unit/conformance/HTTP behavior tests, not count/default/mock wiring tests:

- Store conformance for a response-bearing manual item: direct instructor grades it, exact replay
  returns the first result, stale revision fails, changed replay payload fails, student/foreign
  callers cannot enumerate it, and one scoring job/generation is produced.
- Store conformance for one mixed run: an automatic response and a manual-pending response coexist;
  no current assignment result publishes until the manual evaluation becomes graded; then the
  generation-fenced worker publishes the combined current projection. Assert clear and retention
  remain valid afterward.
- Store conformance for a force-submitted no-response attempt: manual grading is rejected and
  neither a submission nor evaluation row is created. This preserves the existing proof at
  conformance.rs:4296-4304.
- PostgreSQL acceptance fixture: verify the forward migration, RLS denial under each real app role,
  exact receipt replay, `FOR UPDATE` stale-write behavior, and one current evaluation/score after
  concurrent grade attempts.
- `crates/server/src/run.rs` HTTP tests: absent/foreign/non-instructor non-enumeration; no-store on
  every status; strict JSON; absent/malformed/stale `If-Match`; exact/mismatched idempotency replay;
  ETag advance; no answer key/rubric in either view; and current-result `recalculating` projection.
- Extend `crates/server/src/composition.rs` only with its existing route-existence assertion. This
  is a permanent composition smoke test, not a production worker-drain claim.

Focused gates after implementation: `cargo test -p learning-data-access --test conformance`,
`cargo test -p server run::tests`, PostgreSQL Store acceptance with the new migration, then the
database-plan complete gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo test --workspace --all-features`, `./check_codebase.sh`, and `pytest tests/`.

## Security boundary retained

Manual grading changes a normalized result only after the server has loaded the immutable published
question, protected student response, and current tenant evidence. The browser never sends points,
correctness, answer keys, grading rules, private rubric, or a provider grade; it receives only
instructor-authorized student evidence and the current grade state. Problem content remains
immutable, assignment changes remain separate revisioned current state, and the grade worker-not
the HTTP handler-publishes the recalculated current result. This preserves the plan's separation of
immutable facts, mutable current state, and replaceable projections.

## Assumptions and risks

- The status document's six-file checkpoint is treated as frozen even though production durable
  data is not yet accepted. A forward migration is safer than renumbering/checksum-changing the
  audited baseline. If the owner explicitly chooses to rewrite the pre-data baseline, preserve the
  same schema invariant but re-run its entire fresh-install/checksum acceptance gate.
- The present automatic submission path always writes `grading_status = 'graded'`
  ([postgres.rs](../../../crates/learning-data-access/src/postgres.rs):8817-8830). The implementation package also
  needs a server-only `ManualReviewRequired` disposition that persists the student response and a
  pending evaluation atomically. It must not surface the generic checker error as a `422` for a file
  upload or other manual-review question.
- A standalone grade-comment/rubric UX is intentionally excluded. Persisting free text would be a
  new bounded student-record payload, retention policy, disclosure policy, and audit contract.
- This audit finds sufficient authority evidence for a route. It does not authorize a role-wide
  shortcut, an assignment-level `If-Match`, a client-side score preview, or a partial worker
  registry. Those would violate the demonstrated local boundaries.
