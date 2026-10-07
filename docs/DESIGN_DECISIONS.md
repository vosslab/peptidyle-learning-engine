# Design decisions

<!-- VENDORED HEADER: START -->

Record each durable decision about how this code and repository are shaped, once it is settled, with
the reasoning a later reader needs. Guidance Neil Voss states belongs in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), dated history in `docs/CHANGELOG.md`, open discussion in
`docs/active_plans/decisions/`. [PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

This file records the durable rationale that supports
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Human Guidance is the authority for
current product intent. Code, schemas, screenshots, tests, changelogs, and old
plans are evidence about implementation or history; they do not override it.

## How to use this document

- Use the decision and consequence together.
- Follow linked owner documents for implementation detail.
- Treat a current source identifier that uses obsolete vocabulary as a migration
  gap, not as a competing product decision.
- Do not infer a feature from a generic table, enum, worker, capability, or
  mockup.
- If Human Guidance does not resolve a choice, record it as unresolved instead
  of expanding this file by inference.

## Product model

### Assessment is the generic activity object

**Decision.** PLE calls the generic object an Assessment. Assignment is not an
object, category, or parent Type; it appears only inside the Assessment Type
names Weekly Assignment, Unit Review Assignment, and Bonus Assignment.
Quiz and Exam complete the five current Types.

**Why.** A single generic noun keeps course content, attempts, navigation, and
data relationships understandable while Types communicate teaching purpose.

**Consequence.** Schema, Store/server contracts, HTTP routes, DTOs, and
browser paths use Assessment as the generic object. Assignment remains only
in Weekly Assignment, Unit Review Assignment, and Bonus Assignment.
New documentation and UI must not reintroduce Assignment as the generic object.

### DD-A9-01: Assessment terminology uses a direct preproduction cutover

**Decision.** Before the first production deployment, PLE changes every generic
`assignment` identifier to Assessment directly: model and schema names, Store
and server contracts, routes, DTOs, API and Blueprint JSON. Canonical browser
families are `/assessments/due-soon`,
`/courses/:courseInstanceId/assessments/:assessmentId`,
`/instructor/courses/:courseInstanceId/assessments/new`,
`/instructor/courses/:courseInstanceId/assessments/:assessmentId/{questions,properties,student-view,delivery-check}`,
and `/assessment-attempts/:assessmentAttemptId`. Canonical API families are
`/api/assessments/due-soon`, `/api/course-instances/{course_instance_id}/assessments...`,
and `/api/assessment-attempts/{assessment_attempt_id}...`. Public configuration is
called Assessment Properties, not policies. Assignment remains only in the
three Type display names: Weekly Assignment, Unit Review Assignment,
and Bonus Assignment.

**Why.** The product model has one generic activity object. Keeping competing
generic vocabulary in its public contract would make the object model and
teaching language ambiguous.

**Consequence.** This is a coordinated preproduction base-schema cutover, not
a compatibility migration: do not add SQL compatibility views, dual DTOs,
dual import shapes, dual API shapes, or a mixed-nomenclature reader. Preserve
existing public `AXXXXXXXZ` Assessment IDs, Attempt UUIDs, and the five
Assessment Type enum values. A failed preproduction rollout rolls back the
whole deployment and its resettable preproduction database together; it never
leaves a partially renamed public boundary.

Canonical JSON names are `assessment`, `id` or nested `assessmentId` /
`courseInstanceId` for those public IDs, `assessmentAttempt`, `assessmentEntry`,
`assessmentStatus`, Blueprint JSON `assessments`, and
`blueprint_assessment_id`; canonical decoders accept only those names.
There is no parallel `reference` property and no runtime legacy Blueprint
import. A one-time, offline developer export-transform-import may assist the
preproduction rebuild, but a fresh installation is canonical from the start.

There is no legacy browser redirect or API compatibility layer. The
preproduction rebuild uses the canonical Assessment routes directly.

Use focused connected tests to prove the coordinated rebuild. Retain a
permanent test only when it protects an intentionally stable external contract,
such as the canonical path or a preserved public identity, and it meets every
criterion in `PYTEST_STYLE.md`; otherwise remove the check at plan closeout. A
failure of a retained canonical-contract test means the public Assessment
contract regressed and must be repaired before closure. Caller, API, import,
inventory, and fresh-install checks remain temporary and are removed at
closeout; no grep or rename check becomes a permanent test.

**Owner.** `docs/TERMINOLOGY_CONTRACT.md`, `docs/CONTRACTS.md`, and the
Assessment route/schema contracts.

### Reusable and delivered course content are different objects

**Decision.** A Blueprint Course contains Blueprint Assessments and no Students
or dates. A Course Instance contains Course Instance Assessments and Student
relationships. An Instructor-owned Assessment Template is reusable Assessment
settings outside either Course kind and contains no Questions or Pools.

**Why.** Reuse, teaching delivery, and personal templates have different
ownership and privacy boundaries.

**Consequence.** Adoption copies Blueprint content into the Course Instance and
retains exact Blueprint Revision provenance. New Blueprint Revisions are
offered to daughter Courses for Instructor review; existing Assessment changes
are never applied silently. A newly added Blueprint Assessment is automatically
copied as an Unreleased Course Instance Assessment.

### Current state is not a hidden revision family

**Decision.** Published Questions and Blueprint Courses use numbered Revisions for saved
content. Permitted metadata fields can change without creating another Revision; each object
defines those fields. Draft Questions, Course Instances, Assessments, Attempts,
Student Work, names, and lifecycle metadata use current state. Edit Numbers are concurrency
controls, not historical content.

**Why.** History is valuable only where the product needs exact reusable or
submitted evidence. Universal snapshots create cost and a misleading object
model.

**Consequence.** New revision, snapshot, receipt, event, or replay types require
a specific Human Guidance-compatible need. Generic auditability is not enough.

### BiologyProblems.org algorithmic WeBWorK migration is forward-only

**Decision.** Each BiologyProblems.org WeBWorK Problem family records one
canonical algorithmic author source-an official PG/PGML file or its author
generator-and exactly one canonical algorithmic PG/PGML file and normal
Published Question lineage. It replaces generated static variants, including
the HLA family's 199 static variants. Algorithmic Questions ordinarily stand
alone, though an Instructor may deliberately group distinct similar algorithms
in a Question Pool when selection is useful. The currently shipped 119
static banks remain unmigrated. The manifest's current 13 algorithmic-source
definitions, including HLA, are migration inputs rather than runtime proof or a
completed migration.

Only after per-family source acceptance, representative deterministic
rendering/grading, and the expected-current Blueprint Revision CAS may PLE move
current catalog placements. A Pool membership edit changes only to remove
redundant generated variants; it retains any intentionally pooled distinct
algorithmic Questions. PLE then Archives replaced static Question and redundant
Pool lineages through ordinary availability and removes generated source copies.
Exact immutable Question Revisions, Blueprint pins, Pool IDs, Pool Edit Numbers,
and Student Work remain resolvable. PLE never deletes, repurposes, or
raw-SQL-rewrites history.

**Why.** Algorithmic practice needs one reproducible source and ordinary
publication without changing previously delivered or pinned content. Pool
selection among distinct Questions and backend-native variation are independent
forms of variation.

**Consequence.** A content-manifest or Blueprint Revision CAS failure leaves
the current catalog unchanged. Recovery is forward-only through ordinary
availability and a later Blueprint Revision. One ignored per-family proof
records author-source provenance/hash/license, exactly one target PG/PGML file
and lineage, source acceptance, intentional-Pool preservation, redundant-Pool
transitions, pin resolution, archival behavior, and CAS stop. It becomes permanent only if
historic archived-pin resolution satisfies every `PYTEST_STYLE.md` criterion;
otherwise it is removed at plan closeout. C824-C841 own this work.

### Create Blueprint from Course Instance preserves both identities

**Decision.** A Course Instance has deliberately entered short and long names,
not names derived from a parent Blueprint. It starts empty or from a Public
Blueprint and always has at least one assigned Instructor. **Create Blueprint
from Course Instance** creates a separate actor-owned Private Blueprint Course
at Revision 1 from that Course Instance's reusable structure.

**Why.** A teaching Course needs its own identity and delivery context, while a
Blueprint contains reusable Course structure without Student records or dates.

**Consequence.** The new Blueprint records immutable source provenance and the
originating Course Instance as its first Adoption. The source Course Instance
remains the same teaching Course, unchanged and addressable. Students, dates,
releases, Student Work, and other delivery state are not copied.

## Accounts, roles, and authorization

### User roles are global and exclusive

**Decision.** Each global Account has exactly one immutable User Role:
Student, Instructor, or Sysadmin. A person needing multiple roles uses separate
Accounts.

**Why.** Role-specific interfaces and access rules remain explicit.

**Consequence.** Course relationships do not change the Account's User Role.
Future Course Observer, Student Observer, and Grader roles are separate Course
relationships, not User Roles. Grader is not currently needed because
grading is automatic.

### Course authority comes from relationships

**Decision.** All current co-Instructors in a Course Instance have equal
teaching and FERPA authority. The creator or first Instructor has no greater
authority. Student access is limited to active Course relationships and the
Student's own record.

**Why.** A privileged Course owner would contradict ordinary co-teaching and
make staff changes unsafe.

**Consequence.** A route ID or visible Course reference never grants access.
The server and database rederive the exact Account, Course relationship,
Student record, and operation predicate.

### Instructor Profiles are visible within PLE

**Decision.** Everyone with a PLE Account can view Instructor Profiles, including
Students viewing Question authors or Question Pool owners. Instructor Profile images follow
that visibility without a separate permissions mechanism. Instructor vetting
happens before Account creation, outside PLE's Account setup workflow.

**Why.** Human Guidance explicitly makes Instructor Profiles public within PLE.
Star lists do not define the audience for an Instructor's identity.

**Consequence.** Remove the former Star-list-only identity rule and its unsupported
locked-name requirement. Existing implementations of those restrictions need
reconciliation; this decision does not claim they have already changed. Watch
lists remain private, and viewing a Profile does not expose Student records.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#instructor-profile-visibility).

### Sysadmins have full administrative authority

**Decision.** Sysadmins have full administrative authority across PLE, including content,
Accounts, Courses, and Student records. A Sysadmin initiates repairs under their own authority;
the Sysadmin role supplies the access needed for that work. Support work is recorded for audit.

Much of the Sysadmin workflow beyond Instructor Account creation remains deferred
until concrete needs are established. The authority rule does not authorize
speculative support tools or make their implementation a current requirement.

**Why.** Neil: "Sysadmins have god powers, I see no way around iit." The earlier rule that denied
general administrative access did not express his intended role.

**Consequence.** Sysadmin access checks provide full administrative access to Course and
Student records. Course Instructor relationships identify the teaching staff.

### Sysadmin sessions require a TOTP-bound second factor

**Decision.** A Sysadmin session requires a time-based one-time-password
(TOTP) second factor. Human Guidance leaves the authentication mechanism as
implementation latitude; this decision selects TOTP for the higher-consequence
Sysadmin session boundary. Student and Instructor authentication remains
unchanged.

**Why.** Platform administration needs a stronger session ceremony without
turning the Sysadmin role into academic authority or changing ordinary teaching
access.

**Consequence.** Primary authentication creates only an opaque pending-MFA
state. PostgreSQL derives the Account's stored User Role and atomically
creates a Sysadmin session only after it requires and consumes one unused,
short-lived, Account- and browser-bound TOTP attestation. The server enforces
the 30-second counter, replay protection, and rate limits, and does not log a
TOTP value or seed. The seed is encrypted at rest under a wrapping key.

The local Live Demo follows the same ceremony. Selecting Morgan Delgado creates
pending MFA, not a session. Its local controller provisions a genuine
operating-system-CSPRNG seed and writes only a restricted, ignored, mode-0600
operator artifact, and logs only that artifact's path. A separate local
authenticator consumes the artifact; neither the browser nor a fixed shared
secret receives the seed, and the artifact cannot serve as a browser
credential. This decision creates no recovery or self-service flow.

The vertical implementation order is primary authentication, pending-MFA
state, database attestation and one-use session transition, then browser
completion. A failed later step creates no session and the database transition
rolls back as one operation; a consumed attestation cannot be reused. The
boundary applies OWASP ASVS 2.1.1, 2.2.1--2.2.3, 2.3.1 and 2.3.3, 6.1.3,
6.3.1 and 6.3.4, 6.4.3 and 6.4.4, 6.5.1, 6.5.3, 6.5.5, and 6.5.8,
7.2.1--7.2.4, 7.4.1 and 7.4.3, and 7.5.1. Browser controls may add to,
but do not replace, this server and database boundary.

**Owner.** C15 in the active [Human Guidance implementation compliance
plan](archive/human_guidance_implementation_compliance_plan.md)
owns the implementation and verification details.

### Profile is the one time-zone preference page

**Decision.** Every signed-in User Role checks and edits the Account's exact
time-zone preference on `/profile`. The page uses the self-only `GET` /
`PUT /api/account/settings` API boundary. The update body has the closed shape
`{ "timeZone": "exact IANA name" }`; its responses also include the Account's
display-mode preference and personal Theme. Separate self-only appearance
writes change those values. None of these boundaries accepts an Account, Course,
or User Role selector. PostgreSQL
derives the active Account from the authenticated session and atomically reads
or replaces that Account's exact installed IANA name. Profile is the Account
preference page.

**Why.** Profile is the one place to check this preference. The API boundary
does not let a caller select another Account or turn a display choice into
Course authority.

**Consequence.** A successful update changes date display for the Account and,
for an Instructor, the wall-clock interpretation of dates entered later. It
never changes an already stored instant. Other pages format date and time
without repeating the zone name. Profile also owns avatar selection and
Profile-image work.

This Account preference boundary exposes no passkey, email, TOTP, recovery,
Account-status, or session control. Human Guidance's required Student and
Instructor passwordless authentication and multiple Student passkeys remain
owned by Accounts-and-roles milestones. The unresolved
credential-lifecycle choices are only self-service enumeration, revocation,
re-authentication, identity-proofed recovery, notification, and
session-termination semantics; a separate decision must define those rules
before implementation. C15's Sysadmin TOTP session decision remains the
separate authentication boundary, including its absence of self-service TOTP
management or recovery.

The owner chain is browser closed-shape decoding, server request validation,
then one PostgreSQL transaction that derives the active Account and updates its
preference. A failed validation, authorization, or write rolls the transaction
back and leaves the prior preference unchanged. The boundary applies OWASP ASVS
2.1.1, 2.2.1--2.2.3, 4.1.4, 8.1.1--8.1.2, 8.2.1--8.2.3, and 8.3.1; browser
controls aid usability but do not replace the server and database checks.

**Owner.** C819-C823 in the active [Human Guidance implementation compliance
plan](archive/human_guidance_implementation_compliance_plan.md)
own implementation and verification. C821 owns this scope decision.

### Account state preserves history

**Decision.** Deactivation blocks new access but preserves authorship, Course
relationships, Student Work, and history. Reactivation restores eligible
relationships. No permanent Account-closure workflow is currently defined.

**Why.** Authentication state must not become accidental content or record
deletion.

## Questions and Pools

### Question export uses the conversion library

**Decision.** Instructor Question export sends selected Questions to another LMS using the
external `qti-package-maker-rs` library for conversion and packaging.

**Why.** Neil defined export for reuse in another LMS and asked PLE to use the existing Rust
library rather than repeat that work.

**Consequence.** Selecting a Pool exports its members together in one package. Each member is
an individual Question in that package. This packaging interpretation follows Neil's example
of a Pool with 1,000 Questions. Supported formats and bank grouping depend on the external
library and target LMS. This decision does not add PLE backup or transfer requirements.

**Owner.** [QUESTION_EXPORT_SPEC.md](QUESTION_SPECS/QUESTION_EXPORT_SPEC.md) and
[QTI_INTERCHANGE_SPEC.md](QUESTION_SPECS/QTI_INTERCHANGE_SPEC.md).

### Draft and Published Questions are separate

**Decision.** A Draft Question is private, mutable, unpublished, and
unversioned. Publication creates or advances a stable Published Question
lineage with immutable Question Revisions.

**Why.** Private authoring and public reuse have different access, storage, and
evidence needs.

**Consequence.** Manual Draft deletion and publication remain the current
Draft-removal transitions. Human Guidance requires an appropriate warning and recovery period
before cleanup. Automated expired-Draft cleanup remains deferred; no expiration period, duration,
or cleanup schedule is set. Metadata edits that do not
change Question source do not create Revisions. A substantive fork creates a
new Question ID with attribution.

### Published Question forks create a new private Draft through one server command

**Decision.** A fork starts from one exact Published Question Revision and creates an ordinary
private Draft Question with a new Question ID, its Instructor owner, and Revision 1 on publication.
The ordinary Draft and Published Question records carry the exact immediate parent ID and Revision.
Draft creation copies the source license, authors, metadata (including nullable Bloom fields), and
content into the Draft. It offers no license choice and stores no second copy of the source as fork
history. A fork of a fork points to the immediate parent Question Revision. Parent history stays
with the parent Question.

The server resolves the selected source, reserves the fork's new public Question ID, and writes the
Draft, ownership, copied state, parent tuple, and ordinary creation receipt in one transaction. The
private Draft has its own Draft identity. Publication uses the reserved ID and creates its Revision
1. The receipt binds the active Instructor, request key, request fingerprint, and Draft. Retrying
the same request returns the same Draft; reusing the key for different request content is refused.
The receipt follows the ordinary Draft lifecycle and is removed with Draft deletion or publication.
There is no replay workflow after publication.

**Why.** Draft and Published Question rows already own editable and published Question state.
Parentage adds one relationship to those records; it does not need a fork-specific lifecycle or
parallel source-history model. Copying the source at Draft creation lets later Draft edits remain
independent and lets publication use the locked Draft values even if the parent later changes.

**Consequence.** The fork operation reads and locks the selected source Revision, then copies its
current values into ordinary Draft metadata, authorship, and source binding. The reservation
registry holds the new public ID while the Draft is private. Publication reuses that ID, copies the
locked Draft values to Revision 1, and persists the Draft's parent tuple on the new Published
Question. The normal Question Watch event is created once when that Published Question is inserted.
Ordinary Draft and Library readers expose the nullable parent tuple. The archived exact-source read
path remains available; it does not become fork-history storage.

**Owner.** [CONTRACTS.md](CONTRACTS.md)'s Question lineage and revision boundary and
[QUESTION_FORK_SPEC.md](QUESTION_SPECS/QUESTION_FORK_SPEC.md).

### Bulk metadata editing is an all-or-none current-state command

**Status: deferred.** Neil deferred Instructor bulk editing on 2026-10-05. The earlier
design below records implementation history, not a current delivery requirement or a settled
future save policy. See [QUESTION_LIBRARY_BULK_EDIT_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_BULK_EDIT_SPEC.md).

**Decision.** An active Instructor may update selected Published Questions' global Tags,
Discipline, Subject, Topic, and Subtopic together.
The command replaces only fields explicitly present in its closed patch. An empty tag list or
null Topic/Subtopic clears that optional field; Discipline and Subject remain required.
It never edits source, answer, grading,
feedback, assets, backend, Question Type, authorship, ownership, availability, or a Question
Revision.

**Why.** The earlier implementation supported cleanup of large imports and treated
metadata as separate current state. The current model places it on the complete Question record;
see [QUESTION_REVISION_SPEC.md](QUESTION_SPECS/QUESTION_REVISION_SPEC.md). A whole-batch outcome avoids
silently half-cleaned library state and avoids disclosing which selected reference was unavailable
or unauthorized.

**Consequence.** Every selected canonical ID carries its current metadata Edit Number. The server
normalizes the distinct nonempty set in canonical order, enforces one server-owned bounded
maximum, locks and validates all targets before writing any, and uses one transaction to either
commit all replacements with new per-Question Edit Numbers or change none. Results use canonical
Question-ID order. Stale selection returns a whole `412`; invalid selection or patch returns
`422`; inaccessible targets use the normal nonenumerating denial. If a response is ambiguous, the
client refreshes current metadata and Edit Numbers before deciding whether to submit another
command; this boundary promises atomic CAS, not exactly-once delivery or replay receipts. The
numeric batch maximum and future addition of another _stored shared search metadata field_ are
operational/schema decisions. A future field must join the same closed patch and CAS contract; no
arbitrary JSON field patch is permitted.

**Current implementation evidence.** First publication seeds Tags from Native JSON source or
an empty WeBWorK list, then uses separate current metadata storage. This Backend-specific source
duplication needs reconciliation with the shared Question record. Title, Description, Tags,
license, and citation have that same owner for every Backend; see
[QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_METADATA_SPEC.md).

**Owner.** [CONTRACTS.md](CONTRACTS.md)'s Bulk Published Question metadata boundary;
C365-C368 and C893 implement it in the active
[Human Guidance implementation compliance plan](archive/human_guidance_implementation_compliance_plan.md).

### Disciplines use stable retirement, not deletion

**Decision.** A Discipline keeps one stable UUID through rename, retirement,
and restoration. Sysadmins exclusively perform those transitions. PLE does not
delete Disciplines.

**Why.** Courses and Library Objects need durable classification references even
when a value should no longer be offered for new work.

**Consequence.** Retired Disciplines leave new-choice lists but remain visible
and discoverable with retired status on existing references. Exact inheritance
and copying may retain an already referenced retired value. New use requires an
active value, and shared row locks serialize that check against retirement.

### Content stewardship follows Human Guidance

**Decision.** PLE is not an online forum or social-media system. Sysadmins should
be reluctant to intervene in teaching content. Existing Question owner revisions,
forks, and Blueprint Course Change Proposals follow Human Guidance.

**Why.** References to GitHub-like stewardship do not authorize a general
discussion system or routine Sysadmin moderation of Pool content.

**Consequence.** An Instructor who finds a problem in another Instructor's Question or Pool can
fork it and fix it. Neil rejected manually written notices in favor of that existing workflow.
Remove the existing impact-notice implementation as recorded in
[TODO.md](TODO.md#question-spec-implementation-follow-up). Watch notifications cover Question
Revisions, Pool membership edits, and forks. The existing Change Proposal guidance remains
separate from notices.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-library-stewardship-specifications)
and the [interview follow-up](active_plans/decisions/HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md).

### Public Question and Pool IDs are checked human references

**Decision.** Before the first production deployment, PLE uses one direct
Question-ID cutover to `XXXX-ZXXX` at every boundary, including storage,
serialization, and browser use. Its hyphen is part of the form and makes it
immediately recognizable as a Question ID. Human entry may omit the hyphen;
canonicalization restores it before validation and lookup.
The seven identity characters are the four characters before and three after
the embedded checksum character, which is the first character after the hyphen.
The checksum is the high five bits of public unsalted `SHA-256(identity)` digest
byte zero, encoded in the Crockford alphabet. The exact canonical syntax and
rejection rules remain those in
[QUESTION_ID_SPEC.md](QUESTION_SPECS/QUESTION_ID_SPEC.md).

**Why.** A short copyable public ID benefits from typo detection without
revealing creation order or object metadata.

**Consequence.** Every Question-ID entry boundary validates the public checksum
before lookup while preserving the exact canonical value.

Internal, non-user-facing objects retain native UUID identifiers. A public
reference is created only when a human-facing workflow needs one. Existing
`R`, `W`, `D`, `M`, and `I` short-reference concepts are implementation drift,
not approved public formats, and must be removed rather than replaced.
There is no dual parser, legacy-ID reader, data rewrite, or compatibility path.
A fresh-schema preflight requires zero published Question rows before the
cutover; a nonzero count stops the work and escalates rather than converting
stored identities. [QUESTION_ID_SPEC.md](QUESTION_SPECS/QUESTION_ID_SPEC.md) owns the exact
generation and validation contract. UUIDs remain internal.

Pool creation uses this same server-held allocator: a browser never supplies a
Pool ID, and the typed server command retries only a database uniqueness
collision with a newly issued canonical ID. Pool schema owns the exact canonical
ID and current membership storage, but it does not become an
unrouted ID issuer. The create route is the only path that combines the active
Instructor authorization, allocator, and atomic Pool creation operation.

### Question Backends own Question behavior

**Decision.** A Question Backend owns rendering, interaction, response
interpretation, grading, feedback, and backend state. PLE owns authorization,
Assessment and Attempt workflow, persistence of immutable credit fractions,
score calculation, and disclosure.

**Why.** Parsing an external backend's controls inside PLE duplicates semantics
and inevitably drifts.

**Consequence.** Backend presentation and state are opaque. PLE does not infer
Question Type from controls. A backend outage never becomes an incorrect
response. See [QUESTION_BACKEND_SPEC.md](QUESTION_SPECS/QUESTION_BACKEND_SPEC.md).

### Opaque WeBWorK previews report only size

**Decision.** The shared public `ple_bridge.js` recognizes an opaque WeBWorK
preview by its effective `null` origin. That mode reports only the versioned
three-key resize record `ple.webwork.preview.resize` with a safe integer height
from 160 through 1200. It measures visible renderer form and top-level content,
then adds the body's computed lower padding and border without reading the
iframe-sized body rectangle. It runs on load and through `ResizeObserver`, and
suppresses repeated heights.

**Why.** Instructor inspection needs content-sized previews without turning an
opaque no-write renderer document into a response or control protocol.

**Consequence.** The parent accepts a resize only when `event.origin` is `null`,
`event.source` is the exact iframe `contentWindow`, and every message key and
value is exact. The opaque document targets its canonical parent origin from
its URL. Preview mode emits no ready, capture, form, or response message.
Student document bytes and their same-origin bridge behavior remain unchanged.

**Owner.** [opaque_webwork_preview_frame.tsx](../src/components/opaque_webwork_preview_frame.tsx),
[ple_bridge.js](../src/public/ple_bridge.js), and the Backend preview document
row in [API_CONTRACTS.md](API_CONTRACTS.md).

### WeBWorK correct answers stay backend-owned

**Decision.** Correct-answer review is one private renderer operation over the
exact retained Question Revision and Attempt seed. It accepts no Student
response and returns transient backend-owned HTML through completed-history
authorization. The same Instructor-selected correct-answer condition runs before and after
rendering. Waiting for all Students to complete Quizzes and Exams is a default
that Instructors can change.

**Why.** PLE owns disclosure and WeBWorK owns answer presentation. Correct answers
must not pull independently controlled responses, explanations, Hints, Worked
Solutions, or new scores into the disclosure.

**Consequence.** History exposes only the literal availability marker. Each frame
fetch independently authorizes the fixed route and uses the existing opaque
script-only preview sandbox. Generated review assets stay inline in that
protected document. No public review asset, answer parser, persisted review,
grading write, or disclosure latch is introduced. Failure is local to review;
the issued document and immutable Student Work remain intact.

**Owner.** [QUESTION_BACKEND_SPEC.md](QUESTION_SPECS/QUESTION_BACKEND_SPEC.md),
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md), and
[webwork_document_route.rs](../crates/server/src/webwork_document_route.rs).

### Native PLE Question JSON stays deliberately small

**Decision.** The Native PLE Question JSON format is an internal, unpublished
specification without format versions. Native Questions still follow ordinary
Draft, publication, and immutable Question Revision rules. The format is strictly
validated and static. It supports the eight named native Question
types in Human Guidance. Author JavaScript is isolated and untrusted; grading
is server-side.

**Why.** A closed source shape is easier to validate and teach than a public
extension ecosystem or compatibility framework.

**Consequence.** Native-format changes do not add version negotiation. New
behavior requires an explicit product decision and coordinated strict-shape
change.

### Author JavaScript is an answer-free isolated document

**Decision.** An `AuthorContentPresentation` contains only validated
author-script source and closed reviewed library IDs. It excludes Answer Key,
feedback correctness, grading input/output, seed, generated-parameter hash,
response bindings, session/capability, Account/Course/Attempt metadata, and
arbitrary URLs. It is not an ordinary `StudentQuestionPresentation` JSON
payload. A dedicated authenticated `no-store` HTML document route reproduces
the pinned presentation only after exact Student and position authorization;
there is no raw-source browser API, save, submit, grading, object-store, or
general API route.

The route safely encodes source rather than concatenating it, and sends exactly
`Content-Security-Policy: sandbox allow-scripts; default-src 'none'; base-uri
'none'; object-src 'none'; connect-src` restricted to the exact C901
server-selected RDKit WASM path; `img-src 'none'; media-src
'none'; font-src 'none'; frame-src 'none'; worker-src 'none'; form-action
'none'; frame-ancestors 'self'; script-src` restricted to C901 server-selected
reviewed local assets plus a server bootstrap nonce; `Content-Type: text/html;
charset=utf-8`; `X-Content-Type-Options: nosniff`; `Cache-Control: no-store`;
and `Referrer-Policy: no-referrer`. No author-declared URL is admitted.

The browser receives only a typed optional frame reference/availability and
embeds it as `sandbox="allow-scripts" referrerpolicy="no-referrer" allow=""`.
It never grants same-origin, forms, popups, downloads, modals, top navigation,
pointer lock, storage access, or permissions. The script receives its own DOM
and reviewed libraries. Its optional outbound message is versioned
`author-content.resize`, with finite integer dimensions clamped to the declared
bounds and accepted only when `event.source === iframe.contentWindow`;
opaque-origin `null` therefore still has a source-identity check. The parent
also sends the current resolved appearance as one cosmetic
`ple.embed.appearance` version 1 record. It contains only `mode` and ten fixed
`#rrggbb` roles: background, foreground, surface, secondary, accent, highlight,
muted, border, onAccent, and link. `accent` and `onAccent` remain the control
pair; the contrast-qualified `link` role supplies ordinary links and focus
indicators. For an opaque sandbox, the parent delivers this
closed cosmetic record to the exact frame with `*`; same-origin backend frames
receive it at their canonical origin. A receiving bridge accepts only that
canonical parent origin, the exact parent window, and the exact closed record
keys and values. The protocol
has no response, identifier, Course, Account, Question, Attempt, response,
navigation, storage, or API command.

**Why.** Executable author content is inspectable but never secret; it remains
safe only when it is answer-free and grading-independent. The boundary applies
ASVS 1.1.2, 1.2.1, 1.2.3, 2.2.1, 2.2.2, 3.2.1, 3.2.2, 3.4.3-3.4.6, 3.5.5, and
3.6.1 without making the ordinary PLE application context available to it.

**Consequence.** C303 records this architectural handoff but closes no Human
Guidance occurrence. C857-C860 implement the descriptor, document route,
frame, and connected proof; C859 owns the five isolation occurrences and C304
waits for C860. HOTSPOT remains PLE-owned and grading remains server-owned.

### RDKit is a reviewed local runtime asset, not author-selected content

**Decision.** The closed `rdkit` library identifier is the only authority an
author may select. The server resolves the current official `@rdkit/rdkit`
release through a reviewed local registry, with its BSD-3-Clause license and
the documented `RDKit_minimal.js`/`.wasm` `locateFile` pair. Human Guidance's
latest-dependency rule governs this resolution; neither an author nor a
Student Work record chooses or preserves a package version.

The tracked manifest records the current reviewed official provenance, npm
integrity, license checksum, and SHA-256 checksums for exactly the reviewed JS
and WASM files. The deterministic vendoring/check command resolves the current
official release, verifies provenance, license, package integrity, paths, and
bytes, rejects extra runtime files, and writes the generated server registry.
Production never resolves npm, unpkg, a CDN, or an author URL.

The server exposes only the current generated JS/WASM pair through the
unversioned derived `GET`/`HEAD` routes
`/api/author-content-dependencies/rdkit/RDKit_minimal.{js,wasm}`. Responses
use `application/javascript; charset=utf-8` or `application/wasm`,
`X-Content-Type-Options: nosniff`, `Cross-Origin-Resource-Policy: cross-origin`,
and `Cache-Control: no-cache` so a browser revalidates current bytes rather
than treating an unversioned URL as immutable. The two fixed public files also
send `Access-Control-Allow-Origin: *`, with no credential grant, because RDKit
WASM fetches from the opaque sandbox origin. They send no cookie-dependent
response, redirect, directory listing, caller-selected path, or object-store
URL. `cross-origin` is intentional because an `allow-scripts` opaque-origin
frame must load this public pair, while no private content is on the route.

The author document emits the current registry JS SRI hash and exact server
`locateFile` WASM path. Its CSP has a nonce for server bootstrap/encoded author
source, that JS hash, the single exact WASM `connect-src` path, and
`'unsafe-eval'` only because the current official RDKit Emscripten loader
requires it inside this opaque `sandbox="allow-scripts"` document. It does not
use `'self'` as a broad script or connect grant, and the main PLE CSP never
receives that allowance. This narrow WASM exception replaces `connect-src
'none'`; it does not authorize an application API or an author-chosen endpoint.
For the closed `libraries: []` branch, the document instead has no runtime
tags, `connect-src 'none'`, nonce-only `script-src`, and no `'unsafe-eval'`.

**Why.** One reviewed local dependency keeps executable chemistry support inspectable,
reproducible, and isolated from author-controlled or network-resolved runtime code.

**Update workflow.** A review refreshes the current official release, license,
integrity, and two reviewed file hashes from a clean download, checks browser
compatibility and the isolated-frame behavior, then regenerates the local
assets and registry. It directly replaces the pre-production current runtime;
there is no historical dependency catalog, retirement workflow, per-Attempt
version/digest binding, or CDN fallback. An unavailable, malformed, or
mismatched artifact fails closed and leaves author content unavailable.

**Consequence.** The former author-declared `cdnUrl`/`localPath` syntax has
been removed. It was validation evidence, not an approved dependency inventory
or local delivery authority. C900-C902 build, serve, and maintain the reviewed
chain. C858's document route waits for C901's current-runtime handoff, and C903 is the only owner of the three
external-dependency Human Guidance occurrences.

### H5P and iMathAS are deferred Backends

**Decision.** Current production Question Backends are PLE and WeBWorK. iMathAS
and H5P remain desired secondary Backends in Human Guidance's Deferred product
behavior section and are not current implementation requirements. Future H5P
use is limited to Weekly Assignments, Bonus Assignments, and Unit Review
Assignments; Quizzes and Exams do not use H5P because its runtime exposes
answers and correctness to the Student browser.

**Why.** The Deferred section lets current implementation and compliance work
finish without treating desired later Backends as present production support.

**Consequence.** No iMathAS or H5P delivery work dispatches while it remains
deferred. C870's removal of dormant H5P placeholders remains complete. Detailed
future H5P runtime design is outside the current goal and must be decided when
the Backend moves out of Deferred product behavior.

### Native PLE JSON attempt reproduction is seed-free

**Decision.** Question-attempt reproduction is explicitly tagged `Static` or
`Seeded`. Native `pleQuestionJson` is `Static`: an attempt retains its exact
Question Revision, source binding, presentation descriptor, and response-item
bindings, but has neither a `QuestionSeed` nor a generated-parameter hash.
The PLE server may mint a presentation nonce to bind response-item references
and an authored choice-order policy. That nonce is PLE presentation randomness,
not a source, author-JavaScript, or Question-generation seed.

**Why.** A static native source needs no invented variation evidence. The
explicit tag preserves exact explanation for backends that do generate a
variant without giving the generic Attempt model a misleading seed-shaped
placeholder.

**Consequence.** Descriptor evidence checksum v2 binds the reproduction tag
and its applicable facts. A `Static` native descriptor binds the exact source
and presentation facts, including the presentation nonce where it determines
response-item or choice order, and rejects a seed or generated-parameter hash.
A `Seeded` descriptor binds its backend-owned seed and generated-parameter
hash when that backend uses them. WeBWorK is currently `Seeded`. iMathAS and
H5P are deferred and have no current production reproduction requirement; H5P
also has no delivered binding or retained binding seam after C870. Any Backend
moved out of Deferred product behavior must explicitly select `Static` or
`Seeded`; it must not infer or default a seed.

This is a preproduction direct cutover: model, base-schema, Store/server
issuance, descriptor validation, and Student Work readers change together, and
a fresh database is reinitialized. There is no sentinel seed, null-means-two-
things field, compatibility reader, or compatibility shim. The issuance chain
first constructs the tagged descriptor, then validates and persists the
Question Attempt, binding, and response-item evidence in one transaction. A
later validation or persistence failure rolls back the whole issuance and
leaves no partial Student Work evidence.
If this coordinated cutover fails before the first production baseline, rollback
restores the previous pre-production code and base schema together and
reinitializes the disposable database; no mixed code/schema deployment is
valid.

The reviewer may promote one public no-seed contract test only after every
[PYTEST_STYLE.md](PYTEST_STYLE.md) permanent-test checklist item is yes. It
protects the deliberate native `Static` boundary from a plausible regression
while avoiding storage-shape assertions. Otherwise the test remains temporary
and is removed. Its failure means native issuance again exposes or retains a
seed/hash, so the tagged descriptor and issuance boundary must be corrected
before the native no-seed Human Guidance item can close. The vertical database
matrix always remains temporary: it demonstrates both tags and rejection of a
native seed/hash, then is removed.

**Owner.** C300 owns the native-delivery correction milestones in the active
[Human Guidance implementation compliance plan](archive/human_guidance_implementation_compliance_plan.md)
and their verification. A future, product-approved H5P implementation owns a
new explicit binding design; C306 is not a dormant delivery seam. Human
Guidance remains the product authority.

## Blueprint Courses

Blueprint Courses follow a model very similar to GitHub repositories. Revisions,
forks, Change Proposals, Stars, and Watches provide familiar concepts; HG defines
their PLE behavior and the differences for adoption into teaching Course Instances.
The analogy does not authorize every GitHub feature.

### Blueprints use Private, Public, and Archived lifecycle states

**Decision.** Creation and forks start Private. Private is owner-only and
cannot be adopted. Public is visible to Instructors and adoptable.
Archived is read-only, excluded from ordinary discovery and new adoption,
visible only through explicit archived inclusion, and forkable.

**Why.** Visibility, reuse, and retirement need clear author-controlled states.

**Consequence.** Only the owner changes lifecycle state. Public may return to
Private only before any adoption. Once adopted, it remains Public unless
Archived. Archived restores to Public.

### Blueprint Saves create content Revisions only when content changes

**Decision.** A Blueprint is created Private with Revision 1. Explicit Save
creates the next Revision only after a meaningful reusable-content
change. A no-op save creates nothing. Name and lifecycle metadata changes do
not create Revisions.

**Why.** Each Revision should identify an actual reusable course-content state.

**Consequence.** Relative schedules, Course dates, Students, and time zones do
not belong in a Blueprint. Course Instance creation supplies real dates and
local settings.

### Adoption, updates, forks, and Change Proposals preserve provenance

**Decision.** A Course Instance may adopt a Public Blueprint or start empty.
Adoption records the exact Blueprint Revision. New Revisions are offered to
daughter Courses for review and approval. A fork starts a new Private Blueprint
lineage with ancestry and may selectively bring in later source changes.
Instructors may propose changes to another Blueprint through a Blueprint Course
Change Proposal; accepted changes create a new receiving Blueprint Revision.

**Why.** Instructors need both reproducible adoption and independent control.

**Consequence.** Newly added Blueprint Assessments are automatically copied to
daughter Course Instances as Unreleased Assessments. Changes to existing
Assessments require the daughter Course Instructor's review and approval.
Change Proposals never change daughters directly; accepted changes reach them
through the normal Blueprint update workflow.

### Blueprint fork updates are explicit selective saves

**Decision.** A fork's existing immutable origin records its source Blueprint Course Revision Tuple
as provenance, not a required comparison baseline. Human Guidance now
requires any visible related Blueprint Courses in the same fork lineage to be comparable, normally
using the newest source and fork Revisions. Shared Question IDs provide durable content
relationships; Blueprint Assessments are matched by the shared Question IDs they contain, not
internal Assessment identities. Comparison shows shared, added and removed Assessments and Question
IDs and determinable canonical-JSON content changes, remaining useful through renames, reordering
and structural changes. Assessment IDs are local to a Blueprint Course, forks receive
fresh Assessment IDs, and no persistent cross-Blueprint Assessment lineage/history is introduced.
Relationships are inferred only from shared Question IDs; disjoint Question sets remain unmatched.
Prior fork/apply evidence that relies on shared internal Assessment IDs must be reconciled, not
treated as corrected-contract closure. Comparison includes
the complete reusable content tree: module labels, module and Assessment structure and order, and
each Blueprint Assessment's settings, Questions, and Pools. Current short and long names participate
as metadata. No separate per-unit JSON baseline or public comparison-state vocabulary is persisted.

Viewing and comparison follow ordinary Blueprint visibility for both fork and source: every
Instructor may view Public and Archived Blueprints; explicit Archived inclusion governs discovery,
not permission to view a known Course. Private Blueprints are owner-only on either side. Ownership
controls the fork's apply mutation. The Instructor explicitly selects which displayed changes to
bring forward; no source change is applied automatically. One request may select several related
changes. The server constructs and validates one coherent complete fork tree, then uses the ordinary
expected current Blueprint Course Revision Tuple compare-and-swap to save all selected content changes as
one new immutable Blueprint Revision. Selected name changes use the ordinary Blueprint metadata
Edit Number in the same authorized operation (HTTP `If-Match` encodes it).

**User correction (2026-09-16).** This clarifies the existing Human Guidance visibility boundary;
it is not a new product decision. It removes the incorrect owner-only comparison restriction while
leaving ordinary Private, Public, and Archived states unchanged.

**Why.** A fork must remain independently controlled while newer source work is easy to discover,
review, and selectively bring forward without a hidden overwrite or source-information leak.

**Consequence.** Prior C880-C883 evidence remains valid contributor evidence for the internal-ID
direct-source projection, ordinary visibility, current-head read, on-request calculation and
selective-save backend. It does not close the newly authoritative same-lineage pair coverage or
Question-ID-based Assessment matching; those corrections and connected proof remain open in the
Human Guidance compliance plan. The old three-snapshot origin-based projection is implementation
history, not the required comparison model. These boundaries reuse existing provenance and
immutable Blueprint Revisions rather than adding sync-baseline persistence, a merge framework,
candidate digests, replay receipts, or one-unit-per-request restrictions. C413's read-only
source-entry/known-forks/current-comparison slice has accepted evidence; direct fork discovery and
apply UI remain open, and complete C413 closure still waits C884. Question-level hunk selection
remains optional rather than a blocker.

### Canonical Blueprint JSON is the comparison and exchange form

**Decision.** Canonical Blueprint JSON contains Blueprint metadata plus ordered
Blueprint Assessments, their reusable settings, Published Questions, and
published Question Pools. It is complete enough for comparison, import,
export, exchange, and recreation, but is not the primary persistence model.

**Why.** Blueprint comparison and exchange need one exact portable form without
turning that interchange representation into the storage architecture.

**Consequence.** Blueprint JSON contains no deadlines, release dates, Student
data, or Course Instance delivery settings. Change Proposals compare Blueprint
Revisions through this canonical representation.

### Blueprint Stars and Watches belong to the lineage

**Decision.** Instructors may Star or Watch Public and Archived
Blueprint Courses. Stars follow the GitHub model for saving useful content,
showing a count, and seeing who Starred content the viewer can access. Watches follow the GitHub repository model and drive notifications about
Revisions and other important changes. Search can compare aggregate Watch counts. Forking or
adopting does not automatically Star or Watch.

**Why.** Star and notification choices apply to a Blueprint lineage;
adoption and forking are separate Course-creation decisions.

**Consequence.** Stars and Watches follow the Blueprint lineage across all of
its Revisions. They are not copied into a fork or daughter Course. C409 owns
Star/unstar/count and private self Watch/unwatch state plus Revision, publish,
archive, and restore Watch fan-out. Instructors can see who starred a Blueprint;
this does not restrict the visibility of Instructor Profiles elsewhere in PLE.
Personal subscription controls remain private; aggregate Watch counts support
Blueprint search. Search results do not expose watcher identities.

## Assessments and Student Work

### Assessments have a small release lifecycle

**Decision.** A Course Instance Assessment is Unreleased or Released. Release
is explicit. Date-based availability does not create Closed or Archived
Assessment states.

**Why.** Extra stored states duplicate values already determined by release and
time.

**Consequence.** Unrelease is the high-consequence reversal. It requires the
typed Assessment title and deletes all Student Work for that Assessment while
preserving the Assessment and shared content.

### The whole Assessment Attempt is the submission boundary

**Decision.** Complete Question responses are saved and remain editable while
the Attempt is open. Incomplete responses are not saved as complete or graded.
The Student submits the whole Attempt; the deadline can submit it
automatically. That transition finalizes all saved responses together.

**Why.** Students need reliable navigation and saved work without accidentally
finalizing one Question at a time.

**Consequence.** The product has no separate response-finalization action,
public per-response finalization state, or Question-level grading workflow.
Repeating a whole-Attempt submission converges on the same result.

### Backend credit is immutable; points remain current

**Decision.** The Question Backend returns an immutable credit fraction. PLE
stores it unchanged and calculates the score using the Assessment Question's
current point value.

**Why.** Point corrections should update totals without pretending the Student
gave a different response or requiring regrading.

**Consequence.** PLE has no ordinary regrading, grading Retry, mutable result,
or scoring-freshness lifecycle. A current point-value edit recalculates scores
from stored fractions. Removed Assessment entries contribute neither earned
nor possible points to any Attempt. Native JSON regrading remains deferred
under HG; the current immutable-result implementation does not override that
future decision.

### Highest Attempt and point-only scoring

**Decision.** Instructors control Attempt limits and Assessment behavior.
Weekly Assignment defaults support repeated work toward success. When several
Attempts are submitted, the highest Assessment Attempt score is the Student's
Assessment score. An unanswered Question remains visibly unanswered, receives
zero credit, and counts as incorrect without backend evaluation.

**Why.** Highest-score selection supports learning through repeated practice.
Zero for unanswered work keeps the point calculation direct and predictable.

**Consequence.** PLE uses Question points and does not add Grade Categories,
weighted categories, Course Grade Schemes, or Course percentage calculations.
Pilot grade export is CSV or TSV point data for Course-level handling in the
Instructor's home LMS; it is not LMS synchronization.

### Evidence is minimal and purpose-bound

**Decision.** Retain exact Question Revision, Pool ID, and Pool Edit Number selection, backend state
needed to interpret the response, the response, immutable credit fraction, and
disclosure state.

**Why.** Student Work must remain explainable without creating an unnecessary
historical surveillance or replay system.

**Consequence.** Human Guidance does not require rendered-page snapshots,
software-version snapshots, generalized receipts, compatibility layers, or
public background-grading machinery.

## Interface

### Account endcap keeps one visual size

**Decision.** The right-side Account avatar has the same outer box, corner shape,
icon size, and alignment across responsive Ribbon layouts. Touch navigation
spacing does not enlarge this control. Public headers show the same visual box
with a neutral, labeled Not signed in icon when there is no authenticated Account.

**Why.** The Account location stays recognizable as navigation rearranges, and
the public header does not imply that a signed-out visitor has a Profile menu.

**Consequence.** The shared Ribbon avatar style owns the visual dimensions.
The public shell uses a noninteractive signed-out indicator and shows the
Account's avatar as a Profile link when a session is authenticated.

**Owner.** `src/ribbon/app_ribbon.css`, `src/ribbon/app_ribbon_density.css`, and
`src/application_shell.tsx`.

### One stable Ribbon frame serves role-specific work

**Decision.** The shell keeps stable page geometry, separates global context
from page tasks, and preserves every required destination even when its
collection is empty. Every required Instructor destination also remains visible
when its target is incomplete, but it is presented as unavailable rather than
as a usable link. The top Ribbon carries PLE identity, User Role, stable
navigation, and application controls; the compact signed-in phone treatment is
defined below. Human-readable Course and Assessment
hierarchy belongs in breadcrumbs, not in route-specific Ribbon labels. Sign Out
is in the Profile menu.

**Why.** Stable geometry, honest empty states, and visible unavailable tasks
reduce cognitive load without letting incomplete features masquerade as usable.

**Consequence.** Instructor primary tabs are Courses, Questions, and
Assessments. Human Guidance defines each fixed Instructor task row. A settled
role and Tier 1 area keep the same ordered destinations across deeper routes;
object-specific Course and Assessment links stay in page content and
breadcrumbs. Student work is collectively Coursework, while a specific item
uses its Assessment Type name.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
[app_ribbon.tsx](../src/ribbon/app_ribbon.tsx), and
[app_ribbon_density.css](../src/ribbon/app_ribbon_density.css).

### Tier 2 uses the space for destinations

**Decision.** Do not repeat the selected Tier 1 label as a visible caption in
the Tier 2 row. Keep the Tier 2 navigation landmark and its individual,
descriptive destination names.

**Why.** The selected Tier 1 already identifies the row's subject. The caption
was plain text inside the named navigation landmark, not a semantic group label
or relationship. The links keep their own accessible names; the caption only
repeats information in the reading order and uses space needed by destinations.

**Consequence.** Student and Instructor Tier 2 rows use the full row for their
destinations. Area wrappers remain available for layout grouping; no visible
section caption is rendered.

**Owner.** [app_ribbon.tsx](../src/ribbon/app_ribbon.tsx) and
[app_ribbon.css](../src/ribbon/app_ribbon.css).

### Curved Ribbon tabs sit above square content tabs

**Decision.** Tier 1 is one continuous colored bar with integrated resting
choices. Only its selected tab has a silhouette: rounded top corners and small
outward-curving feet flowing into a narrow Ribbon surface beneath the bar.
Tier 2 uses square corners and joins the page content directly. Both tiers
reserve stable control boxes and row heights when selection changes.

**Why.** The selected Tier 1 emerges from the bar as an open file-folder tab.
A raised shoulder on an enclosed rectangle depicted a whole miniature folder,
which misread the human's reference. Both selected tabs use the theme canvas;
the resting bar mixes 24% theme ink into that canvas to separate its value
consistently in light and dark modes. The lower Ribbon strip shares its
selected tab's surface. Borderless, softly antialiased shoulders carry the
silhouette; a traced accent outline made the earlier join look assembled.
Tier 2 uses the same bar color and a square, borderless opening into content.
These surface roles separate selection in either mode without a brightness rule.

**Consequence.** A shared surface stylesheet owns the adjustable value step;
separate tier stylesheets own the two silhouettes and surface connections.
The all-theme check protects a 1.5:1 active/inactive surface regression floor,
separately from the existing 5.5:1 text gate. This surface floor is not a WCAG
text requirement or aesthetic acceptance: fresh renders establish visual quality.
The curved face remains 32px tall inside a 44px touch target. Decorative
pseudo-elements keep the label upright and the click/focus box stable. The
shared outer keyboard focus ring stays clear of the tab decoration; forced
colors retains an outline and underline. Connected faces repaint together
when themes change; only label color transitions. A narrow outer rail defines
the content sheet, with no top rule crossing its selected tab. Phone rail and
inner gutter together preserve 16px for page content. Navigation stays shared.
Overflow chevrons are native, directionally named scroll buttons rather than
decorative marks. They advance their row with overlapping context, preserve the
selected destination, and leave the keyboard order when that direction is
unavailable. Their opaque row surface masks clipped text behind the control.

**Owner.** [ribbon_tier_one.css](../src/ribbon/ribbon_tier_one.css),
[ribbon_surfaces.css](../src/ribbon/ribbon_surfaces.css),
[ribbon_tier_two.css](../src/ribbon/ribbon_tier_two.css),
[ribbon_content_surface.css](../src/ribbon/ribbon_content_surface.css),
[app_ribbon.css](../src/ribbon/app_ribbon.css), and
[app_ribbon_density.css](../src/ribbon/app_ribbon_density.css).

### Breadcrumbs follow the Ribbon hierarchy

**Decision.** Breadcrumbs follow the selected Ribbon hierarchy: Home, the
selected Tier 1 tab, the selected Tier 2 control, then the page's Course,
Assessment, and title context. A route may declare `tierTwoParent`. Course
roster, assessments, and appearance take My Active Courses or My Inactive
Courses from the Course lifecycle on the course summary that route already
loads. A Blueprint Course detail uses its breadcrumb parent. Adjacent crumbs
collapse only when they show the same name.

**Why.** The trail should show where the page sits in navigation. Two different
names can share a URL, and one name should not appear twice in a row.

**Consequence.** Exact Tier 2 selection, including Active Attempt and Latest
Feedback, wins over an ancestor. Until lifecycle or the Blueprint parent is
known, the trail omits that Tier 2 crumb. Account pages omit Tier 1 because
they have no tab.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
[ribbon_contract.ts](../src/ribbon/ribbon_contract.ts),
[ribbon_breadcrumbs.ts](../src/ribbon/ribbon_breadcrumbs.ts), and
[route_contract.ts](../src/route_contract.ts).

### Signed-in phone Ribbon keeps Tier 1 beside the P mark

**Decision.** At phone width, keep the P mark, the signed-in role's Tier 1
navigation, and the fixed Profile box together on the top row. Hide the full
Peptidyle wordmark and User Role badge there; keep Tier 2 in its normal row
beneath.

**Why.** The phone view should read as a compact version of the same Ribbon.
Moving Tier 1 to a second top-bar row and pushing the User Role badge to the
far edge breaks the identity-to-navigation order.

**Consequence.** The phone top row stays one row tall, reserves space for the
fixed Profile box, and tightens Tier 1 spacing to fit the role's navigation.

**Owner.** [app_ribbon.css](../src/ribbon/app_ribbon.css) and
[app_ribbon_density.css](../src/ribbon/app_ribbon_density.css).

### Non-phone signed-in identity uses one layout

**Decision.** Across non-phone Student, Instructor, and Sysadmin views, keep
the same identity sequence and geometry: P mark, Peptidyle wordmark, and a
reserved-width User Role badge. Public pages keep their separate signed-out
header.

**Why.** The left edge of the Ribbon should stay in the same place across
signed-in roles and non-phone viewport sizes. Role text can change inside its
badge without moving the brand or navigation.

**Consequence.** User Role and pointer type change content or interaction
targets, not the non-phone identity layout. Phone uses the one signed-in
compact exception defined above.

**Owner.** [app_ribbon.tsx](../src/ribbon/app_ribbon.tsx),
[app_ribbon.css](../src/ribbon/app_ribbon.css), and
[app_ribbon_density.css](../src/ribbon/app_ribbon_density.css).

### Tier-one navigation is role-only

**Decision.** Tier-one Ribbon control IDs are selected by User Role, not by
route scope. Instructor tier one is Courses, Questions, and Assessments.
Student tier one is Coursework, Grades, and Courses. Coursework and Grades
show records across enrolled Courses; Courses provides direct access to each
Course. Sysadmin tier one follows its User Role catalog. Settled tier-two
rows derive from User Role and Tier 1; object-local navigation stays with
page content and breadcrumbs.

**Why.** A role has one stable primary navigation model. Moving primary tabs
between routes makes the shell and a user's available destinations appear to
change unexpectedly.

**Consequence.** The route contract selects a tier-one area but cannot add or
remove a tier-one control or change a settled task row. Existing route-scoped
Student task rows are replaced by the fixed rows in Human Guidance. The client
presents navigation only; the trusted server continues to enforce function-
and resource-level authorization. This keeps client navigation from becoming
an authorization boundary (ASVS 8.2.1, 8.2.2, and 8.3.1).

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md)'s role navigation rules and
the role catalog in [ribbon_schema.ts](../src/ribbon/ribbon_schema.ts), with
route resolution in [ribbon_catalog.ts](../src/ribbon/ribbon_catalog.ts).

**Implementation boundary.** Declare ordered Tier 2 membership beside Tier 1
in `PRODUCT_TIER_TWO`. The Ribbon model expands the Student Course-list slot
after reading that schema, while ordinary destinations resolve through the
catalog. The small `student_ribbon_navigation` helper loads the Student Course
list in the Courses API order, selects the most recently active Attempt with a
running clock, and resolves Latest Feedback.
The Application Shell invokes it for Student Tier 2 data but does not own
Student-specific query or sort rules. This is presentation data and never
grants access.

### Attempt History groups by Course

**Decision.** Grades -> Attempt History shows one section per enrolled Course.
Each Course section lists that Course's Attempts newest first and has its own
cursor navigation. The interface does not promise one globally chronological
list across Courses.

**Why.** Students normally have one Course and occasionally two. Course
sections identify where each Attempt belongs and reuse the existing
Course-authorized history and cursor behavior without another aggregation
layer.

**Owner.** The Student Attempt History page and the Course-scoped Attempt
History API in [API_CONTRACTS.md](API_CONTRACTS.md).

**Consequence.** Attempts are newest-first within each Course section and
older records remain available through that Course's cursor navigation.

### Student Due Soon uses the existing seven-day window

**Decision.** Student Due Soon uses the same rolling next-seven-day window as
Instructor Assessments Due Soon, evaluated against the server-provided instant.

**Why.** The existing window gives Students and Instructors one consistent
meaning for Due Soon; it is a product convention, not a pedagogical threshold.

**Consequence.** The window includes deadlines at the evaluation instant and
excludes deadlines at or beyond seven days from that instant.

**Owner.** [student_coursework_presentation.ts](../src/pages/student_coursework_presentation.ts)
and the Instructor Assessments Due Soon API.

### Student Tier 2 groups follow Tier 1 purposes

**Decision.** Order Student Tier 1 as Coursework, Grades, and Courses.
Coursework and Grades show the Student's records across all enrolled Courses,
and identify the Course when more than one Course contributes records.
Coursework Tier 2 is All Coursework, Due Soon, Completed, and Active Attempt.
Grades Tier 2 is Scores, Response Stats, Attempt History, and Latest Feedback.
Courses Tier 2 lists the short names of currently enrolled Courses in stable
order shared with the Courses list; selecting a name opens that Course and
selects its Courses Tier 2 link while Course-specific content is open. No
persistent Course selection filters or changes global Coursework or Grades.
Active Attempt is a resume shortcut within Coursework; the Tier 1 Coursework
link remains the stable entry point, and breadcrumbs identify the specific page.

**Why.** Coursework and Grades describe work and results across a Student's
enrollment. Courses provides direct access to each Course without requiring a
Course selection before opening Coursework or Grades.

**Active Attempt evidence.** A Course can have more than one resumable Attempt
because the start gate locks and resumes within one Assessment, while the schema
has no Course-wide active-Attempt constraint. The existing
`latest_activity_at` is the maximum Attempt `started_at`, `submitted_at`, or
saved-response `saved_at` within an Assessment. It does not change for viewing a
Question or for display-duration checkpoints. The landing projection's
`can_resume_assessment_attempt` applies the current Assessment availability,
unsubmitted state, and unexpired deadline. Active Attempt is enabled only for
an unsubmitted Attempt whose server-owned expiry is in the future, so its
clock is running. The shortcut chooses the eligible Attempt with the newest
`latest_activity_at`; across Course Instances, a tie uses Attempt start time
and then Attempt ID. No chooser is added for the uncommon case of multiple
live timed Attempts.

**Consequence.** Course-specific operations pass an explicit Course ID and the
server checks the signed-in Student's active Course membership. Global
Coursework and Grades pages may compose those Course-scoped reads when that is
the simplest contract. A cross-Course read is used when the product action is
itself cross-Course, as with Active Attempt and Latest Feedback. Keep Active
Attempt in its fixed Coursework Tier 2 position, disabled when no Attempt
clock is running across the Student's Courses; otherwise send the Student to
the most recently active timed Attempt. Keep Latest Feedback visible in Grades
and open the latest Attempt review with Student-visible feedback across all
Courses; the Attempt review remains in Grades. Disable Latest Feedback when none
is available. Coursework and Grades page records identify their Course when
multiple Courses contribute entries. Existing Attempt review rules continue to
enforce score and correctness disclosure.

**Owner.** [RIBBON_TASK_MODEL.md](ux/RIBBON_TASK_MODEL.md),
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), [API_CONTRACTS.md](API_CONTRACTS.md),
the Student route and catalog contracts,
and the Attempt review disclosure contract.

### Student Progress distinguishes completion from score

**Decision.** Progress distinguishes no Attempts, in-progress Attempts, and
submitted and graded Attempts with scores below or at 100%. Question scores
and the Assessment total are visible as soon as submission and automatic grading
complete. A submitted Attempt is completion; it does not imply a perfect score.
PLE calculates no weighted Course grade.

**Why.** Progress must distinguish unfinished work from completed work without
withholding automatically graded scores or confusing completion with perfection.

**Consequence.** There is no score-release control, separate posting step, or
"Score not released" state. Completed Coursework means at least one submitted
Attempt. Correct-answer visibility remains separate from score visibility.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
[API_CONTRACTS.md](API_CONTRACTS.md), and the Student Progress API contract.

### Response Stats reports actual outcomes across Assessment types

**Decision.** Grades shows Response Stats for every enrolled Course in a
separately identified Course section. Within a Course, aggregate the signed-in
Student's saved outcomes from eligible submitted Assessments across Assessment
types, grouping by the exact immutable Published Question Revision. Do not
merge counts between Courses. Include outcomes from submitted, graded Attempts;
there is no separate score-release condition.

**Why.** Response Stats should describe real recorded Question outcomes, not
estimated or synthetic results. The all-Course page retains Course context,
while each Course section includes relevant outcomes across Assessment types.

**Consequence.** The existing Course-scoped API reports actual outcome counts
and, when recorded, measured approximate time shown with the Question. It
exposes the Student's recorded outcomes without disclosing correct-answer content
before the selected answer-visibility condition permits it. It exposes no cohort data.

**Owner.** [API_CONTRACTS.md](API_CONTRACTS.md), the Student Response Stats
reader, and its Student page.

### Show the time-zone name only on Profile

**Decision.** Format times with the selected display time zone, but show its
name only on the Profile page. Other pages show the formatted date or time
without repeating a time-zone label.

**Why.** A Student or Instructor reading their own schedule does not need a
time-zone name repeated beside each time.

**Consequence.** Profile remains the place to check the exact IANA preference.
Course and Activity pages keep using the selected display zone for formatting
without adding time-zone text to their task content.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), this decision, and the
date-formatting components.

### Question duration means time shown

**Decision.** Response Stats may show average measured, approximate time shown
with a Question and its sample count. The measurement runs only while that
Question is current and the browser document is visible. Missing duration is
not recorded or inferred from Assessment elapsed time.

**Why.** The browser can measure display duration, but it cannot establish
attention, effort, or difficulty.

**Consequence.** Duration never affects grading. The API stores nullable
cumulative milliseconds as Student Work, freezes updates at Attempt
finalization, and aggregates only disclosed Student data.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
[DATA_CONTRACTS.md](DATA_CONTRACTS.md), and the Question Attempt duration API.

### Signed-in shell rows have unconditional height

**Decision.** Every signed-in role reserves the breadcrumb and tier-two Ribbon
rows at each screen size. Settled tier-two destinations and order remain fixed
for a User Role and Tier 1 area; row height does not depend on route state.

**Why.** Fixed destinations keep navigation predictable. Stable row height
keeps page content from jumping when route state changes or a row renders
without controls.

**Consequence.** Instructor rows follow the settled Human Guidance mappings.
Student rows follow their fixed Human Guidance mappings. Sysadmin tier-two
contents remain unresolved; a route that currently renders no task controls
does not establish an intentionally empty menu. Breadcrumb and page-content
geometry stays stable for every signed-in route.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md)'s permanent breadcrumb-row
rule, [application_shell.tsx](../src/application_shell.tsx), and
[app_ribbon_density.css](../src/ribbon/app_ribbon_density.css).

### PageFrame owns page-level structure

**Decision.** `PageFrame` owns page identity and fixed page-level layout: the
required title, optional `eyebrow` and `lede`, optional page-level actions,
content origin, standard vertical spacing, and width. Its content is one
column stack. `PageSection`, in the same module, owns an in-page heading,
optional helper and actions, and that section's body. `contentClass` remains
only for a task-specific arrangement inside that stack. Reading width is the
default; a route selects `fullWidth` only for dense content, using the
page-layout contract rather than the Ribbon contract. The frame root has fixed
production geometry and is not caller-classed.

**Why.** The WP-C1 baseline found 44 page-heading candidates: 24 used eyebrow,
title, and lede; 14 used eyebrow and title; four used only an eyebrow; and two
used only a title. My Active Courses and My Draft Questions also repeat the
page-level create action before their collections
([course_list_page.tsx](../src/pages/course_list_page.tsx),
[question_drafts_page.tsx](../src/pages/question_drafts_page.tsx)). Those
controls remain supplied by their page, while actions inside a form, editor,
or record remain task content. The shared frame places admitted page-level
slots consistently, as [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) requires.

**Consequence.** `PageFrame` places WP-C1-admitted page-level slots in the same
location and gives page content one fixed origin and spacing region. Callers
provide only the controls and subject-specific children. A page
may omit the eyebrow, lede, or actions, while every frame supplies one title.
Only routes that need dense content select `fullWidth`; ordinary routes inherit
the reading width. This makes one-off frame-width and root-class overrides
unavailable to page callers.

**Owner.** [src/components/page_frame.tsx](../src/components/page_frame.tsx)
and [src/components/page_frame.css](../src/components/page_frame.css).

### Reserve space but never fabricate a control

**Decision.** Reserve structural shell space independently from product
capabilities. A control appears only for a real workflow. An unavailable
required Instructor destination is an annotation-free unavailable control,
not a usable link; empty collections retain their working destination and an
honest empty state.

**Why.** Space prevents layout movement, while invented or apparently usable
controls misrepresent the product.

**Consequence.** Sysadmin tier-two contents remain unsettled until
product evidence defines their destinations and order. Current empty-row
rendering is presentation state, not an approved empty-menu design. The
Student's fixed rows are defined in **Student Tier 2 groups follow Tier 1
purposes**. The browser treats control visibility as navigation presentation,
never as permission evidence.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md)'s required and unavailable
Instructor destination rules, [app_ribbon.tsx](../src/ribbon/app_ribbon.tsx),
and [ribbon_selected_tab_visibility.ts](../src/ribbon/ribbon_selected_tab_visibility.ts).

### Record presentations and server pages have separate ownership

**Decision.** Each record component uses semantics that match its task:
`RecordList` for compact scans, `RecordSequence` for saved order,
`RecordTable` for named columns, `RecordOutlineList` and `RecordOutlineItem`
for nested membership, and `RecordDetailList` for expanded reviews and
comparisons. They share loading, empty, and error states while callers retain
domain and workflow state. `RecordTable` owns its shared table skin and internal
horizontal scrolling; callers set task-specific column proportions and logical
alignment. Bounded server pages replaced client windowing. Question, Pool, and
Blueprint discovery each keep one server page of 50, 100, or 250 records.
The client window helper is gone.

**Why.** A scan row, table row, ordered entry, hierarchy, and full review have
different structural meaning. A shared component should remove repeated
presentation code without erasing those differences. A shorter client slice of
a larger result is no longer useful once discovery already returns one page.

**Consequence.** All presentations use the same accessible collection states.
RecordList renders the returned page. It does not calculate a mounted slice,
overscan, spacers, or scroll-to-record position.

**Owner.** The shared components under
[src/components/record_list/](../src/components/record_list/)
and their composition boundary in [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md).

### Scan rows show one decision-sized summary

**Decision.** A scan row carries the title and the facts a person needs to
compare items without opening each one, plus the main action. Human Guidance
names those facts for each collection. Question Library scans keep description,
authors, classification, and the exact Question ID on the row. Public Blueprint
scans include Course name, classification, author, institution, and useful usage
or stewardship signals as HG requires. Existing API fields do not cap the
required result content.

**Why.** Repeated records need a compact, comparable decision surface rather
than a compressed detail page.

**Consequence.** This rule applies only to scan rows. Dense task tables such
as Gradebook and roster retain the columns their task requires; they are not
capped by the scan-row summary.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md)'s Instructor list-density
guidance and the Question Library scan-row regions in
[question_library_search_definition.ts](../src/pages/question_library_search_definition.ts) and
[library_page.css](../src/pages/library_page.css).

### Reorder controls share mechanics, not workflow policy

**Decision.** The six current reorder sites were compared before extracting a
shared control. The two Blueprint Course editors share deferred-save behavior.
Fork application, Assessment entries, members of the referenced Pool, and
Student Ordering differ in persistence timing, disabled policy, failure
recovery, or announcement behavior. The shared layer therefore provides array
movement plus accessible `RecordList` or `RecordSequence` controls; each caller
keeps its workflow policy.

The Pool-member rows below record the earlier implementation. Current HG supersedes their
membership-reordering behavior: Pools are unordered sets and their editors support display sorting.
Assessment entry order and Student Ordering interactions remain distinct from Pool membership.

| Site                          | Save timing                                                  | Disabled and failure behavior                                                                         | Announcement                                                           |
| ----------------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Blueprint Assessment editor   | Local Blueprint draft; outer Blueprint Save persists it      | Editor availability governs changes; outer Save retains the draft on failure                          | Parent change notice describes the unsaved order                       |
| Blueprint Pool members editor | Local Blueprint draft; outer Blueprint Save persists it      | Editability and list boundaries govern changes; outer Save retains the draft on failure               | Parent change notice describes the unsaved member change               |
| Blueprint fork application    | One explicit apply request                                   | Busy, locked, invalid, or empty selection blocks apply; conflict or uncertain result requires refresh | Result message reports saved, correction, or refresh state             |
| Assessment entries            | Local Assessment draft; Save Questions and order persists it | Busy or reload-required state blocks saving; conflict recovery reloads or discards the draft          | RecordSequence announces each move and returns focus                   |
| members of the referenced Pool | Local Assessment draft; Save Questions and order persists it | Attestation, availability, dirty state, busy state, and list boundaries govern changes                | The editor explains the pending Pool state; it has no move live region |
| Student Ordering response     | Each move updates the response controller                    | Locked response state and list boundaries block changes; controller owns response-save failure        | The response control announces each move and returns focus             |

**Why.** Similar move buttons do not establish identical state transitions.
The comparison preserves the Student response timing required by its workflow
(ASVS 2.3.1) while removing duplicate array movement and accessible focus
mechanics.

**Consequence.** A shared reorder component provides generic live move feedback
and focus restoration. Callers retain save timing, disabled rules, failure
handling, and workflow/result messages. Text is rendered through Solid JSX,
and navigation values remain validated routes or links rather than generated
HTML or unvalidated URL protocols (ASVS 1.2.1 and 1.2.2).

### Role interfaces expose only real capabilities

**Decision.** Student, Instructor, and Sysadmin interfaces differ by actual
role responsibility. Future controls are not presented as usable. Student View
is an Instructor preview mode, not a second Account role or persistent Student
record.

**Why.** Disabled or speculative controls teach the wrong workflow.

### Account avatars are one role-neutral aggregate

**Decision.** PLE stores the current avatar in one role-neutral
`account_avatar` aggregate, rather than keeping an Instructor-only projection.
Its discriminator is generic absence, a closed `ProvidedAvatarId` catalog, or
a self-owned Profile image. The server derives `/api/profile/avatar*` from the
signed-in Account; callers do not supply an Account identifier. Students may
select only a provided avatar and are denied image upload. Instructors and
Sysadmins may select a provided avatar or add their own Profile image. Instructor
Profile images are visible to everyone with a PLE Account, with no separate image
permissions mechanism. Self-owned editing does not imply self-only viewing.

**Why.** The same selected-or-generic avatar must represent each user
consistently, while the allowed choice differs by User Role. A single
aggregate avoids treating Profile images as an Instructor-only feature and
keeps the authorization boundary explicit.

**Consequence.** Generalize the existing Profile Thumbnail saga rather than
adding a parallel media path. Change the owning base-schema modules directly
because PLE is pre-production. Implement in dependency order: model, schema,
store, server, then frontend. The shared frontend projection renders the
generic, provided, or Profile-image result as a consistently cropped rounded
square. This resolves the avatar and Profile-image bullets in
[User top bar](HUMAN_GUIDANCE.md#user-top-bar) and the Instructor generic-icon
bullet in [Instructor interface](HUMAN_GUIDANCE.md#instructor-interface).

**Owner.** The avatar milestones in the active
[Human Guidance implementation compliance plan](archive/human_guidance_implementation_compliance_plan.md)
own the implementation details. Human Guidance remains the product authority.

### PLE-provided avatars are a versioned first-party catalog

**Decision.** PLE-provided avatars are a versioned, first-party static catalog.
The canonical, source-controlled `assets/avatar_catalog/` contains a manifest,
original SVGs, and `PROVENANCE`. A deterministic generator derives the Rust
registry, TypeScript catalog, and SQL seed from that source. Each catalog entry
has a stable ASCII ID that is never renamed or reused, and an explicit
`selectable` flag. A retired entry remains renderable but is not selectable.
`provided_avatar.is_selectable` is the database selection boundary and accepts
only `true` entries.

The original art is abstract toy-brick, color, and pattern art. It contains no
LEGO marks or copied minifigure art and is licensed CC-BY-4.0. The catalog
accepts only a safe, bounded SVG grammar. The app publishes the SVGs as static,
same-origin, fingerprinted public assets. It has no catalog-list API; the
self-choice API exposes only the signed-in Account's selection, and the server
and database authorize the generated seed.

**Why.** A generated first-party catalog keeps displayed art, selectable IDs,
and database authorization in one auditable release unit without letting an
untrusted client choose an object or enumerate other Accounts' choices.

**Consequence.** Deploy and roll back the generated registry, TypeScript
catalog, SQL seed, and fingerprinted assets together. A fresh-install check
proves that unit. An unknown stored ID renders the safe generic avatar and
causes catalog-drift repair; it never becomes selectable. C40 owns the
provided-avatar catalog and reusable picker contributor. C819 owns role-neutral
Profile Settings authorization; C820 owns the real `/profile` route and page
integration. The catalog and picker may be completed before those routes, but
no C40 Human Guidance bullet closes until C819 and C820 make Student selection
discoverable in real Profile Settings. Instructor Profile-image viewing follows
HG's public-within-PLE Instructor Profile rule.

Use temporary generator, schema, fresh-install, and unknown-ID checks while
building the catalog. Retain a permanent test only if it satisfies every
`PYTEST_STYLE.md` checklist item and protects the stable public behavior that
a retired ID renders but cannot be selected. A retained-test failure means the
catalog's selection or backward-rendering contract regressed; correct the
generator, seed, or authorization boundary before closing the affected Human
Guidance item. Otherwise remove the checks at plan closeout.

**Owner.** C40 owns the provided-avatar catalog and reusable picker; C819 owns
role-neutral Profile Settings authorization; and C820 owns the real `/profile`
route and page integration. The active [Human Guidance implementation
compliance plan](archive/human_guidance_implementation_compliance_plan.md)
owns their implementation and verification. Human Guidance remains the product
authority.

### Course Banners use one exact-ratio delivery representation

**Decision.** After EXIF orientation, a Course Banner source is valid only
when it is a complete, positive still PNG, JPEG, or WebP with
`u64(width) == 5 * u64(height)`; incomplete, trailing, and polyglot inputs are
rejected. The server and `ple_api` upload-staging boundary both enforce that
oriented exact 5:1 rule. The existing 8 MiB source-byte and 20-million-pixel
limits remain. There is no minimum dimension; 1280 by 256 is guidance, not an
acceptance requirement. The system retains the immutable source and promotes
one lossless-WebP `Banner` rendition at 1280 by 256, at most 2 MiB: every
smaller valid source is upscaled and every larger valid source is downscaled
with aspect-preserving scaling and no crop or pad. Thus a valid 5 by 1 source,
a smaller valid source, and a 2560 by 512 source are accepted.

**Why.** One exact wide ratio lets the product provide a consistent course
banner without silently cropping teaching material or imposing a fabricated
minimum source size.

**Consequence.** `Hero` and `Card` renditions are removed directly from the
preproduction model, object records, base schema, Store, server, and client.
Each promotion mints a fresh opaque `CourseBannerId`; the private
rendition object identity is deterministically derived from the Course, that
reference, and the sole `Banner` discriminator. The current banner is delivered
only through one same-origin route with `no-store`; no caller selects an object
key or rendition. This is an atomic preproduction cutover. A failed rollout
restores the whole code and schema change together and rebuilds disposable
fixtures; it leaves no alias, mixed representation, or compatibility reader.
C813 owns responsive layout around the one delivered Banner representation.

Temporary checks prove the source-ratio boundary, no-crop/no-pad promotion,
and all-or-nothing promotion, then are removed. A permanent behavior oracle is
allowed only if every [PYTEST_STYLE.md](PYTEST_STYLE.md) checklist item is yes
and its failure blocks release because it signals a ratio, crop, or
partial-promotion regression. It must not freeze the exact 1280 dimensions,
source dimensions, private enum shape, saga slots, or filter implementation.

**Owner.** [CONTRACTS.md](CONTRACTS.md) and
`crates/question_model/src/course_appearance.rs`; the active
[Human Guidance implementation compliance plan](archive/human_guidance_implementation_compliance_plan.md)
owns the implementation and verification milestones.

### High-consequence actions are distinct

**Decision.** Assessment Unrelease, Published Question Archive, and Blueprint
Course Archive use a Danger Zone. Unrelease requires typing the Assessment
title. Archive actions explain their effect and require clear confirmation;
Human Guidance does not prescribe typed-title confirmation for them.

**Why.** These actions have meaningfully different consequences from ordinary
editing.

## Data, privacy, and operations

### Student Account and Course data have separate lifetimes

**Decision.** Student Accounts are global. Course relationship removal,
Account deactivation, or Course inactivity does not delete Student Work.

**Why.** Authentication, access, and educational-record retention are separate
legal and product concerns.

### Active lifetime and FERPA retention are separate

**Decision.** A Course Instance represents one teaching period and becomes
Inactive six months after creation, after an Instructor warning. Assessment
deadlines may move the end of normal teaching only within that Active lifetime.
The latest Assessment deadline starts the separate FERPA retention clock.
Records later leave normal interfaces, remain recoverable during the configured
retention period, and are permanently deleted. Course metadata, Assessment
definitions, Questions, and settings remain.

**Why.** The fixed creation anchor prevents reuse of an old Course Instance for
a later teaching period and prevents deadline extensions from indefinitely
delaying FERPA retention and deletion. The deadline anchor keeps FERPA
retention aligned with actual teaching dates. Becoming Inactive does not itself
delete Student records.

**Consequence.** Assessment Release Validation rejects deadlines after the
six-month limit. Instructors may bulk add Students through roster import but
remove Students only one at a time. The background checks are idempotent. FERPA
durations and table/job shapes are operational and implementation decisions not
specified by Human Guidance. See [RETENTION_POLICY.md](RETENTION_POLICY.md).

### APIs remain stateless and durable state is shared

**Decision.** Correctness-bearing state belongs in PostgreSQL, typed object
storage, or the responsible Question Backend boundary, not API-process memory
or browser caches.

**Why.** Requests must survive restarts and multiple replicas.

**Consequence.** Caches contain answer-free reusable data only and never become
authorization, response, timing, or grading authority.

### Object storage is typed and server-owned

**Decision.** The database owns logical object identity and scope; the server
constructs storage keys and verifies integrity. Browsers use authorized logical
delivery routes.

**Why.** Raw paths and bucket prefixes are not authorization models.

### Background workers are justified by exact product needs

**Decision.** Expired-Attempt submission and retention checks use idempotent
background processing because they must complete without a connected browser.
Bounded asset preparation may use an operation-specific background mechanism.
A generic worker framework does not authorize grading
queues, recovery states, audit machinery, or compatibility jobs.

**Why.** Expiry and retention obligations must complete without a connected browser, while other
background mechanisms need their own demonstrated product requirement before adding durable state.

### Retention notification delivery is a separate least-privilege boundary

**Decision.** Retention notices use a provider-neutral delivery boundary that
is separate from invitations, browser sessions, API serving, object storage,
and Question renderers. One notification identity is
`(course_id, action_kind, due_at, recipient_account_id)`, where `action_kind`
is exactly `warn_inactive` or `notify_archive`; archive and delete never
notice. Its recipients are
the deduplicated union of the Course's assigned Instructor and active
Instructor memberships/accounts. A leaseable receipt records an idempotency
key and provider acceptance. A redacted notice directs the
recipient to sign in; it excludes Course identifiers and title, raw IDs,
FERPA data, a capability, and a recovery link.

**Why.** Retention warnings must be repeat-safe and reach current teaching
staff without turning an email provider, the Live Demo, or a browser-facing
path into educational-record authority.

**Consequence.** The notifier capability claims one receipt at a time with a
lease and `SKIP LOCKED`, derives the current eligible recipient and current
verified email destination for that Instructor on every claim rather than storing an address
snapshot or offering generic Account lookup, and cannot create a second send
for the same identity. A receipt starts with `next_attempt_at =
due_at`. At one evaluated timestamp, a claim requires the action still be due,
`next_attempt_at` be due, no provider acceptance, and no lease or an expired
lease; it orders by `(due_at, id)`, increments the attempt count, and advances
`next_attempt_at` to lease expiry. The durable idempotency key exists before a
provider call: a crash before it leaves the lease to expire, while a crash
after it reclaims only after expiry and reuses that key. Provider acceptance is
terminal for sending; the boundary makes no inbox-delivery claim and has no
delivery callback or provider exactly-once promise. A recorded pre-acceptance
failure clears the lease and sets
`next_attempt_at = failed_at + min(3600 seconds, 60 seconds * 2^(attempt_count

- 1))`. An action no longer due cannot retry, and an accepted receipt never
resends. `NotConfigured`records a non-send failure and never reports fake
success. A late run attempts required earlier notices in due-time order, then
performs the due transition after successfully recorded failure; it stops only
the notice lane for an unknown typed Store state and always continues retention
transitions. Provider credentials are operational configuration; the Live Demo
remains`NotConfigured` and is not delivery evidence.

One isolated retention process has exactly two independently attested,
non-inheriting database profiles/pools: C215's retention executor and C848's
notifier. It has no third database profile and no API, S3/object-store,
session, or renderer authority. Do not reuse invitation export, Mail.app, or a
generic outbound-email path.

**Why.** Infrastructure should implement a decided behavior, not create product
behavior by implication.

## Implementation and evidence

### One Live Demo runtime owns every browser evidence lane

**Decision.** PLE has one canonical Podman Live Demo runtime. Its one Caddy gateway serves the
production frontend bundle, owns the local HTTPS browser origin, proxies the API and public assets,
and applies the shared routing, readiness, and browser-security policy from
`containers/Caddyfile`. `./launchers/run_live_demo.sh`, Playwright, screenshot capture, and focused
connected browser verification all exercise that same topology, seeded data model, gateway
behavior, and application paths.

**Why.** A browser-test-specific gateway or alternate demo topology can pass while the human Live
Demo is broken, turning test plumbing into an accidental second product runtime. Caddy remains a
small, deliberate infrastructure boundary; duplicating its behavior does not.

**Consequence.** The browser Compose overlay may add only mechanics required to run the same
canonical runtime under browser automation, such as scoped certificate trust and fixed test
addressing. It must not replace the Caddyfile, routing, headers, readiness,
asset delivery, frontend bundle, authentication model, or seeded product graph. A gateway change is
accepted only through both the normal Live Demo launcher and the canonical connected-browser path.

### One canonical database baseline serves fresh installation

**Decision.** Pre-production database structure has one reviewed canonical
fresh-install path. Installation data is separate from structure and is
explicitly selected.

**Why.** Fresh and repeated installation should be understandable and
deterministic.

### Tests prove behavior at the owning boundary

**Decision.** Fast deterministic tests, connected PostgreSQL tests, live
backend probes, and browser acceptance prove different things. A passing lower
layer does not claim acceptance at a higher layer.

**Why.** Mocked or structural evidence cannot prove a real teaching workflow.

**Consequence.** Tests are liabilities as well as assets. Permanent tests
protect stable external behavior, not internal call order, inventories, current
dates, artificial delays, or tunable defaults. Bounded work runs its narrow
gate. Full Podman and `source source_me.sh && ./launchers/all_test.sh` acceptance
runs at major milestones, not after every small source or documentation slice.
See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

### Current implementation and target product remain distinguishable

**Decision.** Documentation may describe an existing old route, table, or UI
when needed for migration or operations, but must label it as implementation
evidence and state the target Human Guidance term or behavior nearby.

**Why.** Pretending old implementation does not exist is inaccurate; treating
it as product authority perpetuates it.

### Public IDs are the only stored identity for public aggregates

**Decision.** Account, Course Instance, Assessment, Blueprint Course, Published
Question, and Question Pool primary keys are the public ID domains. There is
no parallel UUID key and no `reference_number` or `public_reference` column.

**Why.** Human Guidance names those public IDs; dual identity forced every
reader to choose which key was real.

**Consequence.** Rust and TypeScript wrappers are `AccountId`,
`CourseInstanceId`, `AssessmentId`, and `BlueprintCourseId` over `String`.
SQL keys follow DATABASE_STYLE (`published_question_id`,
`course_instance_id`). JSON fields for those objects are `id` (or
`courseInstanceId` / `assessmentId` when nested). There is no parallel `reference`
property and no SQL `public_reference` alias. HTTP `ETag`/`If-Match` encode the
aggregate Edit Number as a quoted decimal string; domain APIs use the qualified
Edit Number. Assessment Attempts, Assessment Entries,
and sessions stay UUID because they are not Human Guidance public IDs; their
JSON field is also `id`. PLE is pre-production, so this cutover edits the
contract directly.

### Frozen Assessment policy and Entry facts are content-addressed snapshots

**Decision.** Equal Assessment policies share one
`assessment_policy_snapshot` row (SHA-256 primary key). Equal Entry
configurations share one `assessment_entry_snapshot`. Attempts pin those
rows; later Assessment edits do not rewrite already-started work.

**Why.** Copying title, instructions, and policy strings onto every Attempt
multiplies the fastest-growing rows and cannot be made consistent later.

**Consequence.** Score readers apply current Entry points to the frozen
normalized credit. Scoring rules stay on the Entry snapshot.

### Saved responses finalize in place

**Decision.** There is no `question_response` copy and no
`question_response_grading` wrapper. Submission sets `finalized_at` and
`assessment_submission_id` on `assessment_attempt_saved_response`.

**Why.** Finalized bytes were stored twice; derived state columns drifted
from the facts that implied them.

**Consequence.** Unanswered Issued Questions have no saved-response row.
Unrelease audit counts finalized `assessment_attempt_saved_response` rows as
`finalized_saved_response_count`. JSON may still project
`question_attempt_state` from remaining facts.

### Saving changes to Question Pools

**Decision.** Pools are not a Revision family. A Pool contains an unordered set of Question
Revision Tuples. A Question Pool can contain a Question ID only once. The Question Pool also has its
own properties, defined separately from the properties of the Published Questions it contains.
Those Questions share one Type, Backend, Discipline, and Subject. Each Pool owns its search metadata
and has an owner rather than an Author field;
its compatible license is calculated from member licenses. Initial release supports CC0, CC BY,
and CC BY-SA; NC and ND Questions are excluded and may be reconsidered after release.
Members live in `question_pool_member`; saves compare-and-swap the Pool Edit Number.
Spreadsheet-style sorting changes only the editor display.

**Why.** Saving updates the Pool and advances its Edit Number; no Revision is created.

**Consequence.** Student Work keeps the Question Revision Tuple, Pool ID,
and Pool Edit Number. Forks copy the current set of Question Revision Tuples once. Show which Pool
requirements are not met and block affected Assessment release until resolved; Assessments already
released when a problem develops continue as-is. Per-Question uniqueness and removal of separate Pool Author storage are implemented. Current
ordering and admission-only classification checks still require implementation alignment.
A role-typed composite foreign key enforces Instructor ownership. See
[QUESTION_POOL_SPEC.md](QUESTION_SPECS/QUESTION_POOL_SPEC.md) for the product boundary and
[question_specs_alignment_report.md](active_plans/reports/question_specs_alignment_report.md)
for implementation differences.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-pool-specifications)

### Metadata is assigned or required

**Decision.** PLE assigns metadata where the value follows from existing content, and publication
requires the remaining mandatory fields. Question Type is non-NULL and remains author-declared
or imported. Bloom may be NULL awaiting initial AI assignment, which remains deferred. No time
limit is enforced; the proposed 24-hour deadline was withdrawn. Explicitly optional fields remain
optional. [QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_METADATA_SPEC.md) lists the distinctions.

**Why.** Instructors should not need to supply every metadata value manually or be relied upon to
notice missing required fields. Pending AI classification is different from missing required content.

**Consequence.** Validate required metadata at publication and trusted write boundaries. A combined
Question/Pool result must not turn a required common field into an optional field. This documentation
change does not implement AI processing or certify every write path.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-library-metadata)

### Remove Course-use search filtering

**Decision.** Remove the "Used in my Courses" filter from Question Library search and pickers.

**Why.** Neil never approved it. He questions the value of searching for content already in use
and considers its server overhead unnecessary. No performance measurement is claimed.

**Consequence.** Remove the control, query parameter, result count, and Course-membership joins
from Library reads. Existing Course access checks and Question usage statistics keep their own
purposes. Remaining SQL filters retain parameter binding (ASVS 1.2.4), and server request
validation retains its allowed fields (ASVS 2.2.1-2.2.2).

**Owner.** [QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_FILTER_SPEC.md),
[question_library_operations.sql](../schemas/base_schema/50_functions/question_library_operations.sql).

### Combined Library search uses Pool membership filters

**Decision.** Published Questions and Pools share one search. Questions in no Pool means
Published Questions with no Pool membership, including membership in Pool forks.
This follows from ordinary Pool membership; a fork is itself a Pool. HG currently prefers those Questions plus Pools
as the default; retain its tentative "probably" qualifier. Filters can include member Questions.
Pools match only their own text and metadata. One server query filters, sorts, and pages the
combined results; individual member matches do not expand Pool matches.

**Why.** The Library should show reusable Pools without flooding results with their members.
Member-inclusive search remains available when the Instructor needs individual Questions.

**Consequence.** The implemented Library uses one server union with global filtering, sorting,
facets, and cursor paging. `/library/{id}` resolves object kind before mounting Question or Pool
detail. The Question picker remains Question-only and preserves membership requirements; the
Assessment content picker selects Questions or one Pool through the mixed Library definition. The
Library display default does not make a Question in another Pool ineligible for membership.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-library-specifications)

### Searches share session behavior

**Decision.** Shared search behavior lives in `src/features/search/`. Each content definition
supplies its typed query, decoder/API call, row identity, record description, filters, and orders.
Text applies on Enter or Search; filters, sort, and page size apply immediately. Search state
lasts only while the page or dialog is open.

**Why.** Quoted phrases, exclusions, and field syntax pass through incomplete states while typed.
Explicit submission gives each result count a clear cause. Shared request numbering prevents old
responses from replacing newer results without mixing content-specific APIs or domain rules.

**Consequence.** New queries clear rows, counts, and selection. Paging failures retain the current
page and retry the exact failed request. Page-size changes preserve selection but reset cursors.
Result kinds and Pool membership remain Library rules. The Question picker keeps its ordered tray
outside shared search state: search Clear resets discovery, while its separate Clear selection
action resets the tray. The Assessment content picker owns its Question-or-one-Pool selection rule.

**Owner.** [search_session.ts](../src/features/search/search_session.ts)

### Pool credit identifies its owner and source Pool

**Decision.** Pool credit consists of its owner and, for a fork, its source-Pool link.
The shared-search plan's calculated-license step removes the unused manual Pool provenance
contract, including source text and source URL, alongside its manual license. Each member keeps
the authors, citation, and license of its exact Question Revision.

**Why.** HG names owner and source Pool as the Pool's credit fields. The current Rust and browser
interfaces already follow that model; the remaining manual provenance functions exist only in SQL.
Keeping a second Pool source description would preserve an unused competing credit contract.

**Consequence.** The implementation removes author and attribution storage and the remaining
provenance table and functions, while adding the calculated Pool license. Obsolete manual source
fields leave with that table. Each member's exact Revision retains its own authorship, source, and
license evidence.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-pool-metadata)

### Library usage statistics are retained counters, not reconstructions

**Decision.** Keep Question statistics separately per Published Question Revision and per Pool.
Pool statistics accumulate from Questions delivered through that Pool, including across changes
to its set of Question Revision Tuples.
Show times received by Students, graded-response count, average stored credit, full-credit
percentage, and zero-credit percentage. Use the Backend's stored credit fraction. The agreed
measures are specified in [QUESTION_LIBRARY_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_SPEC.md#usage-statistics).
Cross-Revision rollups are outside approved scope.

**Why.** Neil wants to see how often Students received a Question and how they performed.
Average credit, full-credit percentage, and zero-credit percentage describe different aspects
of those outcomes; the graded-response count gives their sample size. Assessment points are a
separate concern.

**Consequence.** Keep privacy-safe aggregate statistics separate from identifiable Student Work.
Student-record deletion preserves anonymous aggregates. Reconcile the existing counters and
Library displays with the agreed measures through [TODO.md](TODO.md#question-spec-implementation-follow-up).

**Current implementation evidence.** The earlier design entry specified detailed submission
counters, credit sums and squared sums, per-member Pool selection counts, derived Pool difficulty,
and cross-Revision display rollups. Those details are not established product choices merely
because this entry previously called them settled. Storage is described in
[statistics.sql](../schemas/base_schema/20_tables/statistics.sql); evaluate implementation against
the agreed measures rather than treating the former entry as authority.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-library-object-usage-statistics) and
[QUESTION_LIBRARY_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_SPEC.md#usage-statistics).

### Creation clocks are `timestamptz` for enforcement and `date` for authored content

**Decision.** Student Work, sessions, events, Courses, and Accounts use a
full-precision `timestamptz` creation clock. Published Questions, Question
Pools, Published Question Revisions, Blueprint Courses and their Revisions,
Draft Questions, and usage
statistics use a `date`.

**Why.** Ordering and audit need sub-day precision. Authored content and
global counters must not store a time of day that could identify a
Student.

**Consequence.** [DATABASE_STYLE.md](DATABASE_STYLE.md) "Every table has a
clock" is the mechanical check. Domain CHECK helpers run as the current
user, so their `EXECUTE` grant is `PUBLIC`.

**Owner.** [DATABASE_STYLE.md](DATABASE_STYLE.md) "Every table has a
clock".

### Screenshot manifest is generated from scenario declarations

**Decision.** Each screenshot is defined once in a scenario
`captureCheckpoint` declaration. Publish writes
`docs/screenshots/current_capture_manifest.json`, the receipt, the atlas,
and coverage ledgers. `docs/screenshots/coverage_exceptions.json` is the
only hand-kept coverage list. The reached route is observed at capture
time.

**Why.** Hand-editing the manifest, a checkpoints list, the scenario body,
and coverage ledgers together drifted. Pre-production can fix ownership
instead of adding compatibility aliases.

**Consequence.** Adding or removing a capture is one scenario-file edit
followed by `./devel/capture_screenshots.sh`. Generation fails closed on
an uncovered unlisted surface. `--verify` regenerates the manifest in
memory from the live replay and fails when the committed file differs.

**Owner.** [HOW_TO_SCREENSHOT.md](HOW_TO_SCREENSHOT.md).

### PLE accepts converter-generated Native JSON directly

**Decision.** PLE accepts Native JSON directly from qti-package-maker-rs, with no intermediate
Question format. The existing Native JSON specification owns the unversioned format; Question
metadata stays separate. Imported image assets use existing PLE image storage and accompany the
text-only JSON rather than a ZIP package.

**Why.** Neil selected the direct Native JSON handoff and existing asset storage.

**Consequence.** Use the ordinary Draft image upload and metadata paths. Converter and Draft
integrations remain separate implementation work in [TODO.md](TODO.md#future-product-capabilities).
Native JSON display-content strings carry HTML with inline CSS, including ordinary `img src`
references resolved through the existing image tuple and asset storage. This does not change the
source shape, asset identity, or author-JavaScript isolation rules; sanitization remains deferred.
The converter supplies Native JSON and referenced files using content-relative paths; PLE resolves
those paths and owns Question Image Asset IDs, checksums, and storage identities.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#draft-question-specifications),
[NATIVE_JSON_SPEC.md](QUESTION_SPECS/NATIVE_JSON_SPEC.md), and
[QTI_INTERCHANGE_SPEC.md](QUESTION_SPECS/QTI_INTERCHANGE_SPEC.md).

### Question Image, QTI package, and Object are different roles

**Decision.** A QTI ZIP, a retained QTI archive, a still image extracted during
import, a Question-bound displayed still image, an authorized delivered image,
and a physical storage record are different roles. Current Question-bound media
is PNG, JPEG, or WebP only, so the Question-side names are
`QuestionImageAsset` and `QuestionImageRendition`. QTI import uses
`QtiPackageUploadFile`, `QtiPackageArchive`, and
`QtiPackageExtractedImage`. `Object` / `ObjectId` is only the physical
record. Course Banner, Profile Image, and WeBWorK renderer files keep their
own names.

**Why.** Human Guidance names **Question Image Asset** as the Question-bound
still image and **Question Image Rendition** as its authorized delivered form.
A QTI ZIP, retained archive, and extracted image are interchange roles.
`Object` / `ObjectId` is physical storage. A shared `asset` type made readers
infer authority and lifecycle from implementation details, including assigning
`QuestionImageAssetId` to a QTI extract before it was a Question-bound image.

**Consequence.** Domain APIs, SQL, JSON, and Object Address kinds use these
role names. SVG upload input is rewritten into a WebP Question Image Asset
before storage. The stored kind remains PNG, JPEG, or WebP. Non-image Question
media is not a current product type.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md),
[OBJECT_STORAGE.md](OBJECT_STORAGE.md).

### Shared Theme value and independent display mode

**Decision.** `Theme` is the shared closed value for Course appearance and an
Instructor's personal appearance. Course operations keep their specific names:
`CourseThemeUpdate`, `CourseThemeStore`, `updateCourseTheme`, and
`update_course_theme` write a Course's selected `Theme`. The database value
table is `ple_data.theme` with `theme_id`. A viewer's nullable `DisplayMode`
preference is `light`, `dark`, or unset; unset follows the browser. `grass` is
the default Theme.

A Course created from a Blueprint starts with the Blueprint's Theme and can be changed
independently afterward. Neil clarified that this follows the meaning of a Blueprint; the missing
Blueprint Theme field is an implementation gap, not a product choice between copying and ignoring it.

**Why.** The values no longer belong only to Courses. Keeping Course names at
the write boundary states what the operation authorizes. A nullable preference
expresses browser following without introducing a third display mode or a mode
rules engine.

**Consequence.** Every Theme look supplies exactly Canvas, Surface, Secondary,
Accent, and Highlight. One shared derivation supplies the remaining concrete
tokens: readable Canvas ink, Canvas-muted text, Accent actions, Accent/ink
links and focus/strong borders, Accent mixed toward opposite readable text for
hover, and Canvas/Secondary-mixed borders. Theme resolution selects the
authorized Course Theme from a Course, Assessment Attempt, or Assessment
Attempt History route first, then an Instructor's personal Theme on global
Instructor pages, then `grass`; display mode resolves separately from that
Theme choice. `AppearanceOwner` is the sole document-level appearance owner:
it writes the resolved Theme tokens,
`data-theme`, `data-display-mode`, and `color-scheme` to `<html>`. Pages and
Course controls provide only route data or an unsaved Course-Theme preview to
that owner; they do not create nested Theme scopes.

**Owner.** [whole_interface_theme_plan.md](archive/whole_interface_theme_plan.md).

## Unresolved decisions

The complete Sysadmin Ribbon layout is unresolved. HG's Deferred section and
the [interview follow-up](active_plans/decisions/HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md)
track other deferred or unsettled decisions; this section is not an exhaustive
list. Product documentation should
not turn hypothetical capabilities, tunable FERPA retention intervals, or
speculative failure machinery into additional unresolved product questions.

See the temporary
[COMPLIANCE_SUMMARY.md](archive/reports/human_guidance_compliance/COMPLIANCE_SUMMARY.md)
for the corpus review.

### Complete Question records and regular Pool forks

**Decision.** Each Question Revision is a complete record. Permitted metadata edits update fields
on the current record in place; source and grading changes publish another complete Revision.
Bloom follows ordinary metadata editing, including ordinary record concurrency. Native JSON Type
comes from its interaction; other Backends use editable Type classification. Pool forks are regular
reusable Pools with Instructor owners and parent-Pool pointers. Saving changes updates the Pool,
with no undo; their Edit Number is a concurrency counter, not recoverable history.

**Why.** Neil's October 5 clarification rejects special metadata revision systems and Pool kinds.
A fork may be used in hundreds of Assessments. Conservative Revision creation and ordinary fields
keep the model consistent.

**Consequence.** Correct the specs now and track storage, API, permission, and editor alignment in
[TODO.md](TODO.md#question-spec-implementation-follow-up). Keep historical implementation evidence
separate from current product rules.

**Owner.** [QUESTION_REVISION_SPEC.md](QUESTION_SPECS/QUESTION_REVISION_SPEC.md),
[QUESTION_BLOOM_CLASSIFICATION_SPEC.md](QUESTION_SPECS/QUESTION_BLOOM_CLASSIFICATION_SPEC.md), and
[QUESTION_POOL_SPEC.md](QUESTION_SPECS/QUESTION_POOL_SPEC.md).

### Shared Pool references and explicit forks

**Decision.** [QUESTION_POOL_SPEC.md](QUESTION_SPECS/QUESTION_POOL_SPEC.md) owns all Pool behavior.
Assessment entries reference existing Pools and store their own selection counts. Pool forking is
explicit and creates a regular Instructor-owned Pool. Current edits affect future selections at
every reference; existing Attempts keep selected Questions. Save incomplete unreleased Assessment
work and show when an Assessment requests more Questions than a Pool can provide.
Release validates completeness.

**Why.** Neil replaced automatic fork-on-add with one reusable Pool model and
separated valid Pool membership from an Assessment requesting too many Questions. One spec keeps
membership, ownership, selection, and editing rules together.

**Consequence.** Reconcile runtime behavior through [TODO.md](TODO.md). The five smaller Pool specs
are folded into the main spec; Published Question forks retain their separate Draft/Revision spec.

**Owner.** [QUESTION_POOL_SPEC.md](QUESTION_SPECS/QUESTION_POOL_SPEC.md).
