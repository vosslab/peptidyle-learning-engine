# Design decisions

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

**Decision.** An instructor's mutable draft is tenant-owned. Publication mints a separate immutable
shared problem version; corrections publish a new version rather than editing history.

**Why.** This prevents the classic LMS failure where a later edit changes what an earlier learner
was assessed on, and it lets tenant record deletion leave shared educational content intact.

**Consequence.** Assignments and attempts pin an immutable version. Browser requests never choose a
new published identity, and publication atomically records the immutable payload, provenance, and
visibility state.

**Owner.** [DATABASE_TENANCY.md](DATABASE_TENANCY.md#ownership-boundary),
[SECURITY_MODEL.md](SECURITY_MODEL.md#catalog-publication-boundary), and the identity/catalog rows
in [CONTRACTS.md](CONTRACTS.md#domain-contracts).

**Planned closure.** Later schema evolution follows forward migrations and compatibility readers;
it does not rewrite already published versions.

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
the browser does not resend the question, course, assignment, version, seed, backend, or response
family as authority.

**Why.** An issued attempt already binds learner, tenant, run, immutable version, seed, timing,
policy, response schema, and grading backend. Repeating those values expands traffic and creates
conflicting sources of truth.

**Consequence.** Server code loads and validates the issued attempt before decoding and grading an
answer. The browser uses a compact response and receives a policy-projected receipt rather than a
persistence record.

**Owner.** [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md#attempt-authority),
[crates/question_model/src/activity.rs](../crates/question_model/src/activity.rs), and MOD-API-RUN
in [CONTRACTS.md](CONTRACTS.md#api-and-service-contracts).

**Planned closure.** The atomic minimal learner screen and type-free answer decoder replace the
current broader attempt projection under the payload plan's compatibility and acceptance rules.

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
