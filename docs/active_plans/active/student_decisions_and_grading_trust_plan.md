# Plan: Phase 2 Student decisions and scoring trust

Status: in progress. M1 and M2.5 are complete. The former M2/M3 background grading lifecycle is
superseded and is not completion evidence. M2.5 audit cleanup removed obsolete grading-job, retry,
and attention residue; its fresh aggregate receipt is session 38324. M4 remains retained and open.
The manager follows
`production_release_readiness_phase_plan-v2.md` when it differs from this subordinate
implementation plan; that v2 plan is the Phase 2 authority.

## Context

M1 gives Students server-owned access decisions in their selected IANA time zone, resumable timed
Attempts, autosave continuity, and expiry finalization. Those behaviors remain complete.

The former interim implementation accepted saved responses, queued grading Jobs, later recorded a
result, and exposed grading states, polling, and Gradebook attention counts. That public lifecycle
is rejected and not current guidance. Background execution remains justified only for abandoned
expiry finalization and a backend whose completion is deferred; both stay hidden from Students and
Instructors.

The approved architecture model is direct: a Question Backend evaluates a submitted response and
returns credit; PLE records immutable credit as Student Work; PLE calculates Assignment scores
from that credit and current Assignment Entry point values. PLE does not gain a generic regrade
operation or interpretation of backend control formats.

## Objectives

- Preserve M1 access, time-zone, continuity, timer locking, and expiry behavior.
- Store one immutable normalized credit fraction for every submitted Question response.
- Calculate displayed scores from stored credit, current Entry points, and the issued scoring rule.
- Remove the user-visible grading lifecycle and instructor-attention workflow.
- Retain the eleven-topic Genetics Blueprint Course after corrected behavior is verified.

## Design philosophy

Apply KISS aggressively. Direct saved-snapshot evaluation and atomic commit satisfy the product
contract. Reuse the existing worker for abandoned expiry and deferred backend completion; a queue,
worker, retry policy, materialized score, extra state, or permanent test earns inclusion only for
that demonstrated operation. One-time source, renderer, and timing probes remain temporary evidence.

## Scope

- Replace grading-time point snapshots with immutable normalized credit and current-point scoring.
- Change manual submission and expiry finalization to use one direct finalizer, reusing the existing
  worker for abandoned Attempts and deferred backend completion.
- Remove grading states, polling, attention counts, and grading-detail routes.
- Permit zero-valued Assignment Entries consistently with the Rust point-value type while preserving
  the current positive-denominator completion rule.
- Complete the retained Genetics Blueprint Course after M2.5.

## Non-goals

- Build a PLE regrade interface, retry-grading workflow, or backend-control parser.
- Promise a literal HTTP timing shape beyond an observable direct submission result.
- Add repeated expiry-submission retry behavior, a score queue, materialized totals, or assessment
  taxonomy work.
- Build a new generic background-job framework or expose background completion as user state.
- Change issued revision/seed evidence, scoring-rule policy, or feedback disclosure.
- Turn Genetics source content or probes into fixtures, inventories, or recurring gates.

## Current state summary

| Area       | Current evidence                                     | M2.5 direction                                                            |
| ---------- | ---------------------------------------------------- | ------------------------------------------------------------------------- |
| Submission | Direct finalization evidence exists                  | Keep one ordinary finalizer for Student and expiry submission             |
| Backends   | Native and WeBWorK return backend-owned credit today | Keep backend ownership and normalized credit only                         |
| Result     | Immutable normalized credit is recorded              | Remove legacy grading-job/failure residue from the remaining boundary     |
| Scoring    | Readers apply current Entry points to stored credit  | Keep scores derived on read                                               |
| Public UI  | Polling and Gradebook attention detail were removed  | Keep completed results without grading-progress UI                        |
| Expiry     | M1 closes further edits at expiry                    | Existing narrow worker finalizes abandoned expired work; reads stay reads |

Architecture review approved one persisted normalized_credit fraction. All current backends derive
correct from that fraction, including rare partial credit, so correct is not stored independently.
Disclosure, completion, and statistics derive correct where needed as credit equals one.

AssignmentPointValue already permits zero, but assignment_entry SQL still requires positive values.
The smallest compatible correction is greater-than-or-equal-to zero for fixed and Pool point values.
Existing score_at_least retains its positive-denominator rule; zero-total work creates no new
completion semantics.

## Architecture boundaries and ownership

- A Question Backend owns presentation, response interpretation, grading, partial-credit meaning,
  and backend-specific state. PLE stores its outcome without a generic regrade capability.
- One direct finalizer owns authorized saved-snapshot capture and atomic acceptance. A backend may
  return credit immediately or complete through hidden polling; the existing worker finalizes
  abandoned expiry work and deferred completion. Before expiry, saved work remains open; after
  expiry, edits remain closed.
- PostgreSQL owns immutable credit, expiry locking, current Entry values, and score readers. Atomic
  commit rejects a stale snapshot, so older saved work cannot become accepted after a newer save.
- A shared SQL scorer joins issued_question.assignment_entry_id to the current fixed point value or
  Pool point-per-item, including retired Entries, then applies the issued scoring rule.
  assignment_attempt.completion_score remains historical lifecycle evidence, not a current score.
- Browser/API present results and scores, never grading progress.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / workstream | Components                                                             | Review boundary                                         |
| ---------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------- |
| M2.5 / K1              | grading.sql, assignments.sql, score readers and projections            | Immutable credit and current-point scoring              |
| M2.5 / K2              | submission/finalization paths, adapters, existing worker               | Immediate or hidden-polled completion; atomic commit    |
| M2.5 / K3              | grading Jobs, worker composition, status API/browser, Gradebook detail | Remove only lifecycle machinery K1/K2 makes unnecessary |
| M4 / J                 | ordinary Question publication and Blueprint authoring                  | Retained curriculum, not fixture content                |

## Milestone plan

| M    | Title                                   | Summary                                                | Goal                                             |
| ---- | --------------------------------------- | ------------------------------------------------------ | ------------------------------------------------ |
| M1   | Student decisions and resumable Attempt | Complete access, zone, save/reconnect, and timer lock  | Preserve completed behavior                      |
| M2.5 | Direct credit and current-point scoring | Credit storage, direct finalization, lifecycle removal | Fixed response credit; points change scores only |
| M4   | Genetics Blueprint Course               | Publish eleven ordered topic Assignments               | Retained semester-ready Blueprint                |

### Milestone: M1 Student decisions and resumable Attempt

- Status: complete 2026-09-13.
- Done checks: the existing focused and Chromium evidence remains the M1 receipt.
- Parallel-plan ready: no. M1 must remain intact through M2.5.

### Milestone: M2.5 Direct credit and current-point scoring

- Status: complete 2026-09-14. The audit removed obsolete grading-job/failure residue and the
  redundant expiry store test/runner under the test policy, without replacement. The fresh exact
  aggregate passed in session 38324; M4 remains the Phase 2 work.
- Depends on: M1 because direct finalization respects its expiry lock.
- Deliverables: WP-K1 through WP-K3.
- Entry criteria: architect approval of direct credit and on-read scoring. Approved 2026-09-13.
- Exit criteria: every submitted response has one immutable credit result; manual, expiry, and
  hidden backend completion share a path; score reads use current points without backend
  interaction; no public grading lifecycle remains.
- Parallel-plan ready: yes after K1. K2 and K3 have separate owners; schema is serialized through
  K1. Maximum two.

### Milestone: M4 Genetics Blueprint Course

- Status: in progress, held pending M2.5.
- Depends on: M2.5 so Course Instance evidence observes corrected behavior.
- Exit criteria: every in-scope bank has a faithful supported disposition; the Blueprint reloads;
  a disposable Instance proves delivery, continuity, expiry finalization, immutable credit, and
  current-point score display.
- Parallel-plan ready: yes, maximum two for source reconciliation and representation proof;
  population follows serially.

## Work packages

### Work package: WP-K1 Store credit and score with current points

- Owner: one PostgreSQL/read-model owner.
- Touch points: grading.sql, assignments.sql, grading_access.sql, Student landing/history readers,
  Gradebook readers, completion/history projections, and contracts.
- Depends on: M1.
- Outcome: replace grading-time earned/possible snapshots with immutable normalized_credit. One
  shared scorer applies current Entry values and the issued scoring rule.
- Acceptance criteria: point edits change displayed scores without a Question Backend interaction
  or result mutation; fixed/Pool and retired Entries work; current scoring rules remain; zero
  points do not create zero-denominator completion.
- Evidence: focused connected PostgreSQL proof. Keep a permanent test only for a stable invariant
  not already covered.
- Obvious follow-on: WP-K2. Existing executor compatibility is temporary only while K1 proves the
  new evidence/read boundary; K1 is not a completed replacement milestone by itself.

### Work package: WP-K2 Directly finalize saved responses

- Owner: one server/PostgreSQL/backend owner.
- Touch points: attempt operations, submission server path, native/WeBWorK adapter paths, expiry
  finalization, and the existing worker.
- Depends on: WP-K1.
- Outcome: authorized saved snapshot, then atomic current-snapshot verification and commit of
  Question Submissions, credit, and Attempt completion. Backend completion may be immediate or
  hidden polling; abandoned expiry uses the existing worker.
- Acceptance criteria: backend failure and pre-commit process loss record no acceptance; replay
  reads immutable credit; concurrent save rejects stale preparation; expiry/manual/worker races
  converge through server clock and unique Assignment Submission; unanswered expiry positions close
  at zero without backend evaluation. Gradebook, result, and export reads never call a Backend or
  write; expires_at logically blocks edits and counts before physical finalization.
- Decision procedure: reuse the existing worker to submit abandoned expired Attempts and to poll a
  backend only when that backend has accepted a response but not returned credit. Do not delete and
  recreate it or expose its progress publicly.
- Evidence: focused native/WeBWorK checks plus a disposable abandoned-expiry and hidden-completion
  behavior observation.
- Obvious follow-on: WP-K3.

### Work package: WP-K3 Retire grading lifecycle surface

- Owner: one API/browser/worker cleanup owner.
- Touch points: grading Job SQL/grants, Stores/adapters, server worker/composition, browser
  contracts, status routes/clients/decoders/polling, Gradebook attention fields/detail, and tests.
- Depends on: WP-K2.
- Outcome: remove four public states and all Student/Instructor grading-progress or attention paths.
  Narrow the existing worker to abandoned expiry and deferred backend completion.
- Acceptance criteria: no state/polling/attention projection or grading/regrading/retry-grading
  capability survives; unrelated public-asset Jobs remain intact.
- Evidence: public-contract/route search, focused authorization, and visible submit/result check.
  Remove tests that freeze retired orchestration.

### Work package: WP-J1 Publish the Genetics Blueprint Course

- Owner: one Question/Blueprint integration owner with independent review.
- Depends on: M2.5.
- Outcome: retain one reusable Blueprint Course with eleven ordered Genetics topic Assignments, with
  each current source bank retained as a Question Pool.
- Acceptance criteria: source grouping and variation survive in those Pools; opaque backends retain
  document/grading ownership; no bank is silently flattened or omitted.
- Evidence: temporary reconciliation/probe evidence is removed at close-out; curriculum and
  provenance are product data.

### Work package: WP-J2 Verify the Genetics Course

- Owner: an independent Course behavior owner.
- Depends on: WP-J1.
- Outcome: reload the Blueprint and prove a normal disposable Course Instance journey.
- Acceptance criteria: release, render, save/reload/second session, expiry finalization, immutable
  credit, current-point scoring, and Gradebook display work. The Blueprint has no enrollment or
  deadlines.
- Evidence: one connected behavior receipt; no exact-content inventory test.

## Acceptance criteria and gates

- New permanent tests must pass the [PYTEST_STYLE.md](../../PYTEST_STYLE.md) checklist.
  Otherwise use tests/_temp and remove the proof at close-out.
- M2.5 requires focused backend, PostgreSQL, API, and browser behavior evidence, then
  source source_me.sh && ./launchers/all_test.sh. A green aggregate alone is insufficient.
- M4 requires Blueprint reload and a disposable Course Instance receipt after M2.5. No human gate
  or content-inventory fixture is required.
- A red behavior gate blocks its dependent package; repair its named contract before scope grows.

## Test and verification strategy

- Protect immutable credit, current-point scoring without backend interaction, expiry locking, and
  authorization only where existing durable tests do not already establish them.
- Use temporary renderer failure evidence to discover an actual operational limit. Do not set
  arbitrary latency thresholds or preserve a queue by default.
- Use connected PostgreSQL proof for atomic finalization/current-point joins and a browser journey
  for visible submission, result, reload, expiry, and Gradebook behavior.
- Remove temporary Genetics probes and retired lifecycle tests unless they independently earn
  permanent-test admission.

## Risk register

| Risk                         | Impact                       | Trigger                                | Owner | Mitigation                                      |
| ---------------------------- | ---------------------------- | -------------------------------------- | ----- | ----------------------------------------------- |
| Stale evaluation snapshot    | Older saved work is accepted | Concurrent save                        | K2    | Atomic current-snapshot check                   |
| Wrong current-point join     | Score distorts history       | Point edit/read mismatch               | K1    | Shared scorer with issued facts                 |
| Retired public surface       | False user workflow survives | Route, DTO, or polling remains         | K3    | Contract search and visible check               |
| Abandoned expiry work stalls | Saved work never finalizes   | Existing worker stops or misses expiry | K2    | Reuse and verify the bounded expiry worker path |
| Source presentation loss     | Genetics course incomplete   | Unsupported rich source                | J1    | Refuse and repair; never flatten/drop           |

## Documentation close-out requirements

- Update contracts, including TERMINOLOGY_CONTRACT.md, database structure, architecture, and
  changelog only for accepted M2.5 behavior.
- Replace obsolete lifecycle descriptions rather than presenting their removal as old completion.
- Record focused M2.5/M4 evidence and any unrun boundary honestly.

## Open questions and decisions needed

No human decision blocks execution. An independent expiry sweep requires a demonstrated product need
and architect approval.
