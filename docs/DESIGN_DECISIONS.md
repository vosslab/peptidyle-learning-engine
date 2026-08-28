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
Native flat questions, WeBWorK, bounded QTI import/runtime, and contracted external tools sit
behind typed server-side adapters.

**Why.** Biology, genetics, and biochemistry need both reusable static questions and generated
questions without making a vendor format or a browser widget the platform's core model.

**Consequence.** A new family adds a bounded adapter, public render projection, private grading
material, and capability declaration. It does not spread vendor fields, answer rules, or renderer
details through storage, browser DTOs, and UI components.

**Owner.** [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md),
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md), and the adapter entries in
[CONTRACTS.md](CONTRACTS.md#storage-and-adapter-contracts).

**Planned closure.** The release plan owns full learner-runtime and authoring acceptance for the
eight flat families, broader WeBWorK compatibility, and explicitly bounded export claims.

### Mastery is an assignment activity

**Decision.** Mastery assignments mean repeated practice with immediate educational feedback,
fresh variation on a new run, and a highest-score learning record. A first perfect score does not
silently end practice.

**Why.** The teaching goal is confident transfer to varied problems, not one completion of a fixed
set. The instructor should choose a recognizable activity such as Mastery, Exam, or Practice rather
than assemble ordinary pedagogy from implementation primitives.

**Consequence.** The domain keeps completion, grade, continued-practice, variation, timing, and
feedback policies orthogonal for correctness. The instructor and student interfaces present
opinionated activity behavior, with only evidence-supported advanced controls.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#teaching-and-product-priorities),
[crates/question_model/src/run_policy.rs](../crates/question_model/src/run_policy.rs), and
[crates/domain/src/policy.rs](../crates/domain/src/policy.rs).

**Planned closure.** New response families must preserve the same retry, feedback, score, and
continued-practice semantics; this is part of their owning release package, not a later UI cleanup.

### Drafts and publications are different identities

**Decision.** An instructor's mutable draft is tenant-owned. Publication mints one immutable shared
question, with its own Question ID and hidden immutable snapshot. Every content change, including an
original-owner bug correction, publishes a distinct question with a distinct Question ID; explicit
immutable provenance may link that replacement to its source.

**Why.** This prevents the classic LMS failure where a later edit changes what an earlier learner
was assessed on, and it lets tenant record deletion leave shared educational content intact.

**Consequence.** Existing assignments and issued runs retain their exact references. An Instructor
must deliberately replace an assignment item; no publication, correction, or background action may
advance it. Browser requests never choose a hidden version. Internal `(ProblemId, VersionId)`
evidence is freshly opaque for each real publication and supports replay, grading, audit, provenance,
and authorized transport only; publication atomically records the new question's payload, provenance,
and visibility. The assigned `AAA-BBBB` Question ID is the sole durable question identity:
`ProblemPublicId`/`P-...`, `ProblemVersionNumber`, and predecessor/version-chain semantics are not
hidden alternatives.

**Owner.** [DATABASE_TENANCY.md](DATABASE_TENANCY.md#ownership-boundary),
[SECURITY_MODEL.md](SECURITY_MODEL.md#catalog-publication-boundary), and the identity/catalog rows
in [CONTRACTS.md](CONTRACTS.md#domain-contracts).

**Implementation boundary.** PLE directly applies the no-drift design while it remains
pre-production. Real native and WeBWorK host-seed publishers mint fresh opaque publication IDs and
converge only through a protected manifest or verified existing record. Isolated unit fixtures,
derived render/cache identities, and non-question seed records may remain deterministic. Later
schema evolution uses forward migrations and explicitly versioned protocols; no compatibility reader
preserves problem drift. WP-R2 accepted this no-drift boundary on the final material tree. M0 remains
open; WP-PY-L1 is accepted on 2026-08-15 after required live/full Validation and independent reviews
returned ACCEPT with no P0-P3 finding.

### Instructor-facing problem identities are operational

**Decision.** `AAA-BBBB` is the single human-facing Crockford Base32 Question ID. The first six
characters are random and the seventh is an HMAC-SHA256 validation character. Instructors may copy
it from the library, but assignment reuse and checklists are the preferred group workflow. UUIDs,
sequential numbers, and hidden snapshot versions remain internal.

**Why.** An identifier shown to a person needs to support the work that person actually does:
recognizing, communicating, copying, and entering an exact question. A UUID is valuable at internal
boundaries, but it is oversized and hostile for this instructor task.

**Consequence.** The Questions workspace accepts one or more Question IDs, normalizes documented
Crockford transcription aliases, and requires server validation before changing the draft. Invalid,
unavailable, unauthorized, or duplicate input preserves the pasted text and assignment. Every
content change, including a correction, has a new Question ID; an explicit provenance link may name
the source, and an Instructor deliberately replaces any assignment item that should use it.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#teaching-and-product-priorities),
[`QUESTION_ID_SPEC.md`](QUESTION_ID_SPEC.md), `crates/question_model/src/catalog.rs`, and MOD-API-CAT in
[CONTRACTS.md](CONTRACTS.md#api-and-service-contracts).

### Assignment work is one aggregate

**Decision.** WP-PROF-T6 gives each assignment one exact course-scoped Instructor workspace. Its
Overview, Questions, Policies, and Student view are separate tasks over the same assignment record.
Questions owns title and ordered fixed-or-pool content. Policies owns audience, disclosure, run
policies, instructions, schedule, limits, late behavior, and lifecycle. Student view is a read-only
answer-free presentation of the current assignment, not an alternate learner or preview record.

**Why.** Instructors choose a named teaching object before choosing a task. A single aggregate
revision keeps separate pages from silently overwriting each other while focused ownership prevents
a policy save from changing content or a content save from changing delivery rules.

**Consequence.** The server exposes exact nested reads at
`/api/courses/{course}/assignments/{assignment}` and
`.../student-view`, plus title-only Draft creation and focused `.../content` and `.../policies`
mutations. Both mutations use the current `If-Match` revision, update their owned slice atomically,
and return the complete authoritative assignment projection with one new revision. Structural
content changes return a typed issued-learner-work conflict after immutable work exists; a stale
revision remains a retryable conflict. The browser preserves entered values and offers reload
guidance for either case.

An empty persisted Draft or Archived definition is valid and remains reloadable. Publication
readiness is derived from the definition and blocks Published until it has an active deliverable
position and valid policies. This makes an honest multi-page drafting workflow possible without
browser-only state or a combined write.

The Student-view route retains the Instructor identity and exact course authority, returns
`Cache-Control: no-store`, and creates no enrollment, run, attempt, submission, receipt, grade, or
preview record. It reuses the shared answer-free learner landing presentation and course-wide base
delivery facts. Only an ordinary enrolled Student entry creates learner work; that server-owned
grading path remains the source of scores and Instructor gradebook evidence.

**Owner.** [instructor_assignment_workspace_plan.md](active_plans/active/instructor_assignment_workspace_plan.md),
[`crates/question_model/src/assignment_workspace.rs`](../crates/question_model/src/assignment_workspace.rs),
[`crates/server/src/course/assignments/workspace.rs`](../crates/server/src/course/assignments/workspace.rs),
and [API_CONTRACTS.md](API_CONTRACTS.md#instructor-assignment-workspace).

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

**Planned closure.** Remaining E2E, developer, renderer-probe, and destructive-cleanup shell programs
migrate only in later dependency-ordered packages. A retained wrapper stays logic-free.

## Grading and learner traffic

### Grading stays on the server

**Decision.** Answer keys, grading rules, provider credentials, and correctness decisions never
enter browser JSON, generated TypeScript, or the WebAssembly dependency closure.

**Why.** A browser can be inspected and modified. Client-side grading would expose answer-bearing
content and turn a learner-controlled device into an authority.

**Consequence.** The browser performs presentation and format assistance only. It submits a
response to a server-owned attempt; the native grader or private adapter calculates correctness,
partial credit, and permitted feedback.

**Owner.** [SECURITY_MODEL.md](SECURITY_MODEL.md#grading-boundary),
[crates/grading/src/lib.rs](../crates/grading/src/lib.rs), and the MOD-GRD/MOD-WASM boundaries in
[CONTRACTS.md](CONTRACTS.md#boundary-invariants).

**Planned closure.** Every new adapter and question family must prove the same closure before its
browser projection is accepted.

### The attempt is the grading authority

**Decision.** `QuestionAttemptId`, authenticated session, and idempotency key bind a submission;
the server durably accepts one immutable private response before grading. The browser does not
resend the question, course, assignment, version, seed, backend, or response family as authority.

**Why.** An issued attempt already binds learner, tenant, run, immutable version, seed, timing,
policy, response schema, and grading backend. Repeating those values expands traffic and creates
conflicting sources of truth.

**Consequence.** Server code loads and validates the issued attempt before accepting a response.
The acceptance transaction creates the immutable submission, pending evaluation, execution job,
and receipt; the sealed worker later reloads that private response and grades it. Exact replay and
status reads return the answer-free current projection rather than resubmitting the answer.

**Owner.** [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md#attempt-authority),
[crates/question_model/src/activity.rs](../crates/question_model/src/activity.rs), and MOD-API-RUN
in [CONTRACTS.md](CONTRACTS.md#api-and-service-contracts).

**Current boundary.** The learner submission and submission-status routes return a flattened,
answer-free tagged union with `no-store`. A `202 Accepted` response clears the browser response
buffer and exposes **Check grading status**; the worker owns later progress.

### Accepted grading has one recovery owner

**Decision.** Accepted automated grading uses one private execution handler shared by the
synchronous exact-claim path and the background recovery worker. `AcceptedSubmissionExecutionWorker`
owns the worker-only claim, private load, grading call, and tuple-fenced completion or failure.
The ordinary worker retains the existing queue families; automated execution uses a dedicated
store capability and process login, while Instructor operations receive metadata-only recovery
commands.

**Why.** A learner acknowledgement must remain recoverable when the request ends before grading,
and a second scheduler or a browser-held answer would create competing authority.

**Consequence.** A deterministic exception produces one assignment-local operation. The visible
journey is Student exception -> Instructor Retry -> ordinary worker -> current Gradebook. Retry
reuses the accepted private response, advances the execution generation, and leaves the existing
`1830` enqueue and `1831` current-score publication path as the sole score authority.

**Owner.** `crates/server/src/accepted_submission_worker.rs`,
`crates/learning-data-access/src/contracts/grading_operations.rs`, and the
`GradingOperationStore` route in [CONTRACTS.md](CONTRACTS.md).

### Render once, answer compactly

**Decision.** A rich, answer-free render payload is separate from a much smaller response payload.
Assets travel by logical reference through cacheable asset routes, not as repeated inline bytes.

**Why.** Responsiveness depends more on avoiding repeated render data and renderer work than on
trimming a few JSON characters. The split also keeps server evidence out of the browser.

**Consequence.** The target response is an attempt-bound `presentationDigest` plus the minimal
family-specific answer. `kind` belongs in the render payload so a widget can be drawn, but the
server derives its response decoder from the issued attempt.

**Owner.** [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md#target-network-contract)
and [OBJECT_STORAGE.md](OBJECT_STORAGE.md#delivery-grants).

**Planned closure.** The payload migration and one-screen projection remain owned by the
[secure_question_grading_payload_plan.md](active_plans/decisions/secure_question_grading_payload_plan.md).

### Rendered items have presentation identity

**Decision.** Selectable items receive compact, attempt-presentation-scoped rendered IDs. CRC16 is
an error-detection and correspondence mechanism, never authentication or proof of correctness.

**Why.** A visible label such as `B` is only a position. A rendered ID binds a choice, order item,
matching side, blank, or hotspot surface to the exact public state the learner saw.

**Consequence.** PLE enforces uniqueness inside one presentation and maintains the authoritative
mapping to durable semantic IDs server-side. A whole-presentation digest detects stale or
inconsistent render state; normal session, attempt, RLS, and idempotency controls remain the
security boundary.

**Owner.** [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md#rendered-item-ids) and
[secure_question_grading_payload_plan.md](active_plans/decisions/secure_question_grading_payload_plan.md).

**Planned closure.** The codec and migrations land atomically with the minimal response wire format;
no current endpoint treats CRC16 as a bearer token.

## Data and operations

### Tenant records and shared content stay separate

**Decision.** Courses, memberships, assignments, learner work, grades, and audit evidence are
tenant-owned. Published versions and their immutable source/asset evidence are shared content.

**Why.** Educational records need isolation, retention, and deletion, while a reusable question
library should improve across courses without carrying learner identity.

**Consequence.** Tenant context comes only from the authenticated server session. PostgreSQL applies
transaction-local tenant state and forced RLS; membership and learner ownership add narrower access
checks than tenant membership alone.

**Owner.** [DATABASE_TENANCY.md](DATABASE_TENANCY.md),
[SECURITY_MODEL.md](SECURITY_MODEL.md#authentication-and-tenant-derivation), and
`crates/learning-data-access`.

**Planned closure.** Production deployment must still demonstrate the real non-superuser roles,
network boundaries, backups, and managed recovery controls.

### Enrollment is course-level

**Decision.** One opaque PLE `UserId` identifies a learner's account across courses and
institutions. Course membership, tenant-scoped pedagogical `StudentId`, roster metadata, and
assignment enrollment remain separate authorization or educational records. The instructor manages
one course roster; PLE creates the required assignment enrollments and empty summaries atomically
behind that workflow.

**Why.** A learner should retain one PLE account across courses and institutions. Course-scoped
authorization, learner ownership, and RLS control disclosure more reliably than pretending the same
person is a different identity in every class. Verified email is the mutable canonical sign-in
attribute, not the identity key; passkeys are optional convenience credentials for that account.

**Consequence.** An instructor creates a pending invitation with protected course-scoped roster
metadata, then shares its one-time copy link through an existing trusted LMS or uses configured
SMTP. After the learner completes email authentication and claims the invitation, PLE creates the
course membership, tenant learner mapping, assignment enrollments, and empty summaries atomically.
Adding a later assignment preserves the complete student-member by assignment cross product.
Removing access retains education records for the explicit retention workflow. The browser never
asserts a new `UserId`, membership, roster identity, or invitation claim without server-issued
evidence.

**Owner.** [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md),
[IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md), and the course capabilities in
`crates/learning-data-access` and `crates/server/src/course/`.

**Planned closure.** The HTTP roster, invitation claim, bulk preview/commit, and passwordless
account boundary are implemented. Acceptance remains open for the canonical operator-configured
email-provider journey, optional-passkey and multi-replica evidence, and independent security/HCI
closeout.

### Human roles are Student, Instructor, and Sysadmin

**Decision.** PLE has exactly three human roles. Instructor approval requires
real-person validation and direct course membership. Sysadmin is a separate
operator-approved platform role and never substitutes for direct Instructor
membership for general FERPA course access. It has a separate closed and
audited roster-support capability so the operator can help an Instructor
without gaining grade, response, run, export, item-analysis, or ordinary course
authority. Publishing content is an Instructor action; the public-asset
publisher is a service identity, not a person.

**Why.** Ambient administrator or manager roles turn one compromised platform
credential into access to every student's educational record. A publisher
human role also confuses author approval with the least-authority service that
materializes immutable public bytes.

**Consequence.** `UserRole` is the closed Student/Instructor/Sysadmin set;
course membership is the smaller Student/Instructor set. A sysadmin may create
a course and thereby become its direct Instructor, but cannot enumerate or
read another course's teaching records merely because of the platform role.
Roster support records actor/course/action/time for every Sysadmin boundary
crossing. All course-linked student data receives the FERPA radioactive
handling discipline.

**Owner.** [USER_ROLES.md](USER_ROLES.md),
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), and
[DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md).

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

**Decision.** Binary and archival bytes live behind typed server-generated object keys and immutable
`ObjectRecord` evidence; client requests name logical delivery IDs, never buckets or paths.

**Why.** A bucket path is storage implementation detail and an authorization hazard. Typed objects
bind lifecycle, checksum, media type, provenance, and delivery policy to the correct owner.

**Consequence.** Writes verify SHA-256, source and temporary objects are non-deliverable, protected
asset delivery is authorized and audited, and the database records intended existence while storage
proves bytes exist.

**Owner.** [OBJECT_STORAGE.md](OBJECT_STORAGE.md),
[crates/objects/src/lib.rs](../crates/objects/src/lib.rs), and MOD-OBJ in
[CONTRACTS.md](CONTRACTS.md#storage-and-adapter-contracts).

**Planned closure.** Reconciliation of inventory, orphans, and missing referenced bytes remains a
release package. Learner file responses remain fail-closed until their attempt-bound upload
capability and inspection workflow are implemented.

### Privacy deletes records, not learning evidence

**Decision.** Default course lifecycle notifies after 30 days, archives learner records after 100,
and permanently deletes them after 365. Tenant-owned assignment definitions normally remain;
identity-free anonymous aggregates remain available to improve the shared library.

**Why.** Students need privacy by default, while question quality improves only if non-identifying,
non-retractable aggregate evidence survives a learner record's lifecycle.

**Consequence.** Deletion removes the course-owned learner graph and its typed student-record
objects, but never follows immutable assignment references into shared publication. Anonymous
statistics have their own aggregation and k-anonymous disclosure boundary.

**Owner.** [RETENTION_POLICY.md](RETENTION_POLICY.md),
[DATABASE_TENANCY.md](DATABASE_TENANCY.md#retention-and-ferpa-isolation), and MOD-STATS/MOD-RETENTION
in [CONTRACTS.md](CONTRACTS.md#api-and-service-contracts).

**Planned closure.** Institutions may configure a later ordered retention policy. Production backup
retention and recovery objectives require explicit infrastructure choices and evidence.

## Browser and accessibility

### Solid renders; Rust/Wasm validates browser-safe work

**Decision.** SolidJS owns interactive browser composition. Rust/Wasm owns deterministic,
answer-free shared computation such as format validation, timing display, state transitions, and
generated contract support.

**Why.** Solid provides a small reactive UI layer while Rust preserves server/browser consistency for
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
hover-only, coordinate-only, or time-critical required interactions are not eligible for a learner
question; hotspot questions need a pedagogically equivalent keyboard path.

**Owner.** [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md) and the browser
contracts in [CONTRACTS.md](CONTRACTS.md#browser-contracts).

**Planned closure.** Each new response family supplies its own keyboard evidence as it lands; it
does not wait for a generic final audit.

## External systems and evidence

### External grading backends remain private adapters

**Decision.** PLE, never a learner browser, contacts WeBWorK or a contracted external provider. PLE
owns published source, issued seed, response projection, credentials, timeout, sanitization, and
result translation.

**Why.** Upstream systems use their own fields, sessions, HTML, and credentials. Those are neither
stable browser contracts nor safe learner authority.

**Consequence.** The accepted WeBWorK path is the four reviewed Chapter 1 PGML sources, comprising
one radio and one matching question per chapter, via the private external standalone `/render-api`;
browser data is a PLE-native response. Raw source, hidden fields, sessions, provider values, and
renderer output do not cross the PLE browser boundary.

**Owner.** [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md),
[ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md), and MOD-ADP-WW in
[CONTRACTS.md](CONTRACTS.md#storage-and-adapter-contracts).

**Planned closure.** Broader problem compatibility and any unreviewed matching source require their
own accepted projection and live evidence; they are not inferred from the Chapter 1 profile.

### Tests prove behavior at the right layer

**Decision.** Permanent tests are deterministic, offline, behavior-focused evidence. Disposable
live services, container, performance, backup, and visual probes prove environment-dependent
claims once and are recorded rather than retained as brittle routine tests.

**Why.** A permanent test suite must stay trustworthy and fast enough to run often. Exact file
layouts, tunable constants, mock wiring, and live infrastructure can create false confidence or
maintenance burden without proving learner behavior.

**Consequence.** Memory and mock backends support unit and conformance behavior. PostgreSQL RLS,
MinIO, renderer, browser, recovery, and deployment claims use the named disposable oracle or human
acceptance evidence. A source-size gate is permanent architecture evidence because it protects
capability ownership.

**Owner.** [PYTEST_STYLE.md](PYTEST_STYLE.md), [DEVELOPMENT.md](DEVELOPMENT.md#choose-the-right-gate),
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
ordinary memberships, and ordinary learner work through the production-shaped browser and server
stack. Preview, acceptance, and production behavior share this one live product model.

**Why.** A parallel mock product creates false assurance and cannot prove the real teaching path.
Recognizable courses and deterministic observations make returning to the demo resemble checking
on active teaching.

### Teaching workspaces use task-owned composition

**Decision.** Instructor assignment work is composed as focused Overview, Questions, Policies,
Grading operations, and Student-view tasks over one authoritative assignment. Useful desktop width goes to scanning and
editing; the complete current task and primary save action should fit comfortably at 1280 by 800.

**Why.** Task-level hierarchy is easier to teach and operate than a grid of equally padded cards.
Separating content from policy prevents unrelated fields from competing on one page.

### Course appearance derives usable roles from three anchors

**Decision.** A course selects one three-color biome or habitat theme and may add a centered banner
normalized to 1200 by 328 without stretching. The default `grass` anchors are `#73C167`, `#008852`,
and `#BDDEB1`; readable interface roles are derived without changing the stored anchors.

**Why.** The owner wants Blackboard Original-like course identity, not three decorative swatches on
otherwise identical white pages. Derived roles preserve recognizable color while meeting contrast
and accessibility needs.

## Demonstration and release evidence

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
a representative four-question Chapter 1 assignment from published problems, and observe the
Student's submitted and scored work. The complete eight-question sweep is a separate release gate.

**Why.** A focused realistic loop demonstrates first success without substituting a one-question
toy or forcing the full release corpus into every walkthrough.

## Content and grading formats

### Flat JSON is the static-question authority

**Decision.** Versioned PLE flat-question JSON is canonical for MC, MA, FIB, MULTI-FIB, NUM, MATCH,
ORDER, and HOTSPOT. YAML may compile once into that contract; QTI is an import, export, and archival
adapter rather than internal authority.

**Why.** One deterministic cross-language contract avoids competing source models. Adapter formats
can preserve interchange without dictating the engine's internal representation.

### Native interactions adapt the QTI self-test model

**Decision.** Native families borrow the QTI Package Maker self-test's compact task, obvious submit,
visible response state, per-part completion, plain-language feedback, reset, and completed state.
PLE retains server-only grading, labeled controls, keyboard operation, and recoverable errors.

**Why.** Students should learn one clear interaction vocabulary without importing client-side
answers, drag-only controls, result-string protocols, or inaccessible presentation choices.

### Binary question assets use object storage

**Decision.** Images and other binary references keep bytes, checksums, media types, lifecycle, and
authorization in typed PLE object storage rather than JSON or database rows. Optional correct and
incorrect feedback remain shared sidecars and do not determine validity.

**Why.** Typed storage preserves authorization and lifecycle boundaries while keeping the canonical
question contract compact even when author feedback is incomplete.

## Identity, authentication, and compliance

### Visible identifiers are human-readable locators

**Decision.** Visible content, navigation URLs, documentation, and copyable links never expose
UUIDs. Published questions use one non-sequential Crockford Base32 ID displayed as `AAA-BBBB`;
internal UUIDs may remain in hidden server and transport boundaries.

**Why.** People need identifiers they can recognize and communicate. A public reference is a
locator, not authorization, and persistence identity should not leak into the interface.

### Invitations and recovery use verified email

**Decision.** PLE accounts are institution-independent and use passwordless verified email as the
canonical registration, invitation, sign-in, and passkey-recovery path. SMTP delivery is optional;
an Instructor may share a one-time invitation link through a trusted LMS.

**Why.** Email provides one comprehensible account authority without making an institution or
configured mail provider a prerequisite for independent use.

### Authentication storage is strictly necessary

**Decision.** Production authentication uses one host-only `__Host-` HttpOnly, Secure,
`SameSite=Lax`, `Path=/` browser-session cookie with bounded server expiration and immediate
revocation. Persistent login, tracking, or embedded LTI requires separate review and consent.

**Why.** The bearer credential must remain unreadable to JavaScript and limited to providing the
requested signed-in service. Necessary-storage classification still requires clear disclosure and
deployment-specific legal review.

### Security controls preserve privacy and recovery guidance

**Decision.** PostgreSQL, object storage, backups, and deployment volumes use scoped managed
encryption at rest; application AEAD is reserved for stored secrets. Unauthorized users receive
generic unavailable outcomes with accessible guidance that does not disclose protected details.

**Why.** Concealment and humane teaching guidance are complementary. The regulatory basis includes
the EU ePrivacy Directive Article 5(3), Article 29 Working Party Opinion 04/2012, and current ICO
strictly-necessary storage guidance.

## Repository and runtime policy

### Dependency manifests permit current secure releases

**Decision.** Direct registry dependencies use `version = "*"` or an audited open minimum. Caret,
exact, tilde, and upper-bound requirements require a documented repository-specific exception;
lockfiles remain the reviewed exact resolution between deliberate refreshes.

**Why.** The owner prioritizes current security fixes while an open reviewed minimum records the
known-safe floor without blocking later corrective releases.

### Generated output has tracked authority

**Decision.** Reproducible generated output lives under ignored `generated/` and is rebuilt from a
tracked generator or authoritative source before validation. Small reviewed golden baselines may
remain tracked when they define compatibility or durable cross-layer evidence.

**Why.** Ignored output must not become an unverifiable input, while deliberate goldens serve a
different purpose from disposable build products.

### Local-stack replacement is scoped and inspectable

**Decision.** The Python local-stack controller owns project-labelled lifecycle, readiness, and
cleanup. Replacement removes exact-project containers and orphans, retains named data volumes until
their acceptance target permits removal, and prunes only images unused by current containers.

**Why.** The owner's Podman machine is dedicated to disposable project infrastructure, but typed
target, label, and explicit-resource safeguards keep destructive cleanup bounded.
