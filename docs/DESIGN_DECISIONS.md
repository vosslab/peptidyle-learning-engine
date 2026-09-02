# Design decisions

<!-- VENDORED HEADER: START -->

Record each durable decision about how this code and repository are shaped, once it is settled, with
the reasoning a later reader needs. Guidance Neil Voss states belongs in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), dated history in `docs/CHANGELOG.md`, open discussion in
`docs/active_plans/decisions/`. [PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

This is PLE's conceptual entrypoint for settled product and architecture decisions. It answers
"why is this boundary here?" and points to the contract that answers "how does it work?" It does
not replace the dependency order, implementation steps, acceptance gates, or named code owner in the
[implementation_plan.md](active_plans/implementation_plan.md) and
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## How to use the documentation

PLE documentation has three deliberately different layers:

1. **Source authorities** decide what is allowed now: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), the
   active plans, [CONTRACTS.md](CONTRACTS.md), migrations and schemas, and the named code owner.
2. **Decision and contract maps** explain why a boundary exists and how its parts connect. Start
   here, then use the focused maps named under each decision.
3. **Operating and reference documents** explain a local workflow, input format, deployment shape,
   accessibility journey, or external integration. They make a boundary usable but do not silently
   change its authority.

This ordering prevents a useful explanation from being mistaken for an accepted release claim.

## Reading this index

- **Decision** is a durable direction, not a suggestion or UI preference.
- **Consequence** is the constraint that a change must preserve.
- **Owner** identifies the authoritative code and detailed contract.
- **Planned closure** names work that is deliberately not claimed as complete.

The [CONTRACTS.md](CONTRACTS.md) register is the change-control catalog for public module and API
boundaries. This index gives those entries their product and architectural rationale.

## Learning and content

### Question agnosticism

**Decision.** PLE is a learning engine, not a question-authoring language or a single renderer.
PLE Question JSON Questions, WeBWorK, bounded QTI import/runtime, and iMathAS Question Backend operations sit
behind typed server-side adapters.

**Why.** Biology, genetics, and biochemistry need both reusable static questions and generated
questions without making a vendor format or a browser widget the platform's core model.

**Consequence.** A new Question Backend adds a bounded adapter, public render projection, private grading
material, and capability declaration. It does not spread vendor fields, answer rules, or renderer
details through storage, browser DTOs, and UI components.
**Owner.** [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md),
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md), and the adapter entries in
[CONTRACTS.md](CONTRACTS.md#storage-and-adapter-contracts).
**Planned closure.** The release plan owns full student-runtime and authoring acceptance for the
eight PLE Question Types, broader WeBWorK compatibility, and explicitly bounded export claims.

### Bloom classification follows publication

**Decision.** Publishing a Question Revision leaves its Question Bloom Classification unassigned
and completes immediately. AI classification work searches for unassigned Published Question
Revisions and assigns each initial Bloom Cognitive Process and Bloom Knowledge Dimension pair. An
Instructor may later edit either value without creating a Question Revision.

**Why.** The Anderson and Krathwohl revision provides a useful two-dimensional search model for
the cognitive work and knowledge a Question assesses. Searching for unassigned Published Questions
supports automatic library-wide classification without coupling publication to AI availability or
a dedicated queue. Later Instructor correction preserves teaching judgment when course context
changes the best pair.

**Consequence.** A Published Question remains usable and discoverable while classification is
unassigned. Classification metadata targets the exact immutable Question Revision while remaining
separate from its content. Question Search exposes assigned pairs through independent dimension
facets and their derived 4 by 6 intersection. Instructor correction updates that metadata without a
Reason for Edit. Bloom classification remains distinct from cohort-measured Question Difficulty.

**Owner.** [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns the canonical fields.
[QUESTION_MODEL.md](QUESTION_MODEL.md#bloom-classification) owns the rubric, timing, search
semantics, and color associations.

**Planned closure.** Question Model, persistence, unassigned-classification search, server,
generated API, strict decoder, Question Search, Instructor metadata editing, and browser owners
implement the result boundary. A separate AI integration plan must settle model execution,
protected input, scheduling, concurrent claims, retry behavior, and operational evidence.

### Mastery is an assignment activity

**Decision.** Mastery assignments mean repeated practice with immediate educational feedback,
fresh Question Seeds on a new Assignment Attempt, and a highest-score learning record. A first perfect score does not
silently end practice.

**Why.** The teaching goal is confident transfer to varied problems, not one completion of a fixed
set. The instructor should choose a recognizable activity such as Mastery, Exam, or Practice rather
than assemble ordinary pedagogy from implementation primitives.

**Consequence.** The domain keeps completion, grade, continued-practice, Question Variation, timing, and
feedback policies orthogonal for correctness. The instructor and student interfaces present
opinionated activity behavior, with only evidence-supported advanced controls.
**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#course-content-philosophy),
`crates/question_model/src/assignment_activity_rules.rs`, and
[crates/domain/src/policy.rs](../crates/domain/src/policy.rs).
**Planned closure.** Each new Question Type preserves the same retry, feedback, score, and
continued-practice semantics through its owning release package.

### Assignment activity uses Instructor language

**Decision.** An Assignment contains ordered **Assignment Entries**, each a
Fixed Question or Question Pool. Each **Assignment Attempt** binds one exact
Student Record and Assignment. Its concrete, ordered selections are **Issued
Questions**. A **Question Attempt** is one try at one of those selected
questions, and a **Question Submission** accepts its student **Response**. An
optional **Assignment Submission** finalizes the whole Assignment Attempt while
Question Submissions own the Responses.

**Why.** An Instructor can clearly distinguish an attempt at a whole Assignment
from an attempt at one Question and can distinguish per-question answer
acceptance from whole-Assignment finalization. Issued Question is
necessary immutable evidence for pool selection, exact Question Revision,
source entry, order, and scoring treatment. Assignment remains the sole live
teaching definition.

**Consequence.** Assignment Attempt links directly to one Student Record and
Assignment; Issued Question links that pass to its source Assignment Entry and
exact Question Revision; Question Attempt links to one Issued Question; Question
Submission links to one Question Attempt; and Assignment
Submission, when required, links directly to the Assignment Attempt. New
PLE-owned documentation, UI, routes, types, and schema use this full hierarchy.
**Owner.** [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md).
**Planned closure.** The downstream SD1 source, schema, Store, API, browser,
and migration packages own the coordinated implementation cutover and its
acceptance evidence.

### Drafts and publications are different identities

**Decision.** Unpersisted Draft Question Content carries its private Authoring Workspace relationship
without using that Workspace as Question identity. A persisted Draft Question Revision binds that content
to one Draft Question lineage and positive revision number. The Authoring Workspace remains owned by its
Instructor and shared only through an explicit workspace relationship. Publication mints one
immutable QuestionRevision in the installation-wide Question Library under a stable QuestionId.
Private Draft Question and Question Source UUIDs belong only to the server persistence boundary;
the browser receives an opaque Draft Question Reference when it must select one draft. The Question
stewardship decision below classifies whether a later change creates another
version in that lineage or a fork with a new QuestionId. Every Assignment's pinned Question Revision remains
exactly resolvable in both Available and Archived states, with availability visible in the
Instructor-safe Question Library view. Publication has one Question Library visibility contract.
Selection eligibility is separate: Available versions appear in ordinary discovery and selection;
Archived versions remain available through exact historical references. Student access remains bound to an
assignment entitlement, and anonymous web access receives no Question Library authority.

**Why.** This prevents the classic LMS failure where a later edit changes what an earlier Student
was assessed on, while giving every Instructor an equal path to discover, organize, reuse, and
improve shared educational content. Course-record deletion leaves the Question Library intact.
Keeping drafts private prevents unfinished material from reducing discovery quality.

**Consequence.** Existing Assignments and issued Assignment Attempts retain their exact references. An Instructor
must deliberately replace or opt in to a newer version; no publication, correction, or background
action may advance an assignment. Browser requests never choose a hidden version. Internal
Question Revision UUID evidence supports replay, grading, audit, source history, and authorized transport
only; publication atomically records the version payload, lineage, source history, and visibility. The
assigned `AAA-BBBB` Question ID names the durable lineage. The Question Revision UUID is the sole
immutable content identity used by exact Assignment and evidence pins.
**Owner.** [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
[SECURITY_MODEL.md](SECURITY_MODEL.md#question-library-publication-boundary), and the Question Library rows
in [CONTRACTS.md](CONTRACTS.md#domain-contracts).

**Implementation boundary.** PLE directly applies the no-drift design while it remains
pre-production. Real native and WeBWorK host-seed publishers mint fresh opaque QuestionRevision
evidence under the stewarded QuestionId lineage, or a new QuestionId for a major semantic fork,
and converge only through a protected manifest or verified existing record. Isolated unit fixtures,
derived render/cache identities, and non-question seed records may remain deterministic. Later
schema evolution uses forward migrations and explicitly versioned protocols; no compatibility reader
preserves problem drift. WP-R2 accepted this no-drift boundary on the final material tree. M0
remains
open; WP-PY-L1 is accepted on 2026-08-15 after required live/full Validation and independent reviews
returned ACCEPT with no P0-P3 finding.

### Published Questions have four stewardship paths

**Decision.** A stable `QuestionId` names one question lineage and each `QuestionRevision` is
immutable. Published-question stewardship has four paths:

1. A Question Owner may publish a validated moderate edit as an immutable
   same-lineage version.
2. Any vetted Instructor may submit a Change Proposal against an exact
   version after publication validation succeeds. It shows semantic and grading impact; the
   Question Owner accepts or rejects it. A stale base must be rebased or resubmitted; acceptance creates a
   same-lineage version with contributor credit.
3. Any vetted Instructor may create a full fork as a private Draft Question. Publication validation
   then creates a separate `QuestionId` lineage with the fork author's authorship, compatible
   Creative Commons licensing, source attribution, and visible ancestry.
4. **Forced Question Correction** is an audited Sysadmin action reserved for a critical flaw.

Authorship, contributor credit, immutable history, source attribution, and compatible Creative
Commons licensing persist across edits, proposals, and forks. Classify changes by meaning, not byte
thresholds. Editorial/accessibility work, compatible improvements, and grading-semantic corrections
may create same-lineage versions; a correction records impact and recalculation. A major objective,
Question Type, task, or educational-purpose change creates a fork and new `QuestionId`.
Assignments, issued work, graded work, and evidence retain exact immutable version pins. Later
ordinary revisions never rewrite those pins automatically. Adoption is explicit; forced correction
owns audited remediation and preserves original evidence.

The user-facing action is **Suggest an improvement**. Change Proposal is the domain term for its
proposal, rationale, automated validation, and Question Owner review lifecycle. GitHub remains a
documentation analogy; the product implements these four
explicit stewardship paths and their own domain lifecycle.

Question Star is a visible endorsement. Vetted Instructors may see a Question's Star count and the
vetted Instructor identities that starred it. Question Watches
subscribe the watching Instructor to private in-app version, fork, improvement, and impact notices
for the watched lineage or version. A published fork is visible to other vetted Instructors through
the Question Library, while its draft remains private to its creator-owned workspace. Students and
anonymous users see neither the star identity list nor watch state.

**Why.** Stable lineage gives Instructors a durable object to recognize and follow while immutable
versions preserve reproducible grading and historical evidence. Exact pins prevent drift. The four
paths distinguish a Question Owner's edit, a lightweight contribution, a separate fork, and a critical
emergency. Meaning-based stewardship and explicit opt-in propagation preserve Instructor control,
authorship, credit, licensing, and history.

**Consequence.** Global evidence stores counts per exact QuestionRevision: accepted graded attempts,
correct outcomes, and eligible choice counts for supported Question Types.
The Question Library exposes only privacy-safe labeled rollups after applicable disclosure
thresholds are met.
Instructor Student view and previews create no evidence; published-question references stay global,
while Student records, delivery state, and private CourseInstance identity stay outside the
Question Library.
**Owner.** [QUESTION_MODEL.md](QUESTION_MODEL.md),
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
[CONTRACTS.md](CONTRACTS.md#domain-contracts), `crates/question_model/src/question_library.rs`,
`crates/domain/src/statistics.rs`, and
[QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md#lineage-and-versions).

### Forced Question Correction is Sysadmin-approved

**Decision.** Every `QuestionRevision` remains immutable, including during an emergency. A validated
corrected QuestionRevision exists before a closed, immutable, privacy-safe
**Forced Question Correction** Manifest is created. The Manifest binds the flawed version, replacement
version, reason (`security_flaw` or `critical_correctness_flaw`), affected bindings and evidence,
and deterministic remediation. A Sysadmin alone approves the correction. Approval immediately
stops new selection and issuance of the flawed version and atomically activates one authoritative
correction mapping and Correction Generation. New resolution follows that mapping immediately. Bounded,
idempotent, Correction-Generation-fenced workers materialize the mapping as one logical correction across all
active BlueprintCourse, CourseInstance, assignment, pool, and future-issuance references and
perform its remediation; the operation uses no unbounded cross-course SQL transaction. Every
unissued binding passes a deterministic compatibility check recorded in the manifest. No
per-course approval follows.
In-progress items are deterministically reissued or excused. Issued and graded work never silently
swaps versions. Completed work receives a superseding correction receipt and deterministic
recalculation, such as full credit or exclude-and-rescale when no correct answer exists. Original
prompts, responses, scores, and receipts remain immutable history. The flawed version remains
resolvable for authorized history and is marked superseded.
**Why.** Emergency correctness and security response must stop new exposure quickly while applying
one consistent remediation to every active teaching reference. A closed manifest and deterministic
compatibility/remediation check prevent a correction from changing task meaning invisibly. Immutable
original evidence preserves reproducibility, while one Sysadmin approval avoids inconsistent
course-by-course emergency decisions.

**Consequence.** Instructors receive audited correction results and action items through their
authorized course surfaces. The Sysadmin projection contains aggregate affected-version, assignment,
and course counts plus manifest status, but no Student identities, responses, grades, or private
CourseInstance identity. Replacement validation, manifest creation, Sysadmin approval, atomic
reference advancement, reissue or excuse, superseding receipt, course remediation, and
recalculation each append an attributable immutable record containing the authenticated Account, reason, time, and
exact QuestionRevision references. The Question Library labels the flawed version as superseded while
retaining its original evidence and controlled historical resolution.
**Owner.** [SECURITY_MODEL.md](SECURITY_MODEL.md),
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
[CONTRACTS.md](CONTRACTS.md#domain-contracts), `crates/question_model/src/question_library.rs`,
and `crates/domain/src/statistics.rs`.

### Instructor-facing problem identities are operational

**Decision.** `AAA-BBBB` is the single human-facing Crockford Base32 Question ID. The first six
characters are random and the seventh is an HMAC-SHA256 validation character. Instructors may copy
it from the library, but assignment reuse and checklists are the preferred shared workflow. UUIDs,
sequential numbers, and hidden snapshot versions remain internal.

**Why.** An identifier shown to a person needs to support the work that person actually does:
recognizing, communicating, copying, and entering an exact question. A UUID is valuable at internal
boundaries, but it is oversized and hostile for this instructor task.

**Consequence.** The Questions workspace accepts one or more Question IDs, normalizes documented
Crockford transcription aliases, and requires server validation before changing the draft. Invalid,
unavailable, unauthorized, or duplicate input preserves the pasted text and assignment. Every
published version keeps its stable QuestionId lineage or starts a new fork according to the semantic
change class; an explicit source-history link names the source, and an Instructor deliberately replaces
or opts in to a newer version for any assignment that should use it.
**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-philosophy),
[`QUESTION_ID_SPEC.md`](QUESTION_ID_SPEC.md), `crates/question_model/src/question_library.rs`, and
MOD-API-CAT in
[CONTRACTS.md](CONTRACTS.md#api-and-service-contracts).

### Assignment work is one aggregate

**Decision.** WP-INST-T6 gives each assignment one exact course-scoped Instructor workspace. Its
Overview, Questions, Policies, and Student view are separate tasks over the same assignment record.
Questions owns title and ordered fixed-or-pool content. Policies owns disclosure, Assignment
Activity policies, instructions, schedule, limits, late behavior, and lifecycle. Active Student
Course Membership determines ordinary access. Student view is a read-only answer-free
presentation of the current assignment, not an alternate student or preview record.

**Why.** Instructors choose a named teaching object before choosing a task. A single aggregate
revision keeps separate pages from silently overwriting each other while focused ownership prevents
a policy save from changing content or a content save from changing delivery rules.

**Consequence.** The server exposes exact nested reads at
`/api/courses/{course}/assignments/{assignment}` and
`.../student-view`, plus title-only Assignment creation and focused `.../content` and `.../policies`
mutations. Both mutations use the current `If-Match` revision, update their owned slice atomically,
and return the complete authoritative assignment projection with one new revision. Structural
content changes return a typed issued-student-work conflict after immutable work exists; a stale
revision remains a retryable conflict. The browser preserves entered values and offers reload
guidance for either case.

An empty persisted Unreleased or Archived Assignment is valid and remains reloadable. Assignment
Release Requirements are derived from the Assignment and block Released status until it has an
active deliverable position and valid policies. This makes an honest multi-page authoring workflow
possible without browser-only state or a combined write.

The Student-view route retains the Instructor identity and exact course authority, returns
`Cache-Control: no-store`, and creates no enrollment, Assignment Attempt, Question Attempt, submission, receipt, grade, or
preview record. It reuses the shared answer-free student landing presentation and course-wide base
delivery facts. Only an ordinary enrolled Student entry creates student work; that server-owned
grading path remains the source of scores and Instructor gradebook evidence.
**Owner.**
[implementation_status.md](active_plans/implementation_status.md),
[question workspace](../crates/question_model/src/assignment_workspace.rs),
and [API_CONTRACTS.md](API_CONTRACTS.md#instructor-assignment-workspace).

### BlueprintCourse owns reusable course structure

**Decision.** `BlueprintCourse` is the one canonical course-level reusable aggregate. Use ADAPT's
Alpha wording only as comparison history; PLE names no Alpha product type or compatibility alias.
The creating Instructor owns a private draft through its authoring workspace. After complete
validation succeeds, an explicit publication makes the answer-free BlueprintCourse projection
visible and reusable to every vetted Instructor. The BlueprintCourse contains ordered modules and
assignments, reusable definitions, exact published-question pins, and reusable relative schedule
defaults. Published questions remain part of the Question Library.

**Why.** Blueprint and Alpha represented one reusable-course concept with different cardinality and
access rules. One canonical aggregate keeps revision, question selection, publication, and reuse
semantics coherent. Separating reusable structure from live teaching state protects Student privacy,
preserves immutable question evidence, and lets every vetted Instructor benefit from shared content.

**Consequence.** A BlueprintCourse has reusable structure and no Students, live deadlines, releases,
accommodations, grades, or live delivery or FERPA state. Every `CourseInstance` has exactly one
non-null immutable BlueprintCourse parent and records the applied Blueprint revision. Blank-course
creation first creates a minimal BlueprintCourse, then creates its CourseInstance. A CourseInstance
is private to its current equal Teaching Team Members and enrolled Students and owns enrollment, delivery,
and FERPA state.

Relative schedule values are reusable scheduling intent. They become live deadlines only when a
CourseInstance preview and apply resolves them against that instance's term and time zone. The
CourseInstance then owns its delivery changes; local edits never flow upstream automatically.
Referenced BlueprintCourses archive instead of being hard-deleted. A BlueprintCourse change uses an
explicit publish, fork, or propose-update path. New Blueprint assignments reach daughter
CourseInstances as unreleased definitions and require an explicit instance release; propagation
never silently releases or overwrites delivery state.

Privacy-safe Question Statistics may describe global usage and disclosed learning evidence, but they
never name a private CourseInstance. CourseInstance records, Student activity, grades, and other FERPA
state remain under exact course authorization even when their published question references remain
discoverable in the Question Library.
**Owner.**
[implementation_status.md](active_plans/implementation_status.md),
[CONTRACTS.md](CONTRACTS.md#blueprint-and-instance-courses),
[NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md#blueprint-and-instance-courses), and
`crates/question_model/src/blueprint_course.rs`.

### Python owns complex orchestration

**Decision.** Python owns orchestration that keeps state, parses values, creates private temporary
files, controls subprocess or Podman lifecycle, polls, cleans up, or aggregates lanes. Bash may only
be a tiny direct `exec` or `source` wrapper and may not become a second state machine.

**Why.** The typed `local_stack_control` boundary already centralizes provider selection, private
environment handling, disposable-owner authority, process arguments, readiness, and cleanup. A
parallel shell program drifts from those security and lifecycle contracts.

**Consequence.** Python `local_stack_control` owns lifecycle and aggregate acceptance. WP-PY-L1
retired the former `launch.sh`, `_restart.sh`, and `local_identity_bootstrap.sh` launchers together
in favor of direct focused Python ownership rather than a wrapper or dual launcher. It was accepted
on 2026-08-15 after final Validation and independent review.
**Planned closure.** Remaining E2E, developer, renderer-probe, and destructive-cleanup shell
programs
migrate only in later dependency-ordered packages. A retained wrapper stays logic-free.

## Grading and student traffic

### Grading stays on the server

**Decision.** Answer keys, grading rules, provider credentials, and correctness decisions never
enter browser JSON, generated TypeScript, or the WebAssembly dependency closure.

**Why.** A browser can be inspected and modified. Client-side grading would expose answer-bearing
content and turn a student-controlled device into an authority.

**Consequence.** The browser performs presentation and format assistance only. It submits a
response to a server-owned attempt; the native grader or private adapter calculates correctness,
partial credit, and permitted feedback.
**Owner.** [SECURITY_MODEL.md](SECURITY_MODEL.md#grading-boundary),
[crates/grading/src/lib.rs](../crates/grading/src/lib.rs), and the MOD-GRD/MOD-WASM boundaries in
[CONTRACTS.md](CONTRACTS.md#boundary-invariants).
**Planned closure.** Every new Question Backend and Question Type must prove the same closure before its
browser projection is accepted.

### The attempt is the grading authority

**Decision.** Question Attempt UUID, Authenticated Session, and idempotency key bind a Question Submission;
the server loads the complete attempt relationship and durably accepts one immutable private
response before grading.

**Why.** An issued Question Attempt already binds Student Record, Course Instance, Assignment,
Assignment Attempt, Issued Question, immutable Question Revision, seed, timing, policy,
Question Response Format, and grading backend. Repeating those values expands traffic and creates conflicting
sources of truth.

**Consequence.** Server code loads and validates the issued attempt before accepting a response.
The acceptance transaction creates the immutable submission, pending evaluation, execution job,
and receipt; the sealed worker later reloads that private response and grades it. Exact replay and
status reads return the answer-free current projection rather than resubmitting the answer.
**Owner.** [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md#attempt-authority),
[Question Model Student Work Records](../crates/question_model/src/lib.rs), and the
Assignment Attempt API contract in [CONTRACTS.md](CONTRACTS.md#api-and-service-contracts).

**Current boundary.** The student submission and submission-status routes return a flattened,
answer-free tagged union with `no-store`. A `202 Accepted` response clears the browser response
buffer and exposes **Check grading status**; the worker owns later progress.

### Accepted grading has one recovery owner

**Decision.** Accepted automated grading uses one private execution handler shared by the
synchronous exact-claim path and the background recovery worker. `AcceptedSubmissionExecutionWorker`
owns the worker-only claim, private load, grading call, and tuple-fenced completion or failure.
The ordinary worker retains the existing Job Kinds; automated execution uses a dedicated
store capability and service login, while Instructor operations receive metadata-only recovery
commands.

**Why.** A student acknowledgement must remain recoverable when the request ends before grading,
and a second scheduler or a browser-held answer would create competing authority.

**Consequence.** A deterministic exception produces one assignment-local operation. The visible
journey is Student exception -> Instructor Retry -> ordinary worker -> current Gradebook. Retry
reuses the accepted private response, advances the execution generation, and leaves the existing
`1830` enqueue and `1831` current-score publication path as the sole score authority.
Host-only installation that requires immediate convergence claims the exact typed recalculation
job returned by accepted completion and executes it through that same scoring worker handler. It
does not calculate or publish a score through a second path.
**Owner.** `crates/server/src/accepted_submission_worker.rs`,
`crates/learning-data-access/src/contracts/grading_operations.rs`, and the
`GradingOperationStore` route in [CONTRACTS.md](CONTRACTS.md).

### G1 receipt schema repair is forward-only

**Decision.** G1 preserves the accepted SQLx migration files and checksums for
`2026081849`, `1850`, `1855`, `1859`, `1860`, `1861`, and `1865`. Its closeout
uses four consecutive atomic migrations, `2026081866` through `2026081869`,
with one bounded schema or writer responsibility per migration.

**Why.** Accepted migrations are immutable history, and append-only receipts
must describe only facts that were actually recorded. The pre-production
live-demo creates disposable seeded installations, so a nonempty prior receipt
history is an incompatible lifecycle rather than data to reinterpret. Four
transaction boundaries keep source history, execution writers, completion, and
Instructor writers explicit without an oversized migration or a source-limit
exception.

**Consequence.** Migration 1866 fails closed before changing receipt schema if
either `grading_execution_receipt` or `grading_operation_receipt` is nonempty;
it never backfills, disables immutability, assigns invented defaults, or
fabricates categories, accounts, workers, or retry generations. Migrations 1867,
1868, and 1869 then install the closed execution writers, the frozen 36-input
commit-v2 writer, and the Instructor writers in that order. The internal retry
capability is the five-input account-bound
`ple_prepare_accepted_submission_retry_v2`; its public caller transitions to
V2, V1 execute is revoked, and the four-input V1 is dropped with `RESTRICT`.
Truthful append-only evidence, forced RLS, and the existing lease- and
generation-fenced score publisher remain the authority boundaries.
**Owner.** The [implementation status](active_plans/implementation_status.md),
[implementation status](active_plans/implementation_status.md), and
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) own allocation, dependency
order, and acceptance evidence.
**Planned closure.** Fresh disposable PostgreSQL evidence must prove one
successful migration pass, a no-op second pass, compatibility, checksum
mutation detection, and explicit refusal against a nonempty receipt fixture.
The connected G1 oracle must call the actual five-input V2 as `ple_app` with
well-formed values and observe SQLSTATE `42501`; undefined-function failure is
not authorization evidence. The production real-stack browser and service
path must then prove answer-free student and Instructor behavior, followed by
`source source_me.sh && ./all_test.sh` on the exact final material tree.

### Render once, answer compactly

**Decision.** A rich, answer-free render payload is separate from a much smaller response payload.
Assets travel by logical reference through cacheable asset routes, not as repeated inline bytes.

**Why.** Responsiveness depends more on avoiding repeated render data and renderer work than on
trimming a few JSON characters. The split also keeps server evidence out of the browser.

**Consequence.** The target response is an attempt-bound `presentationToken` plus the minimal
answer for the exact Question Response Format. `kind` belongs in the render payload so a widget can be drawn, but the
server derives its response decoder from the issued attempt.
**Owner.** [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md#target-network-contract)
and [OBJECT_STORAGE.md](OBJECT_STORAGE.md#delivery-grants).
**Planned closure.** The payload migration and one-screen projection remain owned by the
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

### Rendered items have presentation identity

**Decision.** Selectable items receive compact, attempt-presentation-scoped rendered IDs. CRC16 is
an error-detection and correspondence mechanism, never authentication or proof of correctness.

**Why.** A visible label such as `B` is only a position. A rendered ID binds a choice, order item,
matching side, blank, Hotspot Surface, or Hotspot Region to the exact public state the student saw.

**Consequence.** PLE enforces uniqueness inside one presentation and maintains the authoritative
mapping to durable semantic IDs server-side. A whole-presentation checksum detects stale or
inconsistent render state; normal session, attempt, RLS, and idempotency controls remain the
security boundary.
**Owner.** [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md#rendered-item-ids) and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).
**Planned closure.** The codec and migrations land atomically with the minimal response wire format;
no current endpoint treats CRC16 as a bearer token.

## Data and operations

### One installation uses exact domain ownership

**Decision.** PLE is one installation with global accounts and one Question Library. Private drafts
belong to an Instructor-owned workspace. Courses, memberships,
assignments, Student work, grades, and audit evidence belong to an exact course; Student records
also bind the Student owner. Every active Instructor has the same product capabilities, while
current direct course membership determines which FERPA records that Instructor may use.

**Why.** Educational records need exact authorization, retention, and deletion. A shared question
library should improve through discovery, Question Folders, reuse, and evidence-backed replacement
without carrying Student identity or introducing an institution hierarchy.

**Consequence.** Authentication resolves the Account from the server session. PostgreSQL forced RLS
evaluates current course membership, Student ownership, private-workspace relationships, or the
specific audited Sysadmin capability in the same transaction. Published Question Library content is global
and immutable. Background work and object delivery carry the smallest real owner such as the
Course Instance, Authoring Workspace, Assignment, Assignment Attempt, or Question Attempt UUID.
**Owner.** [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
[SECURITY_MODEL.md](SECURITY_MODEL.md), and `crates/learning-data-access`.
**Planned closure.** Production deployment must still demonstrate the real non-superuser roles,
network boundaries, backups, and managed recovery controls.

### Enrollment is course-level

**Decision.** One opaque PLE Account UUID names a Student's Account across courses. A Student
Record belongs to exactly one Student Account and Course Instance. Course Enrollment creates a
Student Course Membership episode bound to that stable Student Record; re-enrollment creates a
new membership episode bound to the existing record.
An Assignment Attempt directly binds that Student Record to one Assignment. An Assignment Grade
binds the same pair and selects its contributing Assignment Attempt. Assignment lists and empty
activity states are derived from Active Student Course Membership, Assignment Status, and
effective access rules.

**Why.** A Student should retain one PLE account across courses. Course-scoped authorization,
Student ownership, and RLS control disclosure more reliably than pretending the same
person is a different identity in every class. Verified email is the mutable canonical sign-in
attribute, not the identity key; passkeys are optional convenience credentials for that account.

**Consequence.** An Instructor creates a pending invitation with protected course-scoped roster
metadata, then shares its one-time copy link through an existing trusted LMS or uses configured
SMTP. After the Student completes email authentication and claims the invitation, PLE resolves or
creates the Student Record and creates the exact Course Membership binding atomically. An authorized
pre-activity Assignment read returns
an empty activity projection. Starting an Assignment Attempt creates the direct Student
Record-to-Assignment activity relationship transactionally; calculating a Grade creates its exact
grade record. New Assignments add definitions, while Student rows appear with actual Student work.
The retention workflow preserves existing education records after access ends. Server-issued
evidence establishes every Account, Course Membership, Student Record, and invitation claim.
**Owner.** [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md),
[IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md), and the course capabilities in
`crates/learning-data-access` and `crates/server/src/course/`.

**Planned closure.** The HTTP roster, invitation claim, bulk preview/commit, and passwordless
account boundary are implemented. Acceptance remains open for the canonical operator-configured
email-provider journey, optional-passkey and multi-replica evidence, and independent security/HCI
closeout.

### Current personas are Student, Instructor, and Sysadmin

**Decision.** Each PLE account has exactly one immutable current Student, Instructor, or Sysadmin
role. A person needing multiple roles uses separate accounts; Dr. Voss may use separate Instructor
and Sysadmin accounts. Instructor Vetting is real-person validation before Account Creation, and teaching requires
direct Instructor membership. A Sysadmin creates a Course Instance only for an explicitly assigned
active Instructor account, which receives the initial membership; the Sysadmin receives none.
Course help uses an explicit, audited, time-bounded support capability with a stated purpose.
Sysadmin has no ambient FERPA browsing. Publishing content is an Instructor action; the
public-asset publisher is a service identity, not a person. Every active Instructor has the same
product capabilities, including shared-problem discovery, Question Folders, publication, reuse, and
improvement workflows.

**Why.** Ambient administrator or manager roles turn one compromised platform
credential into access to every student's educational record. A publisher
human role also confuses author approval with the least-authority service that
materializes immutable public bytes.

**Consequence.** Product Role is the closed Student/Instructor/Sysadmin set, currently represented
by `AccountRole` in code, and Account/session storage carries one role, never a collection. Course
Membership is the smaller Student/Instructor relation and must match Product Role. Sysadmin Accounts
cannot hold Course Membership. A Course
may have multiple current Teaching Team Member accounts with equal teaching authority. A support capability
names the exact course and, when needed, Student; it expires on a recorded deadline and records
the authenticated account, purpose, action, and time for every boundary crossing. All course-linked Student data
receives the FERPA radioactive handling discipline. Implementation and acceptance evidence remain
pending under SD1.

**Session issuance rule.** The Authenticated Session issuance operation accepts an existing Account identity
and opaque session parameters, then derives Product Role from the immutable Account row in the same
trusted transaction. A passwordless ceremony, browser request, or adapter never selects Product
Role. The resulting Authenticated Session stores the derived role and remains bound to that Account
for its lifetime. This keeps the fixed-role decision at the trusted service boundary (ASVS 2.2.1,
7.2.1, and 8.3.1). The `2026082906` Authenticated Session Resolution function and `SessionStore` implement this derivation; the
passwordless ceremony and full SD1 acceptance remain separate work.

The authorization boundary remains capability-oriented so a later package can add bounded Grader,
Course Observer, or Student Observer relationships without widening the current personas. A Course
Observer receives anonymous aggregate grades and no Student-level FERPA records. Each future
relationship lands with its visible workflow, revocation, audit, and privacy contracts.

**Owner.** [USER_ROLES.md](USER_ROLES.md),
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), and
[DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md). The accountable-course-assignment evidence is
the binding [implementation_status.md](active_plans/implementation_status.md).

### Course accountability is assigned and transferable

**Decision.** Each CourseInstance records one accountable assigned Instructor from its current
Instructor memberships. Every current Teaching Team Member keeps the same teaching and FERPA predicates.
An audited atomic course-administration operation transfers the assignment only after the successor
holds a current Instructor membership.

**Why.** An accountable Instructor gives course creation, handoff, and support records one clear
human responsibility without turning one ordinary Teaching Team Member into a broader authority class.

**Consequence.** The CourseInstance stores the assigned Instructor as a validated Instructor
account reference. A deferred integrity check requires that account's current Instructor membership
at transaction commit, including after revocation or transfer. Course creation inserts the assigned
Instructor's first ordinary membership in the same transaction. Authorization continues to evaluate
the same predicate for every current Teaching Team Member; only accountability and audit identify the
assigned Instructor.

**Owner.** The binding
[implementation_status.md](active_plans/implementation_status.md),
course-membership schema, and teaching-authority Store contract.

### APIs are stateless; durable state is shared

**Decision.** Any API replica can serve an authenticated request. Durable state lives in PostgreSQL,
object storage, and the queue; a browser copy or a replica's memory never establishes authority.

**Why.** Scale should come from adding replicas and surviving process restarts, not sticky sessions
or a privileged in-memory coordinator.

**Consequence.** Sessions, attempts, idempotency receipts, leases, and prefetch ownership are
durable. Replica recovery has explicit fencing rules, and workers use lease/generation boundaries.

**Owner.** [MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md),
[CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md#current-and-target-topology), and MOD-WORKER in
[CONTRACTS.md](CONTRACTS.md#api-and-service-contracts).

**Planned closure.** Deployment-scale, clock-skew, soak, and managed-service evidence belong to the
release/deployment packages rather than ordinary offline tests.

### Object storage is typed and server-owned

**Decision.** Binary and archival bytes live behind typed server-generated Object Addresses and immutable
`ObjectRecord` evidence; client requests name logical delivery IDs, never buckets or paths.

**Why.** A bucket path is storage implementation detail and an authorization hazard. Typed objects
bind lifecycle, checksum, media type, source history, and delivery policy to the correct owner.

**Consequence.** Writes verify SHA-256, source and temporary objects are non-deliverable, protected
asset delivery is authorized and audited, and the database records intended existence while storage
proves bytes exist.

**Owner.** [OBJECT_STORAGE.md](OBJECT_STORAGE.md),
[crates/objects/src/lib.rs](../crates/objects/src/lib.rs), and MOD-OBJ in
[CONTRACTS.md](CONTRACTS.md#storage-and-adapter-contracts).

**Planned closure.** Inventory checks, orphan cleanup, and handling of missing referenced bytes
remain a release package. Student file responses remain fail-closed until their attempt-bound
upload capability and inspection workflow are implemented.

### Privacy deletes records, not learning evidence

**Decision.** The owner defaults for the CourseInstance lifecycle are notice after 30 days, archive
Student records after 100 days, and permanent deletion after 365 days. Course-owned assignment
definitions normally remain; identity-free anonymous aggregates remain available to improve the
shared library.

**Why.** Students need privacy by default, while question quality improves only if non-identifying,
non-retractable aggregate evidence survives a student record's lifecycle.

**Consequence.** Deletion removes the course-owned student graph and its typed student-record
objects, but never follows immutable assignment references into shared publication. Anonymous
statistics have their own aggregation and k-anonymous disclosure boundary.

**Owner.** [RETENTION_POLICY.md](RETENTION_POLICY.md),
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#radioactive-records-and-retention), and
MOD-STATS/MOD-RETENTION in [CONTRACTS.md](CONTRACTS.md#api-and-service-contracts).

**Planned closure.** A deployment operator may configure a later ordered retention policy.
Production backup
retention and recovery objectives require explicit infrastructure choices and evidence.

## Browser and accessibility

### Solid renders; Rust/Wasm validates browser-safe work

**Decision.** SolidJS owns interactive browser composition. Rust/Wasm owns deterministic,
answer-free shared computation such as format validation, timing display, state transitions, and
generated contract support.

**Why.** Solid provides a small reactive UI layer while Rust preserves server/browser consistency
for
appropriate deterministic calculations without exposing the grader.

**Consequence.** The browser may import only the deliberate Wasm export allowlist. Private keys,
grading code, database access, object paths, and provider credentials remain outside that closure.

**Owner.** [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md#browser-client),
[SECURITY_MODEL.md](SECURITY_MODEL.md#compile-time-closure), and MOD-WASM/MOD-CLIENT in
[CONTRACTS.md](CONTRACTS.md#platform-contracts).

**Planned closure.** New Wasm exports require an explicit security and generated-contract review;
performance work follows measured need rather than speculative porting.

### Keyboard is the primary student path

**Decision.** Every student action works without a mouse. Tab and Shift+Tab move focus; Space uses
native selection or activates a focused button; native links retain Enter. Arrows, digits,
Enter-to-submit, and Escape are optional widget extensions.

**Why.** Keyboard-only operation is a core learning path, not an accessibility afterthought. It also
makes the normal sequence of understand, answer, submit, recover, and continue testable.

**Consequence.** A visible platform-keyboard journey is required before shortcut tests. Drag-only,
hover-only, coordinate-only, or time-critical required interactions are not eligible for a student
question; hotspot questions need a pedagogically equivalent keyboard path.

**Owner.** [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md) and the browser
contracts in [CONTRACTS.md](CONTRACTS.md#browser-contracts).

**Planned closure.** Each new Question Type supplies its own keyboard evidence as it lands; it
does not wait for a generic final audit.

## Question Backend systems and evidence

### Question Backends remain private adapters

**Decision.** PLE, never a student browser, contacts a managed Question Backend such as WeBWorK or iMathAS. PLE
owns published source, issued seed, response projection, credentials, timeout, sanitization, and
result translation.

**Why.** Upstream systems use their own fields, sessions, HTML, and credentials. Those are neither
stable browser contracts nor safe student authority.

**Consequence.** The accepted WeBWorK path is the four reviewed Chapter 1 PGML sources, comprising
one radio and one matching question per chapter, via the private standalone `/render-api` Question Backend;
browser data is a PLE-native response. Raw source, hidden fields, sessions, Question Backend values, and
renderer output do not cross the PLE browser boundary.

**Owner.** [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md),
[ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md), and MOD-ADP-WW in
[CONTRACTS.md](CONTRACTS.md#storage-and-adapter-contracts).

**Planned closure.** Broader problem compatibility and any unreviewed matching source require their
own accepted projection and live evidence; they are not inferred from the Chapter 1 profile.

### H5P Package Import is not a Question Backend

**Decision.** H5P is the `h5p` Question Format and bounded H5P Package Import path.
**Why.** Its immutable archive, checksum, content type, and import fingerprint retain archival evidence for an unpublished, key-free, ungraded practice payload; it has no server validation, issue, reproduction, or automated-grading lifecycle.
**Consequence.** H5P cannot enter Question Backend, backend locator, Question Source, Question Library, or Assignment records; its importer retains hostile-input archive validation, immutable archive resolution, checksum verification, and unsupported-feature refusal; graded Questions use an approved Question Backend.
**Owner.** [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md#question-format-and-question-type), [INPUT_FORMATS.md](INPUT_FORMATS.md), and `crates/adapters/h5p/src/import.rs`.

### Tests prove behavior at the right layer

**Decision.** Permanent tests are deterministic, offline, behavior-focused evidence. Disposable
live services, container, performance, backup, and visual probes prove environment-dependent
claims once and are recorded rather than retained as brittle routine tests.

**Why.** A permanent test suite must stay trustworthy and fast enough to run often. Exact file
layouts, tunable constants, mock wiring, and live infrastructure can create false confidence or
maintenance burden without proving student behavior.

**Consequence.** Memory and mock backends support unit and conformance behavior. PostgreSQL RLS,
MinIO, renderer, browser, recovery, and deployment claims use the named disposable oracle or human
acceptance evidence. A source-size gate is permanent architecture evidence because it protects
capability ownership.

**Owner.** [PYTEST_STYLE.md](PYTEST_STYLE.md),
[DEVELOPMENT.md](DEVELOPMENT.md#choose-the-right-gate),
and [E2E_TESTS.md](E2E_TESTS.md).

**Planned closure.** Each release package records its exact permanent and one-time evidence before
status can claim the behavior is accepted.

## Product presentation and operations

### Viewport and visual evidence profiles

**Decision.** Instructor and Sysadmin design and permanent visual evidence use the canonical 1280
by 800 CSS-pixel desktop profile. The corpus label `laptop` is exactly the established 1280 by 800
desktop 16:10 evidence profile. Student design also covers 800 by 1280 portrait
tablet, 393 by 852 narrow phone, and 800 by 800 square profiles; profile weights guide planning and
do not create screenshot quotas or pixel-equivalence acceptance.

**Why.** The owner prioritizes desktop teaching and administration while Student devices vary.
Semantic usability, accessibility, privacy, and task completion are stronger evidence than exact
rendered dimensions.

### The demo is ordinary product state

**Decision.** The demo uses PostgreSQL, real migrations, RLS, persistent seeded teaching courses,
ordinary memberships, and ordinary student work through the production-shaped browser and server
stack. Preview, acceptance, and production behavior share this one live product model.

**Why.** A parallel mock product creates false assurance and cannot prove the real teaching path.
Recognizable courses and deterministic observations make returning to the demo resemble checking
on active teaching.

### Teaching workspaces use task-owned composition

**Decision.** Instructor assignment work is composed as focused Overview, Questions, Policies,
Grading operations, and Student-view tasks over one authoritative assignment. Useful desktop width
goes to scanning and
editing; the complete current task and primary save action should fit comfortably at 1280 by 800.

**Why.** Task-level hierarchy is easier to teach and operate than a grid of equally padded cards.
Separating content from policy prevents unrelated fields from competing on one page.

### Product navigation exposes Questions through one library surface

**Decision.** The Product Ribbon has four ordered slots: Courses, Question Library, Blueprint
Courses, and Account. Question Library contains All Questions, My Questions, My Question Drafts,
Starred, and Watched as its five Ribbon Tasks. The first three are library views; Starred and
Watched are exact Account relationships to Questions.

**Why.** The Product Ribbon stays organized by primary object type. Ownership, publication state,
endorsement, and notification subscription remain distinct views of Questions instead of becoming
competing top-level repositories.

**Consequence.** Question Folders, tags, classifications, Saved Question Searches, and search
facets organize or find Questions within those views. Star means visible endorsement, and Watch
means private notification subscription.

### Instructor course navigation has one spatial owner

**Decision.** The authorized course-route scope owns one Instructor course frame: stable course
identity, one six-slot Course Instance Ribbon, and one content origin below it. Its ordered slots are
Assignments, Students, Gradebook, Teaching Operations, Blueprint Updates, and Course Setup. Course
Setup contains Grade Settings and Appearance as Ribbon Tasks. Individual route pages own their task
heading and workflow content.

**Why.** A persistent navigation landmark preserves spatial memory and makes a tab change feel like
changing tasks inside one course instead of opening an unrelated page. Central ownership also keeps
nested assignment, Gradebook inspection, and course-setting routes aligned as the product grows.

### Course appearance derives usable roles from three anchors

**Decision.** A course selects one three-color biome or habitat theme and may add a centered banner
normalized to 1200 by 328 without stretching. The default `grass` anchors are `#73C167`, `#008852`,
and `#BDDEB1`; readable interface roles are derived without changing the stored anchors.

**Why.** The owner wants Blackboard Original-like course identity, not three decorative swatches on
otherwise identical white pages. Derived roles preserve recognizable color while meeting contrast
and accessibility needs.

## Demonstration and release evidence

### Exact owners bind authorization decisions

**Decision.** [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md) and
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) are the binding
single-installation authorization contracts for Account identity, Question Library
publication, course records, private authoring, workers, objects, exports,
retention, observer relationships, and Sysadmin support. Each protected
operation resolves its Account from the active session and checks the durable
owner and exact predicate those contracts define.

**Why.** Every authorization decision needs an object that names its real scope.
That keeps Question Library access, course records, private work, and worker leases
independently reviewable without relying on an ambient installation boundary.

**Consequence.** Baseline relations, Store contracts, protected authorization functions, and acceptance
cases derive their parent identifiers and predicates from those contracts.
Observer and support relations remain narrow recorded grants, workers keep
immutable typed targets and leases, and object delivery verifies its actual
Question Library, workspace, Course Instance, Student Record, or lease parent.

**Owner.** [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md) for
PostgreSQL authorization and [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md)
for product and service authorization.

### Locked job targets carry authorization ownership

**Decision.** Every durable job has one server-resolved immutable typed target
in addition to its closed handler kind, generation fence, and opaque current
lease. Course work records its exact Course Instance UUID and Assignment or
Assignment Attempt UUID;
workspace work records its Authoring Workspace UUID and import when applicable;
Question Library work records its exact immutable Question Revision UUID; exports record
their Assignment Export UUID, Course Instance, frozen Manifest, and expected
Artifact UUIDs. A worker
Job claim-and-lease operation compares handler kind, typed target, generation, unexpired lease, and
the requested transition before preparation, reads, writes, retry, cancellation,
or finalization.

**Why.** An object identifier alone cannot establish the authorization parent
for export or import work. Persisting the resolved target at enqueue time
makes a claim self-contained, prevents work from following mutable surrounding
state, and gives each retry and revocation path one exact boundary to verify.

**Consequence.** The baseline schema adds the locked target to each job row.
The enqueue transaction resolves it from currently authorized records; a new
generation creates new work rather than changing a claim's target. The
acceptance suite proves rejection for foreign targets, stale generations,
mismatched Job Kind Registrations, expired leases, and client-supplied scope values.
**Owner.** [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md),
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
`crates/learning-data-access/src/jobs.rs`, and the baseline Job claim-and-lease operation.

### Seed data represents ordinary teaching

**Decision.** Fresh installations seed the named Genetics and Biochemistry teaching courses,
ordinary active memberships, and five deterministic observations on meaningful Chapter 1 work.
Internal installer recipe names stay diagnostic; product navigation displays teaching names.

**Why.** Seed data should demonstrate actual course, assignment, analysis, and discovery workflows
rather than synthetic infrastructure records.

### Direct demo entry replaces verification only

**Decision.** Public demo entry may select a seeded Student, Instructor, or Sysadmin identity, but
the server still resolves the ordinary account, session, role, membership, and authorization.
Elena Instructor and Morgan Sysadmin exercise ordinary passkey enrollment, sign-out, and sign-in.

**Why.** SMTP is not configured for current acceptance. Bypassing only email verification keeps the
demo accessible without replacing authorization or claiming unverified email delivery.

### The canonical walkthrough is a focused teaching loop

**Decision.** The pilot walkthrough has an Instructor create a course, add an active Student, build
a representative four-Question Chapter 1 Assignment from Published Questions, and observe the
Student's submitted and scored work. The complete eight-question sweep is a separate release gate.

**Why.** A focused realistic loop demonstrates first success without substituting a one-question
toy or forcing the full release corpus into every walkthrough.

## Content and grading formats

### iMathAS Session, Context, and evidence have one owner

**Decision.** Question Model owns the typed `ImathasQuestionBackendBinding`: iMathAS Deployment Reference, iMathAS Item Reference, and the pinned `imathas_remote_grading_v1` profile.
LDA owns the sole server-only iMathAS Question Backend Session, persists that exact binding, and owns its typed Session Reference, preparation/restore/lease/iMathAS Result Exchange Store boundary, and XChaCha20-Poly1305 backend-state protection with rotation.
The iMathAS adapter owns iMathAS Launch Reference, iMathAS Launch State protocol bytes, iMathAS Render Cache Entry, and iMathAS Launch/Result HMAC and protocol verification. LDA mints the Session's OS-CSPRNG 256-bit Challenge, which iMathAS carries only as signed `ple_launch_challenge`.
`ImathasGradingContext` remains exactly its redacted non-Serde `{ QuestionAttemptId, QuestionRevisionReference, QuestionSeed }` triple, expires with its Session, and preserves `authentication_payload_v1`; the separate required `QuestionGradingRule` is an issue-time Session fact.
The iMathAS Result Token and checksum are LDA evidence after server-to-server verification; raw bytes never persist or enter browser/generated/log/Debug output.

**Why.** One owner lets `2026090102` enforce exact restore, RLS, forward iMathAS Session/Result Exchange transitions, and four-axis context mismatch refusal without a parallel adapter or browser identity boundary. The browser launch shell accepts only validated `{ launchUrl }`; its LDA-backed Rust route, cookie/env backend composition, and live-backend acceptance remain separate work.

### iMathAS Result uses Ready-to-Commit then worker commit

**Decision.** The approved durable model is Ready-to-Commit plus worker-leased idempotent grading commit. A Question is never Remote or External; `ImathasQuestionBackend`/`imathasQuestionBackend` is the exact renamed response/control/Student Response marker. After iMathAS verification outside PostgreSQL, authenticated staging consumes the exact active iMathAS Session and atomically writes the iMathAS Result Exchange's finite `[0,1]` nonnegative-zero normalized-score-only iMathAS Result, its LDA checksum `SHA-256("ple:imathas-result:v1\\0" || IEEE-754-binary64(score))`, separate iMathAS Result Token checksum, the marker Question Submission, pending Question Submission Grading, and ready typed `grade_accepted_submission` Job. A worker holding that exact Job lease rechecks the lineage and atomically derives the PLE Grading Result plus LDA-owned redacted/non-Serde Automated Grading Receipt Checksum from the fixed v1 prefix, lineage UUID bytes, two Result Exchange checksums, correct byte, canonical big-endian binary64 points, and signed big-endian commit milliseconds; the same transaction writes the Receipt, completes the Job, marks grading graded, and advances the iMathAS Result Exchange to committed.

**Why.** Ready-to-Commit survives interruption without another backend request; an expired Job lease permits a later claim. Final execution failure belongs to the Job and Question Submission Grading (`instructor_attention`), retaining immutable ready evidence for a separately authorized recovery Job. Exact matching staging/commit replays are idempotent; committed replay returns the stored Receipt, Result, and checksum rather than accepting a candidate checksum. The checksum is never command/API/browser/adapter input. The iMathAS Result belongs to its iMathAS Result Exchange and is distinct from raw-token evidence and PLE Grading Result. LTI remains future registered-protocol planning with no current record or schema.

**Consequence.** RQB2 directly amends fresh migration `2026090102`; no alias or compatibility layer is retained, and the accepted submission, lifecycle, relationship, procedure, browser-launch, security, and test boundaries keep their behavior. **Owner.** [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md).

### PLE Question JSON is the static-Question authority

**Decision.** Versioned PLE Question JSON is canonical for MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. YAML may compile once into that contract; QTI is an import, export, and archival adapter rather than internal authority.

**Why.** One deterministic cross-language contract avoids competing source models. Adapter formats can preserve interchange without dictating the engine's internal representation.

### Native interactions adapt the QTI self-test model

**Decision.** PLE Question Implementations borrow the QTI Package Maker self-test's compact task, obvious submit, visible response state, per-part completion, plain-language feedback, reset, and completed state. PLE retains server-only grading, labeled controls, keyboard operation, and recoverable errors.

**Why.** Students should learn one clear interaction vocabulary without importing client-side answers, drag-only controls, result-string protocols, or inaccessible presentation choices.

### Binary question assets use object storage

**Decision.** Images and other binary references keep bytes, checksums, media types, lifecycle, and authorization in typed PLE object storage rather than JSON or database rows. Optional correct and incorrect feedback remain shared sidecars and do not determine validity.

**Why.** Typed storage preserves authorization and lifecycle boundaries while keeping the canonical question contract compact even when author feedback is incomplete.

## Related decisions

The settled identity, authentication, privacy, recovery, and Blueprint-collaboration decisions are retained in [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md). The focused local-stack, Gradebook, wire-contract, and Blueprint-operation decisions are retained in [DESIGN_DECISIONS_OPERATIONS.md](DESIGN_DECISIONS_OPERATIONS.md).
