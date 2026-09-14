# Plan: Remove grading Retry drift and restore expiry auto-submission

Status: complete and archived 2026-09-13. R1 through R6 pass, including fresh PostgreSQL,
disposable Live Demo, fast offline, and exact full aggregate acceptance.

## Current reset receipt

| Package                | Current result                                                                                                                                                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| R1 authority           | Governing plans define recovery as expiry auto-submission and define no human grading action                                                                                                                                   |
| R2 schema              | Manual grading Retry and state re-entry are absent; exact lease-token requeue is limited to the same unfinished Job before any result                                                                                          |
| R3 contracts           | Student and Course Instructor grading APIs are GET-only, answer-free status projections with no generation or mutation DTO                                                                                                     |
| R4 evidence and docs   | Retired per-Question submit/status/prefetch and browser recovery code is removed; current durable docs describe save, whole-Attempt finalization, and expiry auto-submission                                                   |
| R5 fast gate           | `run_fast_checks.sh` generated 337 browser types and passed Rust, 374 Node, and 6,126 Python checks                                                                                                                            |
| R6 connected/full gate | Fresh PostgreSQL lifecycle and baseline evidence, Chromium expiry auto-submission, native worker replacement, and closed WeBWorK transient/final/expired-lease cases pass; the exact `all_test.sh` aggregate ends with complete live acceptance green |

## Context

Question responses are saved while the Student works, and reconnecting resumes the same active
Assignment Attempt without resetting its server-owned clock. The recovery is server-owned
auto-submission of those saved responses when the Attempt expires.

An agent-generated grading design crossed the submission boundary and introduced an unrelated
Instructor-triggered Retry of accepted work. Repository history shows that the concept first entered
the older automated-grading design on 2026-08-27, was revived by the 2026-09-12 readiness report as
an assumed missing "Instructor recovery journey," and was promoted into the initial phased readiness
plan in commit `24cbb8c5`. The extracted Phase 2 plan then incorrectly described it as a human-stated
requirement and spread it through Human Guidance, schema transitions, SQL capabilities, Rust Store
contracts, generated browser contracts, connected tests, and current documentation.

The human corrected the authority on 2026-09-13: Instructors do not grade or retry Student work in
PLE, grading is deterministic, and recovery is auto-submission. The invalid Human Guidance sentence
has been deleted. The reset removes the resulting SQL and Rust mutation design while retaining the
valid pre-result worker and read-only status boundaries.

## Objectives

- Make server-owned expiry auto-submission the only user-facing Assignment Attempt recovery model;
  autosave preserves responses and reconnect/resume continues the same still-active Attempt.
- Remove every Instructor grading or regrading action from current plans, schema, APIs, Store
  contracts, browser contracts, tests, and current documentation.
- Preserve deterministic automatic grading over one immutable accepted response and make every
  recorded Grading Result terminal and immutable.
- Preserve bounded worker requeue only when an infrastructure failure produced no Grading Result;
  describe that behavior as pre-result Job requeue, never recovery or regrading.
- Establish a fast offline verification launcher and finish with the exact full aggregate gate on
  the corrected tree.

## Design philosophy

Use the repository's long-term and evidence-first principles to restore the smallest truthful model:

> Save responses during the active Attempt, resume that Attempt after connection loss, auto-submit
> at expiry, and grade the immutable accepted response once; if infrastructure produces no result,
> the worker may requeue the same unfinished Job.

Scrap the invented Instructor Retry lane rather than preserving compatibility for an unapproved
capability. Incrementally retain the valid Attempt, immutable evidence, worker lease, classified
failure, read-only status, and Gradebook work that does not depend on Instructor mutation.

## Scope

- Correct the original phased readiness plan, the extracted Phase 2 plan, and the active progress
  copy so they state the Assignment Attempt recovery boundary accurately.
- Remove the public and private Instructor Retry SQL functions, grants, resolution helpers, and
  failed-to-ready or attention-to-pending generation-bump transitions.
- Restore fixed-generation Job semantics and remove grading-operation generation fields and
  parameters that exist only to support a newer regrading operation.
- Remove Retry request/result DTOs and Instructor Store mutation methods while retaining answer-free,
  read-only Instructor grading status and Gradebook metadata.
- Replace connected tests of Instructor Retry with tests proving terminal failure cannot be reopened,
  a completed result cannot be replaced, and worker requeue occurs only before a result exists.
- Correct current contracts, architecture, lifecycle, failure, database, security, news, active-plan,
  and changelog prose introduced or modified by the current Phase 2 work.
- Adapt `launchers/run_fast_checks.sh` to this repository as the offline portion of the aggregate
  gate: Rust, TypeScript/Node, and Python checks without the disposable live stack.
- Regenerate Rust-owned browser contracts and verify that no Retry DTO remains.

## Non-goals

- Change the human-approved Assignment Attempt timer, autosave, reconnect/resume, or expiry
  auto-submission behavior.
- Remove ordinary Retry or Reload actions for failed browser persistence, unknown submission
  acknowledgement, or other unrelated network operations.
- Remove bounded automatic worker requeue after a transport, object-store, renderer, or timeout
  failure that produced no Grading Result.
- Add a Student, Instructor, Sysadmin, or operator action that regrades accepted work.
- Add a correction workflow for a defective Question or grader; that requires a separate human
  product decision and append-only correction design.
- Rewrite historical archives or dated audit reports as though they had never contained the drift;
  current authorities may identify them as superseded evidence.
- Rename the existing read-only `needsInstructorAttention` status in this reset. It is notification
  metadata only and grants no grading action; any vocabulary change needs a separate product decision.

## Initial state and reset disposition

| Area                   | Current state                                                                                              | Reset disposition                                                                     |
| ---------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Human authority        | The invalid Instructor Retry sentence has been deleted from `docs/HUMAN_GUIDANCE.md`                       | Keep the deletion; add no agent-authored replacement policy                           |
| Original phased plan   | Instructor Retry appears in Phase 2 scope, milestones, workstream I, WP-F1, WP-I1, gates, tests, and risks | Rewrite current plan claims around Attempt recovery and automatic grading             |
| Extracted Phase 2 plan | Retry was falsely attributed to the human and expanded into WP-F1/F4/F5/I2/J2                              | Remove WP-I2 mutation and re-scope I to read-only status                              |
| Schema                 | Manual Retry functions and generation-bump transitions were added; partial deletion is in progress         | Remove completely; restore one fixed operation per accepted response                  |
| Rust/browser contracts | Instructor Retry DTOs and Store mutation were added; partial deletion is in progress                       | Remove completely; retain read-only detail without generation                         |
| Worker                 | Classified no-result failures can requeue automatically before exhaustion                                  | Retain; use worker-requeue terminology and never reopen a result                      |
| Tests                  | Connected tests currently prove manual Retry and generation bump                                           | Replace with terminality, immutability, authorization, and automatic requeue evidence |
| Aggregate evidence     | `source source_me.sh && ./launchers/all_test.sh` exited 0 before the correction                            | Diagnostic only; rerun after reset and do not cite the earlier run as acceptance      |
| Fast launcher          | Copied script calls foreign `build_github_pages.sh` and is not valid for this repo                         | Make it the aggregate minus `local_stack.py acceptance`                               |

## Resolved decisions

- Internet-loss recovery ends at expiry auto-submission; it does not authorize any grading action.
- A completed deterministic grade is never retried, replaced, or superseded.
- A transient infrastructure error is not a grade. The existing worker may requeue the unfinished
  Job with the same immutable accepted response while no Grading Result exists.
- A final or exhausted Job remains terminal. The Instructor may see answer-free status but has no
  PLE grading control.
- Lease tokens fence worker ownership. The manual Retry design's operation-generation bump and its
  browser-visible expected generation are removed.
- The reset is a direct pre-production correction with no compatibility alias or forward migration.
- Unrelated Retry vocabulary remains where it names persistence recovery, exact request replay,
  worker requeue, cleanup, or another established non-grading behavior.

## Architecture boundaries and ownership

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component                                                                 | Review boundary                                                                |
| ---------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| R1 / Authority         | Human Guidance and current plans                                          | Recovery means expiry auto-submission and does not imply any grading action    |
| R2 / Schema            | `jobs.sql`, `grading.sql`, `grading_access.sql`, install/security catalog | No human grading mutation; worker requeue is possible only before a result     |
| R2 / Contracts         | Browser API contract and LDA trait/Postgres adapter                       | Instructor surface is read-only and answer-free; no generation or mutation DTO |
| R3 / Evidence          | Connected grading tests, current docs, changelog                          | Tests prove durable behavior and negative capability absence                   |
| R3 / Fast gate         | `launchers/run_fast_checks.sh`                                            | Exact aggregate offline gates only; no duplicate or foreign build choreography |

## Milestone plan

| M   | Title                     | Summary                                                                           | Goal                                                  |
| --- | ------------------------- | --------------------------------------------------------------------------------- | ----------------------------------------------------- |
| R1  | Restore authority         | Correct both governing plans and preserve the Human Guidance deletion             | Recovery means expiry auto-submission                 |
| R2  | Remove invalid capability | Delete manual Retry, generation re-entry, and mutation contracts                  | No PLE actor can regrade accepted work                |
| R3  | Rebuild evidence          | Replace drift tests/docs, adapt fast checks, and run focused plus aggregate gates | Corrected tree is mechanically and semantically clean |

### Milestone: R1 Restore authority

- Depends on: none; the human correction is authoritative.
- Deliverables: corrected original phased plan, corrected extracted Phase 2 plan, corrected active
  progress plan, and an exact residual inventory.
- Workstreams: one serial authority lane.
- Entry criteria: human correction recorded in this plan.
- Exit criteria: current plans contain no Instructor grading/regrading action and explicitly define
  connectivity recovery as expiry auto-submission of the saved responses.
- Parallel-plan ready: no; every later package depends on this semantic boundary.

### Milestone: R2 Remove invalid capability

- Depends on: R1, because schema and API removal must follow the corrected authority.
- Deliverables: corrected state machines, SQL capabilities and grants, Rust contracts/adapters,
  generated contracts, and focused tests.
- Workstreams: one serial schema-to-contract lane because the SQL signatures and Rust adapters share
  the same boundary.
- Entry criteria: R1 exit criteria pass.
- Exit criteria: no manual Retry symbol or generation-bump re-entry remains; worker requeue and
  immutable terminal results pass focused Rust and PostgreSQL gates.
- Parallel-plan ready: no; shared schema signatures and generated artifacts require one owner.

### Milestone: R3 Rebuild evidence

- Depends on: R2, because documentation and aggregate evidence must describe the corrected tree.
- Deliverables: current documentation and changelog correction, valid fast-check launcher, generated
  contract freshness, focused PostgreSQL evidence, and final aggregate receipt.
- Workstreams: one serial integration lane.
- Entry criteria: R2 focused gates pass.
- Exit criteria: residual searches classify every remaining Retry occurrence as unrelated or
  automatic unfinished-work handling; fast and full aggregate gates exit 0.
- Parallel-plan ready: no; one final tree and one aggregate verification gate are required.

## Work packages

### Work package: WP-RST1 Correct current planning authority

- Owner: primary implementation agent.
- Touch points: `docs/active_plans/active/production_release_readiness_phase_plan.md`,
  `docs/active_plans/active/student_decisions_and_grading_trust_plan.md`, and `plan-phase_2.md`.
- Depends on: none.
- Acceptance criteria: the plans distinguish Attempt recovery from automatic grading; remove
  Instructor Retry from scope, milestones, workstreams, packages, walkthroughs, risks, tests, and
  documentation instructions; retain unrelated persistence and cleanup retry language.
- Evidence or review: contextual case-insensitive Retry inventory with every remaining match
  classified.
- Obvious follow-ons: WP-RST2.

### Work package: WP-RST2 Restore the one-operation grading state machine

- Owner: primary implementation agent with PostgreSQL review.
- Touch points: `schemas/base_schema/jobs.sql`, `schemas/base_schema/grading.sql`,
  `schemas/base_schema/grading_access.sql`, `schemas/base_schema/install.sql`, and the baseline
  security catalog.
- Depends on: WP-RST1.
- Acceptance criteria: `failed -> ready` generation-bump and
  `instructor_attention -> pending` are rejected; manual Retry functions/helpers/grants are absent;
  Job generation returns to its fixed pre-regrading contract; a result remains unique and immutable;
  a transient pre-result infrastructure failure can only return the same unfinished leased Job to
  ready.
- Evidence or review: focused fresh PostgreSQL baseline and connected lifecycle/security tests.
- Obvious follow-ons: WP-RST3.

### Work package: WP-RST3 Remove mutation and generation from Instructor contracts

- Owner: primary implementation agent.
- Touch points: `crates/browser-api-contract/src/instructor_grading.rs`, LDA Instructor grading
  trait/Postgres adapter/exports/fakes, grading lease contracts and adapters, generated TypeScript.
- Depends on: WP-RST2.
- Acceptance criteria: no `InstructorGradingRetry*`, `retry_question_grading`, expected generation,
  or Instructor grading POST contract remains; read-only detail returns only position and public
  state; worker leases use only the durable ownership facts still required after WP-RST2.
- Evidence or review: Rust generation, format, check, strict Clippy, and focused contract tests.
- Obvious follow-ons: WP-RST4.

### Work package: WP-RST4 Replace drift tests and repair current documentation

- Owner: primary implementation agent.
- Touch points: grading lifecycle/public API/Store connected tests, current contracts and architecture
  docs, lifecycle/failure/database/security docs, current NEWS, active plans, and `docs/CHANGELOG.md`.
- Depends on: WP-RST2 and WP-RST3.
- Acceptance criteria: tests prove final failure cannot reopen, a wrong lease token writes
  nothing, a completed result cannot be replaced, accepted response bytes are unchanged, role-scoped
  readers remain answer-free, and no human grading capability exists. Current prose states the same.
- Evidence or review: focused tests plus residual searches; dated archives remain unchanged.
- Obvious follow-ons: WP-RST5.

### Work package: WP-RST5 Add the repository fast offline gate

- Owner: primary implementation agent.
- Touch points: `launchers/run_fast_checks.sh`, `docs/DEVELOPMENT.md`, and test-front-door references
  if documentation requires them.
- Depends on: none for script adaptation; final receipt depends on WP-RST4.
- Acceptance criteria: the launcher runs `check_rust.sh`, `check_codebase.sh`, and
  `source source_me.sh && python3 -m pytest tests/` in the repository root, invokes no foreign file,
  starts no live stack, and reports a clear pass/fail result.
- Evidence or review: `bash -n` and one successful execution on the corrected tree.
- Obvious follow-ons: WP-RST6.

### Work package: WP-RST6 Revalidate the corrected tree

- Owner: primary implementation agent.
- Touch points: no new product surface; evidence and changelog only.
- Depends on: WP-RST4 and WP-RST5.
- Acceptance criteria: focused grading gates pass, `launchers/run_fast_checks.sh` exits 0, then the
  exact `source source_me.sh && ./launchers/all_test.sh` aggregate exits 0 on the same material tree.
- Evidence or review: command, exit status, stable test counts, and explicit note that the earlier
  pre-reset green run was diagnostic only.
- Obvious follow-ons: resume the corrected Phase 2 package after the reset is accepted.

## Acceptance criteria and gates

- Authority gate: Human Guidance contains the approved Assignment Attempt recovery bullets and no
  Instructor grading/regrading claim.
- Negative capability gate: no current source defines `retry_grading_for_question_attempt`,
  `retry_grade_accepted_submission`, `InstructorGradingRetry`, or `retry_question_grading`.
- State-machine gate: final and completed grading states cannot return to pending/ready; automatic
  no-result requeue stays within the same unfinished Job.
- Projection gate: Student and Instructor readers remain answer-free and expose no worker lease,
  private failure detail, Job identity, or mutation fence.
- Documentation gate: current authorities and active plans never use connectivity recovery as
  justification for regrading.
- Fast integration gate: `source source_me.sh && ./launchers/run_fast_checks.sh` exits 0.
- Full integration gate: `source source_me.sh && ./launchers/all_test.sh` exits 0 after the reset.

## Test and verification strategy

- Run formatter and compiler checks immediately after schema/Rust signature removal.
- Regenerate Rust-owned TypeScript declarations before browser/Node checks and verify idempotence.
- Use connected PostgreSQL tests for transition rejection, function/grant absence, worker requeue,
  terminal failure, immutable response bytes, and immutable unique result evidence.
- Keep permanent tests deterministic and behavior-focused. Do not preserve tests whose only purpose
  was the invalid manual Retry workflow.
- Use `launchers/run_fast_checks.sh` for routine offline integration while correcting the reset.
- Run the exact full aggregate once the material tree is stable; do not repeatedly start disposable
  services for intermediate syntax or type failures.
- Treat any remaining human-triggered grading action, state re-entry, or generated Retry DTO as a
  blocking failure even if all automated tests are green.

## Risk register

| Risk                                                                    | Impact                                               | Trigger                                                     | Owner           | Mitigation                                                                   |
| ----------------------------------------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------- |
| Partial removal leaves a callable SQL capability                        | Instructors retain an unintended grading action      | Function, grant, helper, or route survives                  | WP-RST2 owner   | Catalog absence assertions and exact symbol search                           |
| Removing generation breaks valid worker ownership fencing               | Stale lease could commit                             | Generation is removed without preserving token/state checks | WP-RST2/3 owner | Compare with pre-drift schema; retain exact lease-token and Job-state checks |
| Automatic no-result requeue is mistaken for regrading                   | Valid outage handling is deleted or docs drift again | Broad replacement of every Retry word                       | WP-RST1/4 owner | Classify occurrences by whether a Grading Result already exists              |
| Read-only attention status is mistaken for Instructor grading authority | UI copy implies an action that does not exist        | "review grading" or actionable wording remains              | WP-RST4 owner   | Status-only copy and negative route/capability tests                         |
| Copied fast launcher runs the wrong repository workflow                 | False confidence or missing generated checks         | Foreign build command remains                               | WP-RST5 owner   | Derive it directly from `all_test.sh` minus live acceptance                  |
| Earlier green aggregate is reported as corrected acceptance             | False closeout evidence                              | Pre-reset receipt reused                                    | WP-RST6 owner   | Label it diagnostic and rerun after residual gates pass                      |

## Documentation close-out requirements

- Active plan / progress tracker: record each reset package and update the Phase 2 next package only
  after the reset aggregate passes.
- `docs/CHANGELOG.md` entry: correct the same-day uncommitted Retry claims rather than preserving them
  as accepted behavior, and record the authority drift and removal.
- Current durable docs: define Assignment Attempt recovery as expiry auto-submission and describe
  deterministic automatic grading, automatic unfinished-Job requeue, terminal failure, and
  read-only Instructor status consistently.
- Archive / closure notes: move this plan to `docs/archive/` with `git mv` only after all reset gates
  pass and the corrected Phase 2 plan is again the active source of truth.

## Patch plan and reporting format

- Patch R1: correct the original and extracted plans; publish the classified residual inventory.
- Patch R2: remove schema/API Retry and restore the one-operation state machine.
- Patch R3: remove Rust/browser mutation and generation contracts; regenerate declarations.
- Patch R4: replace drift tests and repair current documentation/changelog.
- Patch R5: adapt and run the fast offline launcher.
- Patch R6: run focused PostgreSQL evidence and final `all_test.sh`; close the reset plan.

Each report states the patch ID, exact changed boundary, retained unrelated retry behavior, focused
gate, residual search result, and next package. No report calls a pre-result worker requeue a regrade.

## Open questions and decisions needed

- Non-blocking follow-up: decide separately whether the read-only public
  `needsInstructorAttention` vocabulary should become a neutral system-failure term. This reset
  removes all Instructor action without expanding into that product-language change.
