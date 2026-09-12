# Plan: production release-readiness phases

Status: approved planning scope; implementation has not started under this plan.

Primary source: `plan-production_release_readiness_2026_09_12.md`

Related active authority:
`docs/active_plans/active/webwork_opaque_backend_plan.md`

## Context

The September 12 production-readiness review identified ten areas where the
current product can mislead an Instructor or Student, hide a working path, or
turn a narrow backend failure into a visible product failure. This plan turns
those findings into bounded implementation phases.

Item 3 is already in progress under the active opaque WeBWorK backend plan. That
plan supersedes the readiness report's older radio- and matching-specific
framing. This plan neither duplicates nor redirects that implementation. It
tracks only the interface checkpoints needed by the other readiness work and
the final Fall teaching walkthrough that accepts item 3 for release.

The remaining work belongs primarily to two product layers:

- the PLE backend, including Rust services, workers, Stores, Object Store
  interfaces, schema, and data access where necessary; and
- the browser UI, including TypeScript/SolidJS pages, navigation, validation,
  persistence feedback, preview, and user-visible recovery.

Provider-specific infrastructure is outside this plan. AWS, OpenTofu, Podman,
container orchestration, and provider backup tooling are not product
implementation targets. Existing disposable environments may still be used as
test substrates, but passing such a test makes a PLE application-contract claim,
not a provider-readiness claim.

The plan also incorporates these product decisions:

- valid simple Assignment policy changes autosave without an undo history;
- structural Assignment Question edits retain an explicit Save boundary and
  dirty-navigation protection;
- Instructor Student View combines the existing Student-view scenario and
  policy evaluation with the real Student delivery path;
- `StudentViewScenario` remains the policy/context model rather than becoming a
  second Question renderer;
- item 9 exposes only the archive and restore operations needed by the current
  content owner; and
- item 10 is limited to provider-neutral application invariants and cleanup.

## Objectives

- Ensure the state visible to an Instructor is either durably saved or clearly
  identified as invalid, saving, failed, conflicted, or structurally unsaved.
- Make every supported Assignment editing page reachable through the ordinary
  Ribbon after its persistence behavior is safe.
- Give Students enough server-authoritative availability, deadline, and grading
  information to decide what to do and to trust that accepted work is retained.
- Give Instructors a real, non-mutating view of the Student Assignment experience
  under a selected or hypothetical Student context.
- Expose narrow archive and restore actions already supported by the backend.
- Make Course Media cleanup, application-role health, restart behavior, and
  restore verification explicit without coupling the server to an
  infrastructure provider.
- Preserve the opaque WeBWorK backend plan as the sole implementation authority
  for item 3 and use a real Fall teaching walkthrough as its readiness gate.

## Design philosophy

- Give one layer clear ownership of each decision. The browser owns local form
  state and human feedback; the server owns authorization, authoritative time,
  validation, concurrency control, and persisted truth.
- Prefer truthful, narrow projections over partially populated aggregates or
  browser inference.
- Distinguish accepted Student work from later grading progress. A renderer or
  worker failure must not make an accepted submission look lost.
- Reuse real Student delivery components for Instructor preview while keeping
  preview read-only and free of Student Work side effects.
- Parallelize only workstreams with separate contracts and file ownership.
  Shared generated models, Student projections, and grading lifecycle changes
  receive an explicit integration checkpoint before downstream work begins.
- Verify behavior at the lowest durable layer, then use focused semantic browser
  replay for user-visible paths. Pixel equality is not an acceptance gate.
- Treat the repository's current source and tests as implementation evidence.
  The readiness report supplies findings and intent, not immutable code-level
  instructions.

## Scope

- Add a policies-only Assignment update contract and valid-change autosave UI.
- Protect explicit-save structural Assignment edits from navigation loss.
- Return and cache a coherent Course Appearance after theme changes.
- Render intentionally unavailable management routes without dead API calls.
- Admit working Assignment Overview and Questions destinations to the Ribbon.
- Project and display server-authoritative Student availability and deadline
  information with an explicit IANA timezone.
- Model and display public grading states, remove the unconditional worker delay,
  and add authorized retry of accepted work.
- Run the Fall WeBWorK teaching walkthrough after the active opaque-backend plan
  reaches its browser and grading checkpoint.
- Build Instructor Student View from Student-view scenario evaluation plus the
  actual answer-free Student presentation path.
- Expose Question and Blueprint archive and restore actions.
- Discover and clean expired staged Course Banner uploads through provider-neutral
  Store and Object Store contracts.
- Define provider-neutral role configuration, health, restart, recovery, and
  application restore-verification behavior.
- Update affected contracts, architecture documentation, tests, and changelog
  entries as each bounded patch lands.

## Non-goals

- Reimplement, fork, or create a second plan for the opaque WeBWorK backend work.
- Add PLE-native knowledge of arbitrary PG controls, responses, or grading
  behavior.
- Implement AWS, OpenTofu, Podman, Kubernetes, container-service definitions,
  provider storage policy, or provider backup creation.
- Claim complete production infrastructure readiness from application tests.
- Add undo history for simple Assignment policy changes.
- Autosave Question ordering, Question addition/removal, or other structural
  Assignment edits.
- Build published Question revision editing, collaboration, proposals, watchers,
  stewardship, or a general content-governance system.
- Implement Grade Settings or Teaching Operations capabilities for this release.
- Rename `StudentViewScenarioAdmission` or other canonical domain terminology as
  part of Student View delivery. The browser must use direct language such as
  `Can start` or `Cannot start`.
- Create a second preview-only Question renderer or persist synthetic preview
  Attempts.
- Expose private grading Job IDs, response internals, correct answers, renderer
  diagnostics, or storage addresses through public status APIs.

## Current state summary

| Item | Current interpretation | Plan status |
| --- | --- | --- |
| 1 | Simple policies can display unsaved values while Release uses saved state; structural edits can be lost on navigation. | Phase 1, milestone 1.1 |
| 2 | Assignment pages work but the Ribbon capability registry hides supported destinations. | Phase 1, milestone 1.4 |
| 3 | The older bounded-control implementation is being replaced by an opaque backend-owned interaction. | In progress in the separate active plan |
| 4 | Student-view scenario and policy modeling exists, but the route, store, and real learner presentation are incomplete. | Phase 3, milestones 3.1-3.3 |
| 5 | Policy timestamps exist but the Student projection and presentation do not provide a complete decision summary. | Phase 2, milestone 2.1 |
| 6 | Submission and grading states are not sufficiently distinct, recovery is weak, and the worker has an avoidable post-claim delay. | Phase 2, milestones 2.2-2.3 |
| 7 | A theme update can return an incomplete Course Appearance and temporarily hide an existing banner. | Phase 1, milestone 1.2 |
| 8 | Hidden management routes can still call nonexistent endpoints and look broken when visited directly. | Phase 1, milestone 1.3 |
| 9 | Backend archive and restore actions exist without a narrow owner-facing browser workflow. | Phase 4, milestone 4.1 |
| 10 | Staged media cleanup and application recovery contracts are incomplete or implicit. | Phase 4, milestone 4.2 |

`StudentViewScenario` is interpreted as:

> Preview this Assignment as this Student, under these Student circumstances,
> at this selected moment.

It supplies the policy and access half of item 4. It does not itself own actual
Question presentation.

## Architecture boundaries and ownership

### Browser UI

The browser owns local input state, immediate syntax validation, visible
persistence status, navigation guards for explicit-save documents, timezone-aware
formatting, and accessible presentation. It does not decide authoritative
Assignment access, grading state transitions, retry eligibility, or object
cleanup.

### PLE service boundary

The Rust service owns authorization, request validation, cross-field policy
validation, ETag comparison, server-time decisions, public projections, and the
separation between preview and Student Work mutation. Every update or operation
is re-authorized at the object or function level (ASVS 8.1.1-8.1.2,
8.2.1-8.2.3, 8.3.1).

### Store and database boundary

Stores own transactional persistence, immutable accepted-response references,
grading operation generations, leases, idempotency, and typed cleanup claims.
The schema changes only when the current model cannot represent a required
invariant. Client-supplied fields are allowlisted and validated at the service
and persistence boundaries (ASVS 1.5.2, 2.1.1-2.1.3, 2.2.1-2.2.3,
2.3.1-2.3.3).

### Backend-owned Question presentation boundary

The active opaque WeBWorK plan owns backend identity, opaque interaction
documents, response forwarding, grading, and embedded presentation. This plan
consumes that boundary only after the following checkpoints:

- **WW-C1 - shared contract stable:** the backend-owned presentation and public
  Student projection types are stable enough for deadline and preview work;
- **WW-C2 - delivery and grading integrated:** the browser delivery path and
  grading worker use the opaque contract end to end; and
- **WW-C3 - active plan accepted:** architecture tests and the active plan's
  focused end-to-end gates pass.

### Provider-neutral operational boundary

PLE can define process roles, required configuration, health, cleanup claims,
idempotent object operations, and verification of a restored equivalent data
set. A deployment system chooses how processes run, where PostgreSQL and object
storage live, and how backups are produced.

### Mapping

| Phase | Milestone | Readiness item | Workstreams | Primary ownership |
| --- | --- | --- | --- | --- |
| 1 - Instructor safety and truthful UI | 1.1 Assignment persistence safety | 1 | A | Browser UI and narrow Rust API |
| 1 - Instructor safety and truthful UI | 1.2 Course Appearance coherence | 7 | B | Rust API and browser cache |
| 1 - Instructor safety and truthful UI | 1.3 Intentional unavailable routes | 8 | C | Browser UI |
| 1 - Instructor safety and truthful UI | 1.4 Assignment Ribbon navigation | 2 | D | Browser UI and generated Ribbon ledger |
| 2 - Student decisions and grading trust | 2.1 Student deadline and access decisions | 5 | E | Rust projection and browser UI |
| 2 - Student decisions and grading trust | 2.2 Grading lifecycle foundation | 6 | F | Rust domain, Store, and schema if needed |
| 2 - Student decisions and grading trust | 2.3 Grading responsiveness and recovery | 6 | G, H, I | Rust worker/API and browser UI |
| 2 - Student decisions and grading trust | 2.4 Fall teaching walkthrough | 3 acceptance | J | Teaching and real-content evidence |
| 3 - Instructor Student View | 3.1 Scenario delivery contract | 4 | K | Rust preview projection |
| 3 - Instructor Student View | 3.2 Real preview presentation | 4 | K | Rust service and shared presentation |
| 3 - Instructor Student View | 3.3 Instructor browser experience | 4 | K | Browser UI and non-mutation evidence |
| 4 - Owner and operational closure | 4.1 Authoring archive and restore | 9 | L | Browser UI and existing APIs |
| 4 - Owner and operational closure | 4.2 Provider-neutral Course Media operations | 10 | M | Rust Store, worker, and Object Store |

## Milestone plan

The four phases are the units of implementation and close-out. Do not begin the
next phase until the current phase's exit gate passes. The separate item 3
implementation lane may continue throughout; its work joins this plan only at
milestone 2.4.

| Phase | Priority | Included items | Internal milestone order | Safe concurrency |
| --- | --- | --- | --- | --- |
| 1 - Instructor safety and truthful UI | P0 | 1, 7, 8, then 2 | 1.1/1.2/1.3 in parallel; 1.4 after 1.1 | Maximum three streams, then one navigation patch |
| 2 - Student decisions and grading trust | P1 | 5, 6, and item 3 acceptance | 2.1; 2.2; 2.3; 2.4 | Within 2.3, G/H/I may run in parallel after F |
| 3 - Instructor Student View | P1 | 4 | 3.1; 3.2; 3.3 | Integrated contract first; tests may split later |
| 4 - Owner and operational closure | P2 | 9 and 10 | 4.1 and 4.2 in parallel | Maximum two streams, then one recovery gate |

### Phase 1 - Instructor safety and truthful UI

**Phase depends on:** no readiness phase. Coordinate generated TypeScript only
if the active WeBWorK lane is regenerating the same contract files.

**Phase deliverable:** Instructor editing and visible management surfaces do not
misrepresent what is saved, releasable, or supported.

#### Milestone 1.1 - Assignment persistence safety

Implement valid-change Assignment policy autosave, explicit persistence states,
Release interlocks, and structural-edit dirty navigation protection through
workstream A.

**Entry criteria:** current focused Assignment tests are green or their
pre-existing failures are recorded.

**Exit criteria:** every affected page distinguishes saved, saving, invalid,
failed, conflicted, or structurally dirty state; Release cannot use a known-stale
visible policy form.

#### Milestone 1.2 - Course Appearance coherence

Make theme updates return and cache a coherent Course Appearance, including an
existing Banner, through workstream B.

**Entry criteria:** current focused Course Appearance tests are green or their
pre-existing failures are recorded.

**Exit criteria:** saving a theme does not temporarily hide an existing Banner.

#### Milestone 1.3 - Intentional unavailable routes

Make Grade Settings and Teaching Operations direct routes render an intentional
unavailable state without mounting clients for unsupported endpoints through
workstream C.

**Entry criteria:** the current capability registry and direct routes have been
verified.

**Exit criteria:** direct navigation is truthful and generates no unsupported
management request.

Milestones 1.1, 1.2, and 1.3 may run in parallel with exclusive feature-module
ownership.

#### Milestone 1.4 - Assignment Ribbon navigation

After milestone 1.1, admit working Assignment Overview and Questions
destinations through workstream D. Regenerate the navigation ledger from its
owner rather than hand-editing derived artifacts.

**Entry criteria:** milestone 1.1 passes, so added navigation cannot introduce a
new path for losing structural edits or releasing stale policies.

**Exit criteria:** an Instructor can move through Questions, Policies, Overview,
and Course Assignments using normal navigation, with dirty protection preserved.

**Phase 1 exit gate:** milestones 1.1 through 1.4 pass their focused checks and a
shared browser flow. Phase 2 is then unblocked.

### Phase 2 - Student decisions and grading trust

**Phase depends on:** Phase 1 and the relevant active WeBWorK checkpoints.

**Phase deliverable:** Students can decide whether and when to work, trust that
their submission was accepted, understand grading progress, and rely on
Instructor recovery when grading fails. Item 3 receives its release-readiness
acceptance here.

#### Milestone 2.1 - Student deadline and access decisions

After WW-C1, use workstream E to provide one server-owned Student Assignment
decision summary containing available, due, and close instants, the Student's
effective IANA timezone, time-limit information, and the server's current start
decision. Display it consistently on the Student Course landing and pre-start
Assignment page.

**Entry criteria:** Phase 1 passes and the shared backend-owned presentation
types have reached WW-C1.

**Exit criteria:** before starting, a Student can determine whether work can be
started, when it is due, when it closes, and whether a time limit begins at
start. The browser formats instants but does not recompute authorization.

#### Milestone 2.2 - Grading lifecycle foundation

After WW-C2, use workstream F to establish the public grading operation states,
immutable accepted-response reference, retry invariants, generation fencing,
and any minimal Store or schema changes.

**Entry criteria:** milestone 2.1 passes and a submitted backend-owned response
can travel through the new WeBWorK delivery and worker boundary.

**Exit criteria:** duplicate or stale grading operations cannot overwrite a
newer result, and Retry is defined over the exact accepted response.

#### Milestone 2.3 - Grading responsiveness and recovery

After milestone 2.2, workstreams G, H, and I may run in parallel with separate
worker, Student UI, and Instructor UI/API ownership. Remove the unconditional
post-claim delay, distinguish submission acceptance from grading in Student UI,
and add metadata-only Instructor Gradebook status and authorized Retry.

**Entry criteria:** the milestone 2.2 lifecycle contract is integrated.

**Exit criteria:** users can distinguish queued, grading, graded, and needs-
Instructor-attention states; Retry reuses the immutable accepted response; and
worker throughput has no deliberate per-job sleep.

#### Milestone 2.4 - Item 3 Fall teaching walkthrough

After milestone 2.3, use workstream J to accept item 3 with representative real
Fall PG content.

**Entry criteria:** workstreams G through I are integrated and WW-C2 is
satisfied.

**Exit criteria:** correct, incorrect, partial-credit, reload, Gradebook,
renderer-failure, Retry, and final-grade cases pass without adding PLE code for
individual PG control types.

**Phase 2 exit gate:** milestones 2.1 through 2.4 pass, accepted work and grading
remain distinct end to end, and item 3 has teaching-workflow evidence. Phase 3
is then unblocked.

### Phase 3 - Instructor Student View

**Phase depends on:** Phase 2 and WW-C2.

**Phase deliverable:** an Instructor can evaluate access and see the actual
Student-facing Assignment and Questions in one non-mutating view.

#### Milestone 3.1 - Student-view scenario delivery contract

Use workstream K to complete hypothetical and selected Student-view scenario
routes, Store integration, generated types, decoder, and fixtures.

**Entry criteria:** Student deadline, grading, and public state terms are stable.

**Exit criteria:** exact selected-Student and deterministic simulated contexts
are authorized, distinct, and correctly labeled.

#### Milestone 3.2 - Real preview presentation

Compose scenario evaluation with the same native and backend-owned Question
presentation used by Student delivery, in answer-free preview mode.

**Entry criteria:** milestone 3.1 passes and native/backend-owned presentation is
available under WW-C2.

**Exit criteria:** actual Questions render without creating Student Work,
Attempts, responses, grades, or grading Jobs; submission paths are blocked.

#### Milestone 3.3 - Instructor browser experience

Build the browser page with scenario selection, selected moment, effective
policy summary, access decision, Assignment instructions, deadlines, and actual
Question presentation.

**Entry criteria:** milestone 3.2 passes its service and non-mutation checks.

**Exit criteria:** an Instructor can answer both `Could this Student access the
Assignment now?` and `What would the Student actually see?` on one trustworthy
page.

**Phase 3 exit gate:** milestones 3.1 through 3.3 pass focused service, browser,
authorization, disclosure, and non-mutation checks. Phase 4 is then unblocked.

### Phase 4 - Owner and operational closure

**Phase depends on:** Phase 3 and WW-C3. Course Appearance must already return
coherent media state, and application roles must reflect the final backend-owned
worker shape.

**Phase deliverable:** the current content owner can use the needed lifecycle
actions, and narrow provider-neutral application cleanup/recovery invariants are
closed.

#### Milestone 4.1 - Authoring archive and restore

Use workstream L to expose Question and Blueprint archive in their Danger Zones
and ordinary Restore actions through the existing backend handlers.

**Entry criteria:** current archive/restore handlers and authorization contracts
are verified.

**Exit criteria:** Neil can archive and restore the supported content lineages
while exact published Revision references remain valid.

#### Milestone 4.2 - Provider-neutral Course Media operations

Use workstream M to add expired staged Banner discovery and cleanup, role-
specific configuration and health, restart/lease recovery, and application
verification of a restored equivalent database/object set.

**Entry criteria:** Course Media staging contracts and the final application
worker roles are verified.

**Exit criteria:** expired unpromoted Banner objects can be cleaned safely; each
process reports whether it can perform its application role; interrupted claims
recover; and an equivalent restored data set passes ordinary migration,
verification, typed record checks, and representative content reads.

Milestones 4.1 and 4.2 may run in parallel with exclusive browser-lifecycle and
backend-operations ownership.

**Phase 4 exit gate:** both milestones and the provider-neutral application
recovery gate pass. All readiness phases are complete.

## Workstream breakdown

### A - Assignment persistence safety

- **Recommended owner:** cross-stack implementation owner.
- **Needs:** current Assignment workspace ETag, policy validation, release
  contract, editor stores, and route-leave behavior.
- **Provides:** policies-only update, autosave controller, persistence states,
  release interlock, and structural dirty guard.
- **Review boundary:** no generalized form framework and no structural autosave.

### B - Course Appearance coherence

- **Recommended owner:** narrow Rust/API owner with browser cache verification.
- **Needs:** current theme update response and Course Appearance Store aggregate.
- **Provides:** truthful full aggregate after a theme change.
- **Review boundary:** no media upload redesign.

### C - Intentional unavailable routes

- **Recommended owner:** browser UI owner.
- **Needs:** Ribbon capability registry, route definitions, and current Grade
  Settings and Teaching Operations pages.
- **Provides:** direct-route unavailable states without unsupported network calls.
- **Review boundary:** no management backend implementation.

### D - Ribbon Assignment navigation

- **Recommended owner:** browser navigation owner.
- **Needs:** accepted milestone 1.1 editing behavior and Ribbon generator.
- **Provides:** admitted destinations, correct selection, and regenerated ledger.
- **Review boundary:** no unrelated Ribbon redesign.

### E - Student decision summary

- **Recommended owner:** cross-stack projection owner.
- **Needs:** Assignment policies, effective Student context, server clock,
  timezone source, and WW-C1.
- **Provides:** one authoritative decision DTO and two consistent browser uses.
- **Review boundary:** the browser formats decisions but never grants access.

### F - Grading lifecycle contract

- **Recommended owner:** Rust Store/domain owner.
- **Needs:** current job, accepted response, grade, failure, and retry models.
- **Provides:** public states, private operation generations, retry invariants,
  and any minimal schema changes.
- **Review boundary:** no renderer-specific public status and no raw Job identity.

### G - Worker responsiveness

- **Recommended owner:** Rust worker owner.
- **Needs:** F and current shutdown/cancellation behavior.
- **Provides:** long-lived wait signal, prompt claims, and clean shutdown.
- **Review boundary:** no speculative distributed scheduler.

### H - Student grading status

- **Recommended owner:** Student browser owner.
- **Needs:** F public projection.
- **Provides:** accepted-submission confirmation and queued/grading/graded/needs-
  attention presentation.
- **Review boundary:** no private diagnostic disclosure.

### I - Instructor grading recovery

- **Recommended owner:** Gradebook/API owner.
- **Needs:** F retry and authorization contracts.
- **Provides:** metadata-only status, actionable failure state, and Retry grading.
- **Review boundary:** retry creates a new fenced operation over the same accepted
  response rather than accepting an edited replacement.

### J - Fall WeBWorK teaching walkthrough

- **Recommended owner:** acceptance/test owner with Instructor teaching review.
- **Needs:** WW-C2 and integrated F through I.
- **Provides:** real-content evidence for item 3 and the shared grading path.
- **Review boundary:** diverse PG content must work without PLE control-specific
  code changes.

### K - Instructor Student View

- **Recommended owner:** integrated preview owner.
- **Needs:** `StudentViewScenario`, Student decision summary, native presentation,
  opaque backend-owned presentation, and disclosure rules.
- **Provides:** selected and hypothetical preview with non-mutation proof.
- **Review boundary:** preview documents are least-data, identity-free where the
  client does not need identity, short-lived, and protected from storage or URL
  leakage (ASVS 3.2.1-3.2.2, 3.4.3-3.4.6, 3.5.5).

### L - Narrow authoring lifecycle UI

- **Recommended owner:** authoring browser owner.
- **Needs:** existing Question and Blueprint archive/restore handlers.
- **Provides:** Danger Zone archive actions and ordinary Restore actions.
- **Review boundary:** no published revision editor or collaboration surface.

### M - Provider-neutral Course Media operations

- **Recommended owner:** Rust Store/Object Store owner.
- **Needs:** staged Banner lifecycle, expiry records, object receipts, role
  configuration, leases, and verification commands.
- **Provides:** bounded cleanup worker, health contract, restart recovery, and
  restore-verification procedure.
- **Review boundary:** addresses are typed and server-derived; cleanup accepts no
  client path and prevents path traversal or unauthorized file selection
  (ASVS 5.3.1-5.3.2).

## Work packages

### WP-A1 - Autosave simple Assignment policies

Add a policies-only operation such as
`PUT /api/course-instances/{course}/assignments/{assignment}/policies` with
`If-Match`. The server validates the combined policy slice, preserves title and
Questions, commits atomically, and returns the complete Assignment workspace
plus a new ETag.

In the browser:

- save valid select, checkbox, committed date/time, and committed numeric
  changes immediately;
- save longer policy text, such as instructions, after a short idle interval or
  blur;
- retain invalid input locally, show its validation error, and send no request;
- serialize requests so only one is in flight, coalescing later valid changes;
- show `Saving`, `Saved`, `Invalid`, `Save failed`, or `Conflict` explicitly;
- keep the user's visible value after a failure or conflict;
- prevent Release and Check release while valid changes are still saving or a
  visible change is invalid, failed, or conflicted; and
- provide Retry or Reload server state for failed/conflicted persistence.

There is no undo history. An Instructor restores an earlier date or policy value
by entering it again. Both browser and server validate input, ETags prevent lost
updates, and failure messages remain generic enough not to reveal internals
(ASVS 1.5.2, 2.1.1-2.1.3, 2.2.1-2.2.3, 2.3.1-2.3.3,
3.5.1-3.5.3, 4.1.1).

**Acceptance:** the saved server workspace normally matches every valid value
shown in the form; Release cannot proceed during ambiguous persistence; invalid
dates remain editable and unsent; two rapid changes converge on the last valid
value; a stale ETag never silently overwrites another editor.

### WP-A2 - Protect structural Assignment edits

Keep explicit Save for Question ordering, addition, removal, and other coherent
structural edits. Track dirty state in the owning editor store and intercept
internal route changes, Ribbon changes, browser back/forward, and unload where
the platform permits. Offer Save and continue, Discard and continue, or Stay.
Release remains unavailable while structural changes are dirty.

**Acceptance:** a structural edit cannot disappear through ordinary navigation
without a deliberate discard; successful Save clears the guard; failed Save
does not clear it.

### WP-B1 - Return coherent Course Appearance

Make a theme update return the complete current Course Appearance, including the
existing Banner reference. The browser replaces its aggregate cache only with
that coherent response.

**Acceptance:** saving a theme never hides the current Banner; a reload produces
the same appearance; a Course with no Banner remains correctly banner-free.

### WP-C1 - Make unavailable management routes truthful

Have Grade Settings and Teaching Operations direct routes render an intentional
unavailable state using the same capability authority as the Ribbon. Do not
mount stores or issue requests to endpoints that are not live.

**Acceptance:** direct navigation is stable, accessible, and makes zero
unsupported management API calls; supported live feature failures retain their
normal error behavior and are not mislabeled unavailable.

### WP-D1 - Admit supported Assignment destinations

Update the owning capability registry and generator inputs for Assignment
Overview and Questions. Regenerate the Ribbon contract or ledger. Verify current
selection, scope preservation, breadcrumbs, keyboard focus, and return paths.

**Acceptance:** Questions, Policies, Overview, and Course Assignments are
reachable in the expected sequence without typed URLs or browser history.

### WP-E1 - Project the Student decision summary

Add one Student-facing server projection with:

- availability/open instant;
- due instant;
- close instant;
- time-limit duration or absence;
- effective Student IANA timezone;
- server evaluation instant;
- current server decision and a public reason when start is unavailable; and
- only the accommodation effects the Student is authorized to see.

Use UTC instants at the API boundary and keep the server's start decision
authoritative. The browser must not infer permission from its clock.

**Acceptance:** boundary cases at open, due, and close instants are deterministic;
the same decision appears on Student landing and pre-start pages; unauthorized
Student or accommodation data is absent.

### WP-E2 - Present deadlines for decisions

Format each instant in the supplied IANA timezone with a human label and timezone
abbreviation or name. Distinguish due from close, and explain the time limit
before Start.

**Acceptance:** a Student can answer `Can I start now?`, `When is it due?`,
`When does it close?`, and `What timer starts when I begin?` without interpreting
raw ISO strings.

### WP-F1 - Establish grading operation invariants

Define public states as `queued`, `grading`, `graded`, and
`needsInstructorAttention`. Keep worker claims, failure detail, generation, and
Job identity private. Retain the accepted response independently of every grade
operation. A Retry creates or reactivates an authorized operation for that exact
response, uses generation fencing, and cannot let an older worker overwrite a
newer terminal result.

**Acceptance:** duplicate submissions or retries converge safely; stale workers
cannot win; completed grades remain stable; schema additions, if any, express
invariants rather than UI history.

### WP-G1 - Remove avoidable grading delay

Replace the unconditional post-claim sleep in `worker.rs` with a long-lived
cancellation/wakeup mechanism. When work exists, begin the next eligible grade
immediately unless shutdown wins. When no work exists, wait efficiently and
remain responsive to shutdown.

**Acceptance:** focused timing evidence shows no deliberate three-second floor
between trivial jobs; idle operation does not busy-spin; shutdown remains
bounded and deterministic.

### WP-H1 - Explain grading to Students

Show accepted submission state separately from grading state. A failed grading
operation says that the response was accepted and that Instructor attention is
needed. Do not expose retry controls to Students unless a separate product
decision grants that capability.

**Acceptance:** every accepted submission remains visibly accepted while queued,
grading, or failed; graded work follows existing disclosure policy.

### WP-I1 - Add Instructor grading recovery

Project metadata-only grading status in the Gradebook and add an authorized
Retry grading action for `needsInstructorAttention`. Confirm the target using
public Assignment/Student/Attempt context, not a raw Job ID. Return the new public
state after the operation.

**Acceptance:** an Instructor with the required Course authority can retry the
exact accepted response; another Course's Instructor cannot; repeated Retry is
idempotent or returns a clear current-operation result.

### WP-J1 - Accept item 3 through the Fall teaching walkthrough

Use representative real Fall PG content, including interaction diversity that
the old radio/matching projection could not generalize. Exercise render,
correct/incorrect/partial response, reload where supported, submission, PG
grading, Student state/history, Instructor Gradebook, renderer failure, Retry,
and final grade.

**Acceptance:** the walkthrough succeeds without adding PLE code for individual
PG control types. Record content identifiers, seeds, observable outcomes, and
sanitized failure/recovery evidence without copying private answers into logs.

### WP-K1 - Complete Student-view scenario delivery

Add the route, Store integration, generated types, decoder, and fixtures for
hypothetical and selected Student scenarios. Use the exact existing Attempt
context when one is selected and authorized. Otherwise use a deterministic
sample context labeled `Simulated preview`, including selected moment, prior
Attempt count, resolved policies, accommodation comparison, and access/start
decision.

**Acceptance:** selected and hypothetical cases are visibly distinct; membership
and Course authorization are enforced; response data carries no Student identity
that the browser does not need.

### WP-K2 - Compose preview with real presentation

Compose scenario evaluation with the same native and backend-owned presentation
components used by Student delivery. Request answer-free, feedback-safe content
under a preview-specific server operation. Do not create Student Work, Attempts,
responses, grades, or grading Jobs. Block all submission bridges and controls.

**Acceptance:** native and diverse WeBWorK Questions render through their real
components; pre/post row and object counts prove non-mutation; no correct answer
or undisclosed feedback crosses the boundary.

### WP-K3 - Present Instructor `View as Student`

Build the Instructor page with scenario selection, selected moment, effective
policy summary, access decision, Assignment instructions, deadlines, and actual
Question presentation. Use direct UI language such as `Can start`, `Cannot
start`, `Selected Student`, and `Simulated preview`.

**Acceptance:** an Instructor can answer both `Could this Student access the
Assignment now?` and `What would the Student actually see?` on one trustworthy
page.

### WP-L1 - Expose Question archive and restore

Add archive to the Question Danger Zone and Restore to archived Question views,
using the existing backend handler and ETag/authorization conventions.

**Acceptance:** archived Questions leave ordinary selection while pinned
Revisions remain resolvable; Restore returns the lineage to ordinary selection.

### WP-L2 - Expose Blueprint archive and restore

Add the equivalent narrow Blueprint actions without expanding into collaboration
or revision editing.

**Acceptance:** current owner workflows can archive and restore a Blueprint;
existing published Revision references remain valid.

### WP-M1 - Claim expired staged Banner cleanup safely

Add a Store operation that claims a bounded batch of expired, unpromoted staged
Banner uploads under a lease. Return typed server-derived object subjects and
expected receipts/checksums. Never accept an arbitrary object key or path from a
browser or operator request.

**Acceptance:** active and promoted objects are never claimable; concurrent
workers cannot both own a live claim; expired abandoned objects are eventually
eligible after lease recovery.

### WP-M2 - Run provider-neutral cleanup and health

Add a dedicated Course Media cleanup worker using the existing Object Store
interface. Record retained, removed, already absent, retryable, and repair-needed
outcomes idempotently. Define the role's minimum configuration and make readiness
reflect database access, object-store capability, and any required schema
identity without disclosing secrets.

**Acceptance:** repeated cleanup converges; transient deletion failure is
retryable; receipt mismatch becomes repair-needed rather than deleting an
uncertain object; missing configuration makes the role unready with a redacted
diagnostic.

### WP-M3 - Verify restart and restored-equivalent invariants

Document and automate the application-owned check for restarting with active or
expired leases and for opening an equivalent restored PostgreSQL/Object Store
set. Run ordinary migration/verification, typed record checks, and representative
content reads. Backup creation and provider recovery orchestration remain outside
the application.

**Acceptance:** interrupted operations recover after their lease rules; an
equivalent restored set is accepted only when database references and object
receipts agree; mismatches fail closed with repair guidance.

## Acceptance criteria and gates

### Product gates

- No Instructor can release an Assignment while the policy values visibly shown
  are known to be invalid, unsaved, failed, conflicted, or structurally dirty.
- No supported Assignment page is hidden from ordinary navigation after Phase 1.
- No intentionally unavailable management route calls a nonexistent endpoint.
- A Student sees server-authoritative open, due, close, timezone, time-limit, and
  current start-decision information before beginning.
- An accepted submission remains visibly accepted across every grading state.
- Instructor Retry operates on immutable accepted work and is authorization-
  checked, fenced, and idempotent.
- Representative Fall WeBWorK content completes the opaque presentation and
  grading lifecycle without control-specific PLE changes.
- Instructor Student View renders actual native and backend-owned Questions and
  creates no Student Work.
- Question and Blueprint archive/restore preserve existing exact Revision
  references.
- Course Media cleanup cannot select promoted, unexpired, or arbitrarily named
  objects and recovers after interruption.

### Integration gates

- Generated Rust, TypeScript, decoder, schema, and fixture surfaces agree.
- New state transitions are tested at their lowest authoritative layer.
- Public DTOs contain no private IDs, answers, renderer internals, storage
  addresses, or cross-Course data.
- ETag, transaction, authorization, generation, and lease races have deterministic
  tests where the outcome matters.
- Browser acceptance uses built application code and real role boundaries.

### Release gate

Phases 1 through 4 must be complete, item 3 must reach WW-C3, the Fall teaching
walkthrough must pass, and the aggregate repository gate must be green. Any
provider deployment decision is a separate release-management concern.

## Test and verification strategy

### Focused implementation checks

- Run the narrow Rust package or test target for each service, Store, worker, or
  schema change before the repository Rust gate.
- Run focused TypeScript/Node tests for stores, decoders, components, and Ribbon
  contracts before the browser/build gate.
- Add deterministic concurrency tests for autosave ETag conflict, grading
  generation fencing, Retry idempotency, and cleanup leasing.
- Use a controllable clock for policy, expiry, grading, and lease boundaries.
- Prove preview non-mutation using before/after Student Work, Attempt, response,
  grade, Job, and object counts rather than relying on UI observation.

### Repository gates

Use the current repository commands documented at implementation time. Expected
top-level gates are:

```bash
./check_rust.sh
./check_codebase.sh
source source_me.sh && ./launchers/all_test.sh
./devel/run_playwright_tests.sh --build
```

Python helpers must run through `source source_me.sh && python3`. If a gate name
or scope changes before a milestone begins, use the current documented command
and update this plan rather than preserving an obsolete invocation.

### Browser evidence

- Phase 1: valid autosave, invalid date, conflict, failure/retry, structural
  leave, theme save with Banner, direct unavailable routes, and Questions to
  Policies to Overview to Course Assignments.
- Phase 2: Student decision display across open/due/close boundaries and two
  IANA timezones; accepted submission through every grading state; Instructor
  Retry; and the Fall WeBWorK walkthrough.
- Phase 3: selected and hypothetical preview with native and backend-owned Questions,
  plus explicit non-mutation evidence.
- Phase 4: archive/restore visibility and application-level cleanup/recovery
  evidence.

### Test lifetime

Keep permanent tests fast, deterministic, offline, and focused on durable
behavior. Treat real-service timing, Fall content walkthroughs, restore drills,
and human visual review as release or cutover evidence unless they protect a
stable contract cheaply. Do not freeze generated file inventories, exact markup,
delays, provider names, or implementation choreography.

## Risk register

| Risk | Trigger or signal | Mitigation | Release consequence |
| --- | --- | --- | --- |
| Collision with active item 3 | Both plans edit generated presentation, response, or browser contract files | Honor WW-C1/WW-C2 checkpoints; assign exclusive file ownership; regenerate once after integration | Pause dependent stream, not item 3 |
| Autosave race or silent overwrite | Responses arrive out of order or a stale editor saves | One in-flight queue, coalescing, ETags, full authoritative response, conflict UI | Blocks milestone 1.1 and Ribbon admission |
| Invalid local value looks saved | Date/time parser accepts partial input or status remains `Saved` | Separate local validity from persistence status; disable release; server revalidation | Blocks release gate |
| Student browser makes access decisions | Client clock or timezone changes the start control | Return server decision and evaluation instant; browser only formats | Blocks milestone 2.1 |
| Grading retry duplicates or overwrites | Repeated Retry or stale worker completion | Immutable response reference, idempotency, operation generation fencing | Blocks Phase 2 |
| Public grading status leaks internals | DTO includes Job ID, private answers, renderer trace, or cross-Course context | Closed metadata-only DTOs and negative authorization/projection tests | Security release blocker |
| Preview mutates real Student state | Delivery helper implicitly creates an Attempt or response | Dedicated preview operation, submission bridge disabled, before/after persistence proof | Blocks Phase 3 |
| Simulated preview is mistaken for exact history | Hypothetical prior Attempt or time is unlabeled | Deterministic sample with persistent `Simulated preview` label | Blocks Phase 3 |
| Cleanup deletes live or foreign data | Raw path/key input, stale claim, or promoted object selected | Typed server-derived subjects, locked bounded claim, receipts, leases, idempotency | Security/data-loss blocker |
| Provider work expands the plan | Infrastructure manifests or provider backup procedures enter patches | Enforce Non-goals and provider-neutral acceptance language | Remove from patch before review |
| Authoring lifecycle expands into governance | Revision editor or collaboration behavior appears in Phase 4 | Limit L to existing archive/restore handlers | Split into a future approved plan |
| Brittle tests accumulate | Tests assert exact HTML, delays, file inventories, or external fixture state | Apply `docs/TEST_EVIDENCE_MODEL.md`; keep only behavior-protecting evidence | Remove brittle test before close-out |

## Rollout and release checklist

### Before implementation

- [ ] Confirm the active WeBWorK plan's current WW-C1/WW-C2/WW-C3 status.
- [ ] Record the current focused gate baseline for the first milestone.
- [ ] Assign exclusive ownership for generated contracts before parallel work.
- [ ] Keep this plan `not started` until explicit implementation guidance is
      received.

### Phase 1

- [ ] Land A, B, and C as separate bounded patches with narrow gates.
- [ ] Run the shared milestones 1.1-1.3 integration/browser gate.
- [ ] Land milestone 1.4 only after Assignment persistence safety passes.
- [ ] Confirm no unsupported route makes a dead API call.

### Phase 2

- [ ] Confirm WW-C1.
- [ ] Complete milestone 2.1, landing the server decision projection before
      browser formatting.
- [ ] Verify open, due, close, time-limit, and timezone behavior.
- [ ] Confirm WW-C2.
- [ ] Complete milestone 2.2 by landing F before G, H, or I.
- [ ] Integrate worker, Student, and Instructor grading streams.
- [ ] Run and record the Fall WeBWorK teaching walkthrough.

### Phase 3

- [ ] Verify selected and hypothetical scenario authorization.
- [ ] Verify native and backend-owned actual presentation.
- [ ] Prove the preview produced no Student Work or grading side effects.

### Phase 4

- [ ] Verify existing Question and Blueprint lifecycle handlers before adding UI.
- [ ] Verify cleanup claim, lease, receipt, idempotency, and health behavior.
- [ ] Run the provider-neutral restart and restored-equivalent check.

### Final release-readiness close-out

- [ ] Confirm Phase 1 through Phase 4 exit criteria.
- [ ] Confirm the active WeBWorK plan reached WW-C3.
- [ ] Run aggregate Rust, browser, Python, and semantic real-role gates.
- [ ] Complete a human teaching review of Student deadlines, grading states,
      WeBWorK delivery, and Instructor Student View.
- [ ] Record remaining findings as explicit future scope rather than weakening
      an acceptance criterion.

## Documentation close-out

- Update `docs/CONTRACTS.md` for the policies-only update, Student decision,
  grading operation, preview, and cleanup contracts that become durable.
- Update `docs/CODE_ARCHITECTURE.md`, `docs/FILE_STRUCTURE.md`, and
  `docs/DATABASE_STRUCTURE.md` only where implementation changes their current
  truth.
- Update `docs/HUMAN_GUIDANCE.md` or `docs/DESIGN_DECISIONS.md` when a completed
  patch establishes a new human or architectural authority.
- Add one bounded `docs/CHANGELOG.md` entry after each completed task and narrow
  gate, following the repository rotation policy.
- Keep `plan-production_release_readiness_2026_09_12.md` as the source review
  artifact. Do not rewrite its historical findings to look implemented.
- When every release gate passes, move this file with `git mv` to
  `docs/active_plans/archive/production_release_readiness_phase_plan.md` and
  record the evidence that closed it.

## Patch plan and reporting

Implement one bounded patch at a time. A milestone may contain parallel patches
only where the milestone section explicitly allows it.

1. **Phase 1:** milestones 1.1, 1.2, and 1.3 independently; milestone 1.4 after
   1.1; integrate and report persistence, truthful-surface, and navigation
   evidence.
2. **Phase 2:** milestone 2.1; milestone 2.2; parallel G/H/I work in milestone
   2.3; milestone 2.4 last; report Student decisions, lifecycle races, worker
   responsiveness, public UI states, and the Fall walkthrough.
3. **Phase 3:** milestones 3.1 through 3.3 in order; report scenario
   authorization, actual presentation, and non-mutation proof.
4. **Phase 4:** milestones 4.1 and 4.2 independently; integrate and report
   lifecycle, cleanup, restart, and restored-equivalent evidence.

Each patch report must state:

- the work package and files owned;
- the behavior completed;
- the focused and aggregate gates run;
- any pre-existing or newly discovered failure;
- whether an external WeBWorK checkpoint changed; and
- the next work package now unblocked.

## Open questions

There are no execution-blocking product questions in this plan. Implementation
must use current source to settle exact route names, component locations, and
whether an invariant requires schema work.

If the active opaque WeBWorK plan changes a shared interface, that active plan
remains authoritative. Update this plan's checkpoint mapping and adapt the
dependent work rather than creating a compatibility layer or duplicate
presentation contract.
