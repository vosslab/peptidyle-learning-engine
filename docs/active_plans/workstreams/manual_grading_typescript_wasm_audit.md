# Manual grading TypeScript and Wasm boundary audit

> **Historical audit.** This dated audit is retained as evidence, not current task direction.
> Current authority is the [release completion plan](../active/release_completion_plan.md) and
> [implementation status](../implementation_status.md).

Status: complete read-only audit, 2026-08-08. No production change is required in the TypeScript
client or the Wasm bridge before the server/store manual-grading behavior exists. This note records
the narrow contract required if the package includes instructor review UI.

## Decision

Manual grading is an instructor-only server workflow. It must not alter the learner envelope,
student submission acknowledgement, generated question-content model, or the Wasm API. A submitted
manual item can move an attempt to `needs_manual_grading` (already modeled at
`crates/question_model/src/activity.rs:248-264`); server-side evaluation then records the current
normalized credit fraction and asks the existing scoring-generation worker to recalculate. The
browser submits a grade decision and receives only an action result/current status. It never sees an
answer key, checker, expected response, evaluator implementation, or a local correctness decision.

This matches the database plan's separation of server-produced normalized credit from mutable
assignment points (`docs/active_plans/decisions/database_schema_evolution_plan.md:214-241`) and its
requirement that `submission_evaluation` carries manual-grading status while current summaries remain
the gradebook read model (`docs/active_plans/decisions/database_schema_evolution_plan.md:244-276`).

## Current boundary evidence

| Surface                 | Evidence                                                                                                                                                                                                                                                                  | Finding                                                                                                           |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Generated content types | `crates/question_model/src/definition.rs:220-249` makes `GradingDefinition` policy/weight only; `crates/question_model/src/envelope.rs:82-103` exposes only version, seed, title, prompt, and response shape.                                                             | No answer-bearing content needs a new generated binding.                                                          |
| Learner transport       | `src/api/contracts.ts:73-122` has key-free issued and summary projections; `src/api/decoders.ts:1866-1908` strictly decodes the envelope.                                                                                                                                 | Do not add manual status, a response-to-grade, or a grade command to learner routes.                              |
| Existing gradebook      | `crates/question_model/src/course.rs:89-111` defines a compact summary-only row; `src/pages/gradebook_page_model.ts:10-25` makes one summary request and lazy history; `tests/test_gradebook.mjs:67-80` forbids attempt, feedback, and grading reads in its initial view. | The existing default gradebook cannot drive manual review and must remain summary-only.                           |
| Decoder hardening       | `src/api/decoders.ts:1448-1514` rejects unknown grading/draft fields; `tests/test_http_client.mjs:56-88` proves answer and provider-secret rejection; `src/api/decoders.ts:2387-2463` strictly validates compact gradebook rows.                                          | New instructor DTOs require their own exact-field decoders and must not reuse permissive storage/domain decoding. |
| Wasm closure            | `crates/wasm/Cargo.toml:14-22` depends only on `question_model` and `domain`; `crates/grading/Cargo.toml:1-14` is server-only; `tests/test_crate_boundaries.py:148-185` proves the exact closure and no private feedback crossing.                                        | Manual grading must add no Wasm dependency, export, or facade method.                                             |
| Wasm exports            | `crates/wasm/src/lib.rs:29-109` exports format validation, timing, capability validation, and draft preview only; `tests/e2e/e2e_wasm_export_allowlist.mjs` freezes that list.                                                                                           | A `grade`, `manual_grade`, `correct`, or answer-related Wasm export is prohibited.                                |

The TypeScript facade does import full `QuestionDefinition` for author capability validation
(`src/wasm/index.ts:59-76`), but the learner route is independently guarded against it and grading
terms (`tests/test_frontend_contract.mjs:253-263`). Manual review must not broaden that facade.

## Proposed API DTO projection

These are API-owned transport shapes, not generated `question_model` types and not Store/PostgreSQL
rows. Put the Rust route request/response beside the authenticated manual-grading handler and the
matching browser contracts/decoders beside `src/api/contracts.ts` and `src/api/decoders.ts`. The
server maps to/from domain/storage types after authorization and validation.

```ts
// src/api/contracts.ts -- instructor-only transport projection
export interface ManualGradingQueueItem {
  readonly courseId: CourseId;
  readonly assignmentId: AssignmentId;
  readonly assignmentTitle: string;
  readonly enrollmentId: EnrollmentId;
  readonly studentId: StudentId;
  readonly runId: RunId;
  readonly attemptId: QuestionAttemptId;
  readonly assignmentPosition: number;
  readonly submittedAt: number;
  readonly question: QuestionEnvelope;
  readonly response: StudentResponse;
  readonly evaluationRevision: string;
}

export interface SubmitManualGradeInput {
  readonly actionId: string; // UUID supplied once and reused for a retry
  readonly evaluationRevision: string; // strong ETag / optimistic revision
  readonly creditFraction: string; // canonical, bounded decimal; not binary points
  readonly feedback: ReadonlyArray<ContentBlock> | undefined;
}

export interface SubmitManualGradeResult {
  readonly attemptId: QuestionAttemptId;
  readonly status: "submitted" | "needs_manual_grading";
  readonly scoringStatus: ScoringStatus;
}
```

`ManualGradingQueueItem.question` is the same key-free rendered envelope used for a student, and
`response` is the protected student evidence an authorized instructor needs to assess. Neither is an
answer. `feedback` is optional instructor-authored, sanitized teaching material; it is server-stored
and later passes through the existing policy-redacted `DisclosedFeedback` route, never directly to a
learner. The plan permits partial _or negative_ normalized credit, so the decoder must not silently
clamp it; the server owns the documented finite range and canonical-decimal parsing. Assignment
points remain exclusively server-derived from the current item/scoring mode.

The queue is cursor-paged and scoped by the authenticated course authority. It should be exposed by
a dedicated instructor route, for example `GET /api/instructor/courses/:courseId/manual-grading`, and
the command by `POST /api/instructor/attempts/:attemptId/manual-grade`; route paths are proposals,
not an existing contract. Both require the server's course-role check, tenant context, bounded page
size/body, action-id idempotency, and `If-Match`/revision conflict handling. Do not send `tenantId`
from a browser command and do not accept a student-selected attempt as authority.

For the compact gradebook, the only justified extension is a small maintained status/count projection
(for example, `pendingManualGradeCount` and assignment `scoringStatus`) if instructors must see that
work exists. It belongs on an explicit gradebook-summary DTO backed by a current summary projection,
not by per-row attempt scans. Do not put queue response bodies, prompt blocks, feedback, or per-item
scores in `GradebookSummaryRow`.

## Ownership and decoder requirements

| Owner            | Shape                                                                         | Must not be reused as                        |
| ---------------- | ----------------------------------------------------------------------------- | -------------------------------------------- |
| Store/PostgreSQL | submission, submission-evaluation, current summary, idempotency/audit records | browser JSON or generated model              |
| Domain/grading   | authoritative checker result, answer key, normalized grading semantics        | Wasm input or TypeScript DTO                 |
| Server route     | authorized queue projection and manual-grade command/result                   | generic question definition or gradebook row |
| Browser client   | strict decoded DTOs and view state                                            | source of correctness or point calculation   |
| Wasm             | response-format, timer, capability, and safe preview tools only               | manual grading engine or review UI backend   |

Required runtime-validation cases:

1. Queue response rejects extra `answer`, `answerKey`, `expectedResponse`, `checker`, `grading`,
   `provenance`, `sourceArtifact`, provider credential, and raw object-key fields at every nested
   envelope/response record.
2. Queue item rejects foreign/mismatched route course IDs, non-UUID identifiers, null/missing
   response or submission time, invalid position/timestamp, an unrecognized status, and a response
   inconsistent with the rendered response definition.
3. Command decoder accepts only an action UUID, a strong revision, a canonical finite decimal
   credit value, and bounded sanitized feedback blocks. It refuses `pointsEarned`, `correct`,
   `pointsPossible`, grading policy, an answer, or client-supplied tenant/student/course identity.
4. Result decoder accepts only the requested attempt ID, the post-command status, and recognized
   scoring status; it rejects a current score, answer-bearing evaluation, and unexpected fields.
5. A gradebook status/count extension rejects attempt arrays, response bodies, run history, and
   answer/feedback/evaluator fields exactly as the present gradebook decoder rejects extra history
   (`tests/test_http_client.mjs:302-325`).
6. Mocks must serialize the exact new API contract and model idempotent retry/conflict/forbidden
   paths; they must not call local Wasm or a checker to calculate credit.

## UI scope

No existing page needs a mechanical update to make server behavior correct. If this package includes
the instructor operation, add a focused review panel reached from the gradebook rather than changing
the initial gradebook fetch. It should show the issued prompt, the student's submitted response,
clear `Needs manual grading` / `Recalculating` status, a decimal-credit input, save/conflict/retry
states, and a return-to-queue control. Student screens continue to show only policy-released feedback
after the server round trip. The current gradebook deliberately shows compact summaries and lazy
run history (`src/pages/gradebook_page.tsx:113-275`), so it is not presently a manual-review UI.

## Required implementation gates

Run the narrow contract checks after adding the DTO and before the full package gate:

```bash
cargo run --quiet -p project-tools -- tsgen
npx tsc --noEmit -p tsconfig.json
node --import tsx --test tests/test_gradebook.mjs tests/test_http_client.mjs tests/test_frontend_contract.mjs
source source_me.sh && python3 -m pytest -q tests/test_crate_boundaries.py
node tests/e2e/e2e_wasm_export_allowlist.mjs
cargo test -p wasm_bridge
./check_codebase.sh
```

The first command verifies generated bindings remain derived from the Rust-owned public model; it is
not permission to generate an instructor route DTO from a storage row. The Python closure check is a
permanent fast security test. The Wasm export allowlist is a permanent emitted-artifact E2E check,
not a fast Node unit test. A native-versus-Wasm parity test is intentionally not added for manual
grading because grade calculation is server-only; requiring one would weaken the security boundary
rather than test product behavior. A one-time browser network trace should confirm that the
manual-review request/response contains no answer material and that a learner never receives the
instructor queue route.

## Audit execution

- `npx tsc --noEmit -p tsconfig.json`: exit 0 with no diagnostic output.
- The focused gradebook, HTTP-client, and frontend-contract Node suite passed. The slower Wasm export
  artifact check runs separately from `tests/e2e/e2e_wasm_export_allowlist.mjs`.
- `source source_me.sh && python3 -m pytest -q tests/test_crate_boundaries.py`: 4 passed.

No production files were edited. No change to generated bindings, client contracts/decoders/mocks,
gradebook UI, `crates/wasm`, Wasm exports, or the `grading` dependency closure is appropriate until
the server/store manual-grading contract exists.
