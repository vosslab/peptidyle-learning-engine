# Design decisions

<!-- VENDORED HEADER: START -->
Record each durable decision about how this code and repository are shaped, once it is settled, with
the reasoning a later reader needs. Guidance Neil Voss states belongs in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), dated history in `docs/CHANGELOG.md`, open discussion in
`docs/active_plans/decisions/`. [PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

This is PLE's conceptual entrypoint for settled product and architecture decisions. It answers
"why is this boundary here?" and points to the contract that answers "how does it work?" It does
not replace the release direction in [ROADMAP.md](ROADMAP.md), unfinished-work routing in
[TODO.md](TODO.md), acceptance rules in [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md), or the
named code owner.

## How to use the documentation

PLE documentation has three deliberately different layers:

1. **Source authorities** decide what is allowed now: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
   [CONTRACTS.md](CONTRACTS.md), migrations and schemas, and the named code owner. Execution-only
   notes may narrow a work item but never replace those authorities.
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

### Static sources do not carry a variation rule

**Decision.** Static PLE Question JSON and QTI-imported static Questions do not carry a
Question-authored `Static` variation-rule field. Static is a characteristic of the complete
Question Source. The Assignment-owned Question Variation Rule remains the separate choice to reuse
or replace Question Variations in later Assignment Attempts.

**Why.** A source field whose only current value is `Static` repeats what the source format and QTI
profile already establish. It would also overload the Assignment rule that governs later-Attempt
behavior. QTI is interchange input and converts an accepted static item to PLE Question JSON; it is
not a runtime Question Backend or a second source-policy owner.

**Consequence.** PLE Question JSON, QTI mapping, Draft Question, and Question Revision contracts add
no `RandomizationDefinition::Static`, `QuestionVariationDefinition::Static`, or
`questionVariationDefinition`. A future seeded Question Generator must introduce its complete
source, publication, issuance, grading, repair, and reproduction path; it does not add a
Question-authored variation rule. `AssignmentQuestionVariationRule` remains the separate
Assignment-owned choice between Reuse Variation and New Variation.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md), and the PLE and QTI adapters.

### Draft and published source bindings are separate

**Decision.** Draft Question Source Binding and Question Revision Source Binding are
separate relationships and separate persistence boundaries. Each binds one complete opaque
format-specific Question Source to its exact owner together with Question Backend, Question Format,
Source Object Reference, Source Object Checksum, and any exact backend-specific location. Neither
binding has a surrogate UUID.

**Why.** Drafts are high-churn sandbox content with expiration, while Published Question Revisions
are durable shared content. One mixed table would make published source resolution carry nullable
draft ownership, draft update behavior, and cleanup concerns. Qualified bindings preserve the
real owner and lifecycle boundary.

**Consequence.** Draft saves replace only the Draft Question Source Binding. New-lineage
publication resolves and verifies the exact current draft source, then writes the same bytes to a
new immutable Question Revision Object Address. The P1 Store transaction rechecks the exact Draft
Question Edit Number and source facts before it
creates the complete first Question Revision and Question Revision Source Binding. Published reads
use only the latter. The private-authoring baseline directly creates the two qualified Source
Binding tables instead of a mixed owner relationship.
Existing RLS, grants, retry semantics, and typed addresses apply to each exact relationship.
Because PLE is pre-production, migrations in the fresh baseline are current construction authority,
not immutable compatibility history: the earlier baseline must create and operate on the qualified
bindings directly, and later migrations must not translate from or drop the retired mixed table.
P2 implements the server-only new-lineage object-copy coordination. Same-lineage publication,
Question Search isolation, Server Routes, and cleanup remain parent QSOM1 work.

**Owner.** `docs/TERMINOLOGY_CONTRACT.md` and the active fresh-schema migrations.

### References remain scoped locators

**Decision.** Opaque UUID-backed private record IDs and SQL `*_id` keys remain
IDs. A public Reference is a separate reviewed, prefixed, authorization-scoped
locator, and it exists only where a product boundary approves it.

**Why.** Rebranding a UUID as a Reference conceals a different representation
and collapses the distinction between record identity, route location, and
authority. Existing `C-`, `M-`, `A-`, `R-`, and `U-` references prove that a
locator needs its own grammar and authorized resolution boundary.

**Consequence.** `CourseInstanceReference`, `CourseMembershipReference`,
`AssignmentReference`, and `AssignmentAttemptReference` remain separate from
their private IDs. No Student Record, Issued Question, or Question Attempt
Reference is invented. A future locator requires separate product, schema,
resolver, service, wire, and privacy decisions; it does not rename SQL keys or
trusted UUID values.

**Owner.** [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) and
[NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) define the durable naming
boundary; the active release plan owns any future implementation allocation.

**Completion.** `WN1-TERM-LEARNING-REFERENCE-H0` is accepted and completed
after independent review. This documentation-only correction changes no source,
schema, generated contract, route, test, detector, or migration allocation.

### Question agnosticism

**Decision.** PLE is a learning engine, not a question-authoring language or a single renderer.
PLE Question JSON, WeBWorK PG, iMathAS, H5P Packages, and future registered Question technologies
retain their distinct Question Source and Question Backend boundaries behind typed server-side
adapters. QTI import/export/archive is the flat-question interchange pathway. A supported QTI
import becomes PLE Question JSON before it enters the Draft Question and publication lifecycle.
Draft Question remains the shared mutable authoring lifecycle for every supported Question Type,
Question Format, and Question Backend. Its identity and lifecycle are independent of PLE Question
JSON. Authoring, validation, preview, testing, publication, Assignment selection, issuance,
presentation, submission, evaluation, and feedback release use the same PLE contracts for every
Question Backend. Each shared operation resolves the registered backend and delegates its
format-specific work. Publication freezes the selected format-specific Question Source into the new
immutable Question Revision.

**Why.** Biology, genetics, and biochemistry need both reusable static questions and generated
questions without making a vendor format or a browser Question Response Control the platform's core model.

**Consequence.** A new Question Backend adds a bounded adapter, public Question Presentation,
format-specific private evaluation artifacts when needed, and a capability declaration. It does not
spread vendor fields, answer rules, or renderer details through storage, browser DTOs, and UI components.
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
teaching object.

**Consequence.** Assignment Attempt links directly to one Student Record and
Assignment; Issued Question links that pass to its source Assignment Entry and
exact Question Revision; Question Attempt links to one Issued Question; Question
Submission links to one Question Attempt; and Assignment
Submission, when required, links directly to the Assignment Attempt. New
PLE-owned documentation, UI, routes, types, and schema use this full hierarchy.
**Owner.** [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md).
**Planned closure.** The downstream source, schema, Store, API, browser, and
migration work own the coordinated implementation cutover and its acceptance
evidence.

### Mutable Draft Question cut supersedes draft revision history

**Decision.** One mutable Draft Question belongs to one Authoring Workspace. Its private
Draft Question UUID is server-only; its opaque Draft Question Reference supports authorized
Instructor navigation. Each accepted save updates that same Draft Question and advances its positive
Draft Question Edit Number, which is the concurrency token. Its complete opaque format-specific
Question Source and current Draft Question Source Binding belong to that Draft Question; the
binding has no surrogate UUID.
Future authorized atomic publication validates one exact Edit Number, then mints an immutable
Question Revision in the installation-wide Question Library under a stable QuestionId. The
Question Revision Reference is
`{ QuestionId, positive Question Revision Number }` and is also the storage identity.

Draft Questions are isolated sandbox authoring content. They may be incomplete,
invalid, experimental, duplicated, or abandoned. Draft rows, editable metadata,
and Draft Question Source Object References remain in private Authoring Workspace
storage and are absent from Published Question tables and indexes. Draft Question
Metadata and Published Question Metadata use parallel tables with shared field
validation where the facts correspond, while their owner keys, mutability, RLS,
indexes, and retention remain separate. Publication validates the exact Draft
Question Edit Number, copies the accepted values into the Published Question
Metadata table, writes the complete source to a new immutable Question
Revision-owned object path, and stores a new Source Object Reference and
Source Object Checksum for that published object. Mutable Published Question
metadata such as Question Title and Question Description belongs to the stable
Question lineage and may change without creating a Question Revision. Immutable
Question Revision language applies to its source and exact historical evidence,
not to every Published Question metadata field. The Question Revision has no
identity, storage, or lifecycle dependency on the Draft Question or its object path.

Draft Question cleanup uses the last accepted edit time and a configured expiration
policy. Publishing a Question does not make retention of its source Draft Question
necessary; cleanup may remove the draft rows, draft metadata, and draft source object
after any configured recovery period while the Published Question remains complete.

This supersedes retained Draft Question Revision, Draft Question Revision Number, Draft Question
Revision UUID, and Draft Question Revision Reference concepts. It also supersedes Draft Question
Revision ownership for Draft Question Source Binding. The Authoring Workspace remains owned by its
Instructor and shared only through an explicit workspace relationship. The browser receives only the
opaque Draft Question Reference and the Draft Question Edit Number. The Question stewardship decision
below classifies whether a later publication creates another version in that lineage or a fork with a
new QuestionId. Every Assignment's pinned Question Revision remains
exactly resolvable in both Available and Archived states, with availability visible in the
Instructor-safe Question Library view. Publication has one Question Library visibility contract.
Selection eligibility is separate: Available versions appear in ordinary discovery and selection;
Archived versions remain available through exact historical references. Student access remains bound to an
Assignment Access, and anonymous web access receives no Question Library authority.

**Why.** This prevents the classic LMS failure where a later edit changes what an earlier Student
was assessed on, while giving every Instructor an equal path to discover, organize, reuse, and
improve shared educational content. Course-record deletion leaves the Question Library intact.
Keeping drafts private prevents unfinished material from reducing discovery quality.

**Consequence.** Existing Assignments and issued Assignment Attempts retain their exact references. An Instructor
must deliberately replace or opt in to a newer version; no publication, correction, or background
action may advance an assignment. Browser requests never choose a hidden version. Internal
Question Revision Reference evidence supports replay, grading, audit, source history, and authorized
transport only; publication atomically records the version payload, lineage, source history, and
visibility. The assigned `AAA-BBBB` Question ID names the durable lineage. The `(Question ID,
positive Question Revision Number)` pair is the sole immutable content identity used by exact
Assignment and evidence pins.
**Owner.** [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
[SECURITY_MODEL.md](SECURITY_MODEL.md#question-library-publication-boundary), and the Question Library rows
in [CONTRACTS.md](CONTRACTS.md#domain-contracts).

**Implementation boundary.** PLE directly applies the no-drift design while it remains
pre-production. Real native and WeBWorK host-seed publishers mint fresh opaque QuestionRevision
evidence under the stewarded QuestionId lineage, or a new QuestionId for a major semantic fork,
and converge only through a protected manifest or verified existing record. Isolated unit fixtures,
derived render/cache identities, and non-question seed records may remain deterministic. Later
schema evolution uses forward migrations and explicitly versioned protocols; no compatibility reader
preserves retired PLE terminology drift. The no-drift boundary was accepted on the final material tree. M0
remains
open; The Python lifecycle conversion is accepted on 2026-08-15 after required live/full Validation and independent reviews
returned ACCEPT with no P0-P3 finding.

### Published Questions have four stewardship paths

**Decision.** A stable `QuestionId` names one question lineage and each `QuestionRevision` is
immutable. Published-question stewardship has four paths:

1. A Question Owner may publish a validated moderate edit as an immutable
   same-lineage version.
2. Any vetted Instructor may submit a Change Proposal against an exact
   version after Question Publication Validation succeeds. It shows semantic and grading impact; the
   Question Owner accepts or rejects it. A stale base must be rebased or resubmitted; acceptance creates a
   same-lineage version with contributor credit.
3. Any vetted Instructor may create a full fork as a private Draft Question. Question Publication Validation
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
Question Ownership Events form an ordered, repeatable transfer chain. The initial owner records the
initial event, only the current Question Owner records an accepted transfer, and the next owner must
be an Active Instructor Account at transfer time. Ownership grants stewardship authority but never
limits answer-free Question Library visibility for another Active Instructor Account. Question
Owner identity remains server-side unless a future explicitly authorized View needs it.
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
idempotent, Correction-Generation-fenced workers apply the authoritative correction mapping and remediation across all
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
authorized course surfaces. The Sysadmin correction result contains aggregate affected-version, assignment,
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

### Instructor-facing Question identities are operational

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
The server-only new-lineage publisher now generates the six random Crockford characters with the OS
CSPRNG and computes the seventh character with HMAC-SHA-256 under a redacted 256-bit installation
secret. Secret loading, rotation, the publication Server Route, lookup validation, and browser entry remain
their owning composition and Question Library packages; the issuer creates no alternate identity.
**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-philosophy),
[`QUESTION_ID_SPEC.md`](QUESTION_ID_SPEC.md), `crates/question_model/src/question_library.rs`, and
Question Library API in
[CONTRACTS.md](CONTRACTS.md#api-and-service-contracts).

### Assignment work is one aggregate

**Decision.** The Assignment Workspace gives each assignment one exact course-scoped Instructor workspace. Its
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
and return the complete authoritative Assignment result with one new revision. Structural
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
[question workspace](../crates/question_model/src/assignment_workspace.rs),
and [API_CONTRACTS.md](API_CONTRACTS.md#instructor-assignment-workspace).

### BlueprintCourse owns reusable course structure

**Decision.** `BlueprintCourse` is the one canonical course-level reusable aggregate. Use ADAPT's
Alpha wording only as comparison history; PLE names no Alpha product type or compatibility alias.
The creating Instructor owns a private draft through its authoring workspace. After complete
validation succeeds, an explicit publication makes the answer-free `BlueprintCourseView`
visible and reusable to every vetted Instructor. The BlueprintCourse contains ordered modules and
assignments, reusable Blueprint Revision Content, exact published-question pins, and reusable relative schedule
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
CourseInstances as unreleased Assignments and require an explicit instance release; propagation
never silently releases or overwrites delivery state.

Privacy-safe Question Statistics may describe global usage and disclosed learning evidence, but they
never name a private CourseInstance. CourseInstance records, Student activity, grades, and other FERPA
state remain under exact course authorization even when their published question references remain
discoverable in the Question Library.

The private Blueprint Course UUID identifies only the stable Blueprint Course database record. The
Blueprint Course has a separate bounded `BP-` reference number. PostgreSQL identifies an immutable
Blueprint Revision, and every Course Instance, Course Origin, Assignment source, publication,
availability, and collaboration relationship that refers to it, only by the composite Blueprint
Course Reference number and positive Blueprint Revision Number. No Blueprint Revision UUID or
parallel Blueprint Course UUID plus revision identity exists.

**Owner.**
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

**Consequence.** Python `local_stack_control` owns lifecycle and aggregate acceptance. The Python lifecycle conversion
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
[crates/grading/src/lib.rs](../crates/grading/src/lib.rs), and the Question Grading and Question Model Wasm boundaries in
[CONTRACTS.md](CONTRACTS.md#boundary-invariants).
**Planned closure.** Every new Question Backend and Question Type must prove the same closure before its
Question Presentation is accepted.

### The attempt is the grading authority

**Decision.** Question Attempt identity and Authenticated Session bind a Question Submission;
the server loads the complete attempt relationship and durably accepts one immutable private
response before grading.

**Why.** An issued Question Attempt already binds Student Record, Course Instance, Assignment,
Assignment Attempt, Issued Question, immutable Question Revision, seed, timing, policy,
Question Response Format, and grading backend. Repeating those values expands traffic and creates conflicting
sources of truth.

**Consequence.** Server code loads and validates the issued attempt before accepting a response.
The acceptance transaction creates the immutable submission, pending evaluation, execution job,
and receipt; the sealed worker later reloads that private response and grades it. Exact replay and
status reads return the answer-free current `StudentQuestionAttemptView` rather than resubmitting the answer.
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
**Owner.** [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md), [ROADMAP.md](ROADMAP.md), and
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) own migration sequence, release direction, and
acceptance evidence.
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
answer for the exact Question Response Format. `kind` belongs in the render payload so a Question Response Control can be drawn, but the
server derives its response decoder from the issued attempt.
**Owner.** [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md#target-network-contract)
and [OBJECT_STORAGE.md](OBJECT_STORAGE.md#delivery-grants).
**Planned closure.** The payload migration and one-screen `StudentQuestionAttemptView` remain owned by the
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

### Presentation Response Item References have presentation identity

**Decision.** Selectable Response Items receive compact, attempt-presentation-scoped Presentation Response Item References. CRC16 is
an error-detection and correspondence mechanism, never authentication or proof of correctness.

**Why.** A visible label such as `B` is only a position. A Presentation Response Item Reference binds a Question Choice, Ordering Item,
Matching Prompt, Matching Choice, Text Entry Slot, Hotspot Surface, or Hotspot Region to the exact public state the Student saw.

**Consequence.** PLE enforces uniqueness inside one presentation and maintains the authoritative
Response Item Binding to durable Response Item References server-side. A whole-presentation Question Presentation Checksum detects stale or
inconsistent render state; normal session, attempt, RLS, and idempotency controls remain the
security boundary.
**Owner.** [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md#presentation-response-item-references) and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).
**Planned closure.** The codec and migrations land atomically with the minimal response wire format;
no current endpoint treats CRC16 as a bearer token.

## Data and operations

### Repeated mutations use existing identity first

**Decision.** Repeated-request handling belongs to each exact operation. Use its existing record
identity, revision, Request Checksum, Receipt, signed result identity, and database constraints
before adding a separate Retry Token. A qualified Retry Token is allowed only when an implemented
Store and Server Route demonstrate a concrete request that those existing facts cannot identify
safely. The HTTP `idempotency-key` header remains transport vocabulary.

**Why.** A universal Retry Token model creates parallel identity and matching rules for operations
that already have unique durable subjects. That increases implementation and validation work
without improving correctness. The exceptional operation can add a narrow token later without
redesigning unrelated operations.

**Consequence.** The pre-production `RequestRetryToken`, `QuestionSubmissionRetryToken`,
`CourseRosterRetryToken`, and `ImathasResultExchangeRetryToken` candidates were removed after
operation-by-operation review found no demonstrated need. A browser-only type, future contract, or
unaccepted persistence slice does not establish need. Add a qualified token only with its exact
Store, Server Route, conflict behavior, and acceptance evidence; otherwise use the operation's
existing identity and constraints.

**Owner.** [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md), the exact operation contract, and its
implemented Store and Server Route.

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

**Decision.** One opaque PLE Account UUID names a global Student Account across courses and
semesters. Its institutional email is immutable for the lifetime of that Student Account. A
Student Record belongs to exactly one Student Account and Course Instance. Course Enrollment
creates a Student Course Membership episode bound to that stable Student Record; re-enrollment
creates a new membership episode bound to the existing record.
An Assignment Attempt directly binds that Student Record to one Assignment. An Assignment Grade
binds the same pair and selects its contributing Assignment Attempt. Assignment lists and empty
activity states are derived from Active Student Course Membership, Assignment Status, and
effective access rules.

**Why.** A Student retains one global PLE Account across courses and semesters. The Account UUID
is the stable identity, and its immutable institutional email supplies passwordless sign-in and
Course Roster Import matching. Course-scoped authorization, Student ownership, and RLS control
disclosure through separate Student Records and Student Course Memberships. Passkeys are optional
convenience credentials for that Account.

**Consequence.** The planned Course Roster Import transaction will use each
institutional email to resolve an existing Student Account or create one when none exists, then
complete its Store, route, invitation, and Course Enrollment transaction atomically. Current
authentication ceremonies authenticate existing Accounts only. An authorized pre-activity Assignment read returns
an empty `AssignmentProgress` result. Starting an Assignment Attempt creates the direct Student
Record-to-Assignment activity relationship transactionally; calculating a Grade creates its exact
grade record. New Assignments add Assignment Content, while Student rows appear with actual Student work.
Student Work Records and Grades follow the Course Retention Plan independently of the Student
Account's lifetime. Server-issued evidence establishes every Account, Course Membership, Student
Record, and invitation claim.
**Owner.** [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md),
[IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md), and the course capabilities in
`crates/learning-data-access` and `crates/server/src/course/`.

**Planned closure.** Course Roster Import delivery must implement the Course Roster Import Service,
invitation claim, and passwordless enrollment flow with their Stores, Server Routes, and transaction
proof. The current baseline exposes none of those Server Routes. Operator-configured email-provider, optional-passkey,
multi-replica, security, and HCI evidence remain open.

### Course Roster change numbers stay exact

**Decision.** Course-bound browser contracts use `rosterChangeNumber`, typed as the generated
`CourseRosterChangeNumber` canonical positive PostgreSQL-BIGINT decimal string. The browser emits
and verifies its strong ETag as the exact quoted decimal.

**Why.** The containing Course scope supplies the subject, so repeating it in the member name adds
no ownership. A JavaScript number could lose valid PostgreSQL-BIGINT precision, while the prior
`rosterRevision` wording incorrectly implies retained roster revisions.

**Consequence.** Course Roster pages, roster aggregate actions, invitation email-rule responses,
roster-import responses, calculated Gradebook pages, and submitted Assignment Attempt chooser pages
use the same exact string. `CourseInvitationStatePrecondition`, `importRevision`, scheme revisions,
and scoring generations remain distinct contracts.

**Owner.** `generated/api/CourseRosterChangeNumber.ts`, `src/api/enrollment.ts`, and the Course
Roster and Gradebook browser decoders own the current browser boundary.

### Current personas are Student, Instructor, and Sysadmin

**Decision.** Each PLE account has exactly one immutable current Student, Instructor, or Sysadmin
role. A person needing multiple roles uses separate accounts; Dr. Voss may use separate Instructor
and Sysadmin accounts. Instructor Vetting is real-person validation before the Sysadmin Create Instructor Account operation, and teaching requires
direct Instructor membership. A Sysadmin creates a Course Instance only for an explicitly assigned
active Instructor account, which receives the initial membership; the Sysadmin receives none.
The later verified Instructor Authentication Email replacement operation may replace that email; the
same Instructor Account retains its Product Role, Question authorship and ownership, Authoring
Workspace relationships, Course Memberships, authored content, and teaching history.
Course help uses an explicit, audited, time-bounded support capability with a stated purpose.
Sysadmin has no ambient FERPA browsing. Publishing content is an Instructor action; the
public-asset publisher is a service identity, not a person. Every active Instructor has the same
product capabilities, including shared Question discovery, Question Folders, publication, reuse, and
improvement workflows.

**Why.** Ambient administrator or manager roles turn one compromised platform
credential into access to every student's educational record. A publisher
human role also confuses author approval with the least-authority service that
writes and verifies immutable public objects before activation.

**Consequence.** Product Role is the closed Student/Instructor/Sysadmin set, represented by
`ProductRole` in code, and Account/session storage carries one Product Role, never a collection. Course
Membership is the smaller Student/Instructor relation and must match Product Role. Sysadmin Accounts
cannot hold Course Membership. A Course
may have multiple current Teaching Team Member accounts with equal teaching authority. A support capability
names the exact course and, when needed, Student; it expires on a recorded deadline and records
the authenticated account, purpose, action, and time for every boundary crossing. All course-linked Student data
receives the FERPA radioactive handling discipline. The Course Membership,
support-capability, and FERPA handling boundaries require their own implementation
and acceptance evidence.

**Session issuance rule.** The Authenticated Session issuance operation accepts an existing Account identity
and opaque session parameters, then derives Product Role from the immutable Account row in the same
trusted transaction. A passwordless ceremony, browser request, or adapter never selects Product
Role. The resulting Authenticated Session stores the derived role and remains bound to that Account
for its lifetime. This keeps the fixed-role decision at the trusted service boundary (ASVS 2.2.1,
7.2.1, and 8.3.1). The `2026082906` Authenticated Session Resolution function and `SessionStore` implement this derivation; the
passwordless ceremony and connected authentication acceptance remain separate work.

The authorization boundary remains capability-oriented so a later package can add bounded Grader,
Course Observer, or Student Observer relationships without widening the current personas. A Course
Observer receives anonymous aggregate grades and no Student-level FERPA records. Each future
relationship lands with its visible workflow, revocation, audit, and privacy contracts.

**Owner.** [USER_ROLES.md](USER_ROLES.md),
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), and
[DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md). Accepted accountable-course-assignment evidence
is recorded in [CHANGELOG.md](CHANGELOG.md).

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

**Owner.** [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md), the course-membership schema, and the
teaching-authority Store contract.

### APIs are stateless; durable state is shared

**Decision.** Any API replica can serve an authenticated request. Durable state lives in PostgreSQL,
object storage, and the queue; a browser copy or a replica's memory never establishes authority.

**Why.** Scale should come from adding replicas and surviving process restarts, not sticky sessions
or a privileged in-memory coordinator.

**Consequence.** Sessions, attempts, idempotency receipts, leases, and prefetch ownership are
durable. Replica recovery has explicit fencing rules, and workers use lease/generation boundaries.

**Owner.** [MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md),
[CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md#current-and-target-topology), and Background Job Execution in
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
[crates/objects/src/lib.rs](../crates/objects/src/lib.rs), and Object Storage in
[CONTRACTS.md](CONTRACTS.md#storage-and-adapter-contracts).

**Planned closure.** Inventory checks, orphan cleanup, and handling of missing referenced bytes
remain a release package.

### Privacy deletes records, not learning evidence

**Decision.** The owner defaults for the CourseInstance lifecycle are notice after 30 days, archive
Student records after 100 days, and permanent deletion after 365 days. Course-owned assignment
Assignment Content normally remains; identity-free anonymous aggregates remain available to improve the
shared library.

**Why.** Students need privacy by default, while question quality improves only if non-identifying,
non-retractable aggregate evidence survives a student record's lifecycle.

**Consequence.** Deletion removes the course-owned student graph and its typed student-record
objects, but never follows immutable assignment references into shared publication. Anonymous
statistics have their own aggregation and k-anonymous disclosure boundary.

**Owner.** [RETENTION_POLICY.md](RETENTION_POLICY.md),
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#radioactive-records-and-retention), and
Question Statistics and Course Retention in [CONTRACTS.md](CONTRACTS.md#api-and-service-contracts).

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
[SECURITY_MODEL.md](SECURITY_MODEL.md#compile-time-closure), and Question Model Wasm and Browser API Client in
[CONTRACTS.md](CONTRACTS.md#platform-contracts).

**Planned closure.** New Wasm exports require an explicit security and generated-contract review;
performance work follows measured need rather than speculative porting.

### Keyboard is the primary student path

**Decision.** Every student action works without a mouse. Tab and Shift+Tab move focus; Space uses
native selection or activates a focused button; native links retain Enter. Arrows, digits,
Enter-to-submit, and Escape are optional Question Response Control extensions.

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
owns published source, issued seed, Student Response result, credentials, timeout, sanitization, and
result translation.

**Why.** Upstream systems use their own fields, sessions, HTML, and credentials. Those are neither
stable browser contracts nor safe student authority.

**Consequence.** The accepted WeBWorK path is the four reviewed Chapter 1 PGML sources, comprising
one radio and one matching question per chapter, via the private standalone `/render-api` Question Backend;
browser data is a PLE-native response. Raw source, hidden fields, sessions, Question Backend values, and
renderer output do not cross the PLE browser boundary.

**Owner.** [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md),
[ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md), and WeBWorK Question Backend in
[CONTRACTS.md](CONTRACTS.md#storage-and-adapter-contracts).

**Planned closure.** Broader WeBWorK Question compatibility and any unreviewed matching source require their
own accepted Question Presentation and live evidence; they are not inferred from the Chapter 1 profile.

### H5P Package Import retains its minimal QSOM1 adaptation

**Decision.** H5P is the `h5p` Question Format and bounded H5P Package Import path.
**Why.** Its immutable archive, checksum, content type, and import fingerprint retain archival evidence for an unpublished, key-free, ungraded practice payload; it has no server validation, issue, reproduction, or automated-grading lifecycle.
**Consequence.** H5P retains its exact format-specific archive/source boundary with only the minimal
QSOM1 adaptation. It creates no generic PLE source fields, generic Assignment attempt/time controls, or
generic grading-rule facts. Its importer retains hostile-input archive validation, immutable archive
resolution, checksum verification, and unsupported-feature refusal; graded Questions use an approved
Question Backend.
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
by 800 CSS-pixel desktop 16:10 viewport profile. The historical screenshot filename label `laptop`
identifies that exact 1280 by 800 evidence profile. Student design also covers 800 by 1280 portrait
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

**Decision.** The Instructor Product Ribbon has three ordered slots: Courses, Question Library,
and Blueprint Courses. Account and Profile are Ribbon Context Controls. The Question Library
interface area links to All Questions, My Questions, My Question Drafts, Starred, and Watched.
All Questions and My Questions are Published Question Library Views; My Question Drafts navigates
to the separate private Authoring Workspace Store. Starred and Watched are exact Account
relationships to Published Questions.

**Why.** The Product Ribbon stays organized by primary object type. Ownership, publication state,
endorsement, and notification subscription remain distinct destinations instead of becoming
competing top-level repositories. Interface adjacency does not merge private Draft Questions into
the Question Library.

**Consequence.** Question Folders, Question Tags, Saved Question Searches, and search facets
organize or find Questions within those views. Star means visible endorsement, and Watch
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

### Student View Scenario admission preserves its branch

**Decision.** An allowed Assignment Delivery Preview evaluation reports the
closed `StudentViewScenarioAdmission` output. The selected-Student origin pairs
only with `SelectedStudentActiveStudentCourseMembership`; the hypothetical
origin pairs only with `HypotheticalStudentViewScenarioAdmission`.

**Why.** Active Student Course Membership proves a real Student Record's
relationship to a Course Instance. Hypothetical scenario admission proves only
Course and Assignment scope. A shared output field named after selected
membership would claim the wrong proof for an identity-free scenario.

**Consequence.** The browser contract rejects cross-paired origin and admission
values. `ActiveStudentCourseMembershipGrantReason` continues to describe actual
selected-membership surfaces. The hypothetical branch retains its private
admission evaluator and policy decision. The output has no person locator or
authority token. The declared Assignment Delivery Preview route, Store, schema,
PostgreSQL persistence, fixture, and browser feature do not exist yet.

**Owner.** `crates/question_model/src/preview_plane.rs` owns the public output;
`crates/domain/src/preview_plane.rs` owns branch evaluation; and
[CONTRACTS.md](CONTRACTS.md) owns the browser contract and Browser Surface availability status.

### Locked job targets carry authorization ownership

**Decision.** Every durable job has one server-resolved immutable typed target
in addition to its closed handler kind, generation fence, and opaque current
lease. Course work records its exact Course Instance UUID and Assignment or
Assignment Attempt UUID;
workspace work records its Authoring Workspace UUID and import when applicable;
Question Library work records its exact immutable Question Revision Reference. A future approved
Assignment Export service records its own exact Course Instance, Assignment Revision, frozen
Manifest, and Artifact identities before it creates work. A worker
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

### Assignment Export enters only as a complete typed service

**Decision.** PLE has no current Assignment Export service. The Assignment Export stub removal establishes
that baseline by removing unused export tables, export-only Job target members, and present-tense
service claims. The package includes the former route/Store/worker/delivery claims in `SECURITY_MODEL`,
identity claims in `IDENTITY_CONTRACTS`, authorization and retention claims in
`DATABASE_AUTHORIZATION`, worker/storage/classification/audit claims in `MULTI_SERVER_SETUP`,
`OBJECT_STORAGE`, `DATA_CLASSIFICATION`, and `AUTHORIZATION_CONTRACTS`, the Background Job
Execution consumer in `CONTRACTS`, and the active plan/customer-spec request, worker, and
milestone claims. The answer-key-free DOCX/PDF renderer and QTI interchange remain independently
implemented. A future
Assignment Export Manifest is a server-created private immutable typed frozen input for one exact
Assignment Revision; it is not an Object ID or preparatory schema.

**Why.** The retired records name an Assignment Export Reference, Manifest, Artifact, Format, and
State without a Store, route, worker, browser contract, or authorized delivery. An opaque object
reference cannot express the immutable selection and private-data authority a real export needs.

**Consequence.** A future service introduces its Manifest, ordered Question Revision selection,
printable Question Asset Object References and Checksums, format and component-release members,
and per-format private-input allow-lists as one authorized transaction. It also supplies requester
authorization, retry binding, least-privilege lease-scoped execution, private delivery, `no-store`
status/download projection, retention, redacted audit evidence, and connected acceptance. ASVS
2.1.1--2.3.4, 8.1.1--8.3.2, and 14.1.1--14.2.7 require evidence at that admission.

**Owner.** The future Assignment Export service package owns its domain/schema/Store/route/worker
boundary and the PostgreSQL catalog, least-privilege, object, and connected acceptance evidence.

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
a representative four-Question Chapter 1 Assignment from Published Questions in the Question Library, and observe the
Student's submitted and scored work. The complete eight-question sweep is a separate release gate.

**Why.** A focused realistic loop demonstrates first success without substituting a one-question
toy or forcing the full release Question Library into every walkthrough.

## Content and grading formats

### iMathAS Session, Context, and evidence have one owner

**Decision.** Question Model owns the typed `ImathasQuestionBackendBinding`: iMathAS Deployment Reference, iMathAS Item Reference, and the pinned `imathas_remote_grading_v1` profile.
LDA owns the sole server-only iMathAS Question Backend Session, persists that exact binding, and owns its typed Session Reference, preparation/restore/lease/iMathAS Result Exchange Store boundary, and XChaCha20-Poly1305 backend-state protection with rotation.
The iMathAS adapter owns iMathAS Launch Reference, iMathAS Launch State protocol bytes, iMathAS Render Cache Entry, and iMathAS Launch/Result HMAC and protocol verification. LDA mints the Session's OS-CSPRNG 256-bit Challenge, which iMathAS carries only as signed `ple_launch_challenge`.
`ImathasGradingContext` remains exactly its redacted non-Serde `{ QuestionAttemptId, QuestionRevisionReference, QuestionSeed }` triple, expires with its Session, and preserves `authentication_payload_v1`. The Session stores authentication and Result lifecycle facts and binds QuestionAttemptId; the atomic worker commit locks the selected IssuedQuestion, resolves its point_value and scoring_rule, and combines those Assignment facts with backend QuestionEvaluation to write the Assignment-owned GradingResult.
The iMathAS Result Token and checksum are LDA evidence after server-to-server verification; raw bytes never persist or enter browser/generated/log/Debug output.

**Why.** One owner lets `2026090102` enforce exact restore, RLS, forward iMathAS Session/Result Exchange transitions, and four-axis context mismatch refusal without a parallel adapter or browser identity boundary. The browser launch shell accepts only validated `{ launchUrl }`; its LDA-backed Rust route, cookie/env backend composition, and live-backend acceptance remain separate work.

### iMathAS Result uses Ready-to-Commit then worker commit

**Decision.** The approved durable model is Ready-to-Commit plus worker-leased idempotent grading commit. A Question is never Remote or External; `ImathasQuestionBackend`/`imathasQuestionBackend` is the exact renamed response/control/Student Response marker. After iMathAS verification outside PostgreSQL, authenticated staging consumes the exact active iMathAS Session and atomically writes the iMathAS Result Exchange's finite `[0,1]` nonnegative-zero normalized-score-only iMathAS Result, its LDA checksum `SHA-256("ple:imathas-result:v1\\0" || IEEE-754-binary64(score))`, separate iMathAS Result Token checksum, the marker Question Submission, pending Question Submission Grading, and ready typed `grade_accepted_submission` Job. A worker holding that exact Job lease rechecks the lineage and atomically derives the PLE Grading Result plus LDA-owned redacted/non-Serde Automated Grading Receipt Checksum from the fixed v1 prefix, lineage UUID bytes, two Result Exchange checksums, correct byte, canonical big-endian binary64 points, and signed big-endian commit milliseconds; the same transaction writes the Receipt, completes the Job, marks grading graded, and advances the iMathAS Result Exchange to committed.

**Why.** Ready-to-Commit survives interruption without another backend request; an expired Job lease permits a later claim. Final execution failure belongs to the Job and Question Submission Grading (`instructor_attention`), retaining immutable ready evidence for a separately authorized recovery Job. Exact matching staging/commit replays are idempotent; committed replay returns the stored Receipt, Result, and checksum rather than accepting a candidate checksum. The checksum is never command/API/browser/adapter input. The iMathAS Result belongs to its iMathAS Result Exchange and is distinct from raw-token evidence and PLE Grading Result. LTI remains future registered-protocol planning with no current record or schema.

**Consequence.** RQB2 directly amends fresh migration `2026090102`; no alias or compatibility layer is retained, and the accepted submission, lifecycle, relationship, procedure, browser-launch, security, and test boundaries keep their behavior. **Owner.** [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md).

### PLE Question JSON is the static-Question authority

**Decision.** PLE Question JSON version 3 is the sole PLE Question JSON reader and is canonical for
MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. Its source excludes points, Question Attempt
Limit, and Question Attempt Time Limit; the exact Assignment Entry owns those facts. YAML may compile
once into that contract. QTI is an import, export, and archival pathway rather than a stored runtime
Question Format or Question Backend. An accepted Workspace Import validates and maps each supported QTI
item into one complete PLE Question JSON Draft Question.
When that Draft Question is published, its Question Revision owns the mapped PLE Question JSON as
its immutable Question Source and the PLE Question Backend presents and evaluates it. Published
Questions backed by WeBWorK, iMathAS, H5P, or another registered technology retain their own
complete Question Source and Question Backend boundary. The QTI-to-PLE Question JSON mapping
applies specifically to supported flat QTI imports.

The original QTI package, QTI Profile, QTI Package Item Reference, mappings, warnings, checksums, and
vendor points remain Workspace Import evidence. QTI export may be generated from PLE Question JSON
where the supported mapping preserves its meaning. A future explicitly authored PLE Question JSON
Accessibility Alternative may serve a Question whose primary source uses WeBWorK, iMathAS, H5P, or
another registered technology. It uses the shared backend-agnostic Question operations and remains
separate future implementation and authoring work.

**Why.** One deterministic cross-language contract avoids competing source models. QTI preserves
interchange and import evidence without dictating runtime storage, presentation, or grading.

### Native interactions adapt the QTI self-test model

**Decision.** PLE Question Implementations borrow the QTI Package Maker self-test's compact task, obvious submit, visible response state, per-part completion, plain-language feedback, reset, and completed state. PLE retains server-only grading, labeled controls, keyboard operation, and recoverable errors.

**Why.** Students should learn one clear interaction vocabulary without importing client-side answers, drag-only controls, result-string protocols, or inaccessible presentation choices.

### Binary question assets use object storage

**Decision.** Images and other binary references keep bytes, checksums, media types, lifecycle, and authorization in typed PLE object storage rather than JSON or database rows. Optional feedback remains part of the complete format-specific Question Source; its Question Backend may derive Question Feedback for an authorized release.

**Why.** Typed storage preserves authorization and lifecycle boundaries while keeping the canonical question contract compact even when author feedback is incomplete.

## Related decisions

### Question Library Browse is browser presentation, not Question Search transport

**Decision.** Generated `QuestionSearchRequest`, `QuestionSearchResult`, and
`QuestionSearchPage` name only the Question Model transport contract. The
flattened answer-free browser contract is the `QuestionLibraryBrowse*` family:
row, evidence, facet aggregate, query, page, repository, state, session,
decoder, normalization, virtual-window helper, and page-item bound. The
production API repository is the sole explicit generated-to-browse adapter.
`QuestionSearchAuthorship` remains generated vocabulary in the browse query,
and `questionSearchRequest()` remains the server-request constructor. No alias
or dual local/generated browser shape is permitted.

**Why.** The generated result is `{ summary, evidence }` and the generated
page carries server facets, whereas the browser contract has display text,
author names, capabilities, browser evidence, and aggregates. Calling both
shapes Question Search makes wrong imports and wrong-shape calls plausible.
The available Library, Question Picker, and Assignment Editor need the flattened
presentation shape, while accepted QC2 removed the Question Curation aggregate;
stale Graphify Curation edges are not consumer authority.

**Consequence.** This direct terminology cutover changes no Store, route,
schema, generated transport source, fixture, or behavior. Independent QLB1
review passed, and the browser-local/generated Question Search collision is
closed.

The settled identity, authentication, privacy, recovery, and Blueprint-collaboration decisions are retained in [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md). The focused local-stack, Gradebook, wire-contract, and Blueprint-operation decisions are retained in [DESIGN_DECISIONS_OPERATIONS.md](DESIGN_DECISIONS_OPERATIONS.md).
