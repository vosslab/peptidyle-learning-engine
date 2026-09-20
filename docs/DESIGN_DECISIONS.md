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
names Regular Assignment, Practice Question Assignment, and Bonus Assignment.
Quiz and Exam complete the five current Types.

**Why.** A single generic noun keeps course content, attempts, navigation, and
data relationships understandable while Types communicate teaching purpose.

**Consequence.** Schema, Store/server contracts, HTTP routes, DTOs, and
browser paths use Assessment as the generic object. Assignment remains only
in Regular Assignment, Practice Question Assignment, and Bonus Assignment.
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
three Type display names: Regular Assignment, Practice Question Assignment,
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

**Decision.** Published Questions, published Question Pools, and Blueprint
Courses have immutable Revision families. Draft Questions, Course Instances,
Assessments, Attempts, Student Work, names, and lifecycle metadata use current
state. Edit Numbers are concurrency controls, not historical content.

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

### Product roles are global and exclusive

**Decision.** Each global Account has exactly one immutable Product Role:
Student, Instructor, or Sysadmin. A person needing multiple roles uses separate
Accounts.

**Why.** Role-specific interfaces and access rules remain explicit.

**Consequence.** Course relationships do not change the Account's Product Role.
Future Course Observer, Student Observer, and Grader roles are separate Course
relationships, not Product Roles. Grader is not currently needed because
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

### Verified Instructor Display Name is a vetting-time endorsement attribute

**Decision.** A bounded, server-controlled Verified Instructor Display Name is
captured only during real identity vetting and Account creation. It is not
self-editable, a Profile field, a directory entry, or a general Account
projection. An active Instructor may receive it only when viewing the vetted
Instructor Star list for a Published Question or the vetted Instructor Star
list for a Public or Archived Blueprint Course. Either projection contains no
email, UUID, Account reference, avatar, Course information, or substitute
identifier.

**Why.** Human Guidance requires vetted Instructors to see which vetted
Instructors endorsed a Published Question, but does not authorize an identity
directory or a mutable public Profile.

**Consequence.** C17/C18 establish the only write path. C370's authorized
Question projection and C856's authorized Blueprint projection may join that
controlled attribute only for an active Instructor on their respective
Published-Question or Public/Archived-Blueprint Star lists. A self-Star or
count route without exact verified names is interim evidence and cannot close
C371. Students, anonymous callers, inactive Accounts, Watch surfaces, and every
other surface receive neither the name nor a substitute identifier.

### Sysadmin is platform administration, not ambient FERPA access

**Decision.** Sysadmins manage platform configuration and operations but do not
automatically read Course Student records. Support access is deliberate,
scoped, and recorded.

**Why.** Operational privilege and educational-record access have different
purposes.

**Consequence.** A Sysadmin-created Course gains an ordinary Instructor
relationship for its teaching staff; the Sysadmin does not acquire Course
membership merely by creating or supporting it.

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
state. PostgreSQL derives the Account's stored Product Role and atomically
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
plan](active_plans/active/human_guidance_implementation_compliance_plan.md)
owns the implementation and verification details.

### Account Settings is one self-only time-zone preference

**Decision.** Every signed-in Product Role uses the same self-only
`/account-settings` surface and `GET` / `PUT /api/account/settings` boundary.
Both responses and the only accepted update body have the closed shape
`{ "timeZone": "exact IANA name" }`. The boundary accepts no Account,
Course, or Product Role selector. PostgreSQL derives the active Account from
the authenticated session and atomically reads or replaces that Account's
exact installed IANA name.

**Why.** Account Settings needs one small, understandable preference that does
not let a caller select another Account or accidentally turn a display choice
into Course authority. One boundary also removes the current split between
Student time-zone handling and the Instructor Profile editor.

**Consequence.** A successful update changes date display for the Account and,
for an Instructor, the wall-clock interpretation of dates entered later. It
never changes an already stored instant. Profile Settings owns avatar selection
and Profile-image work. Instructor Profile displays the current Account time
zone and links to Account Settings; it does not edit the zone itself.

This Account Settings boundary exposes no passkey, email, TOTP, recovery,
Account-status, or session control. Human Guidance's required Student and
Instructor passwordless authentication and multiple Student passkeys remain
owned by Accounts-and-roles milestones, not by Account Settings. The unresolved
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
plan](active_plans/active/human_guidance_implementation_compliance_plan.md)
own implementation and verification. C821 owns this scope decision.

### Account state preserves history

**Decision.** Deactivation blocks new access but preserves authorship, Course
relationships, Student Work, and history. Reactivation restores eligible
relationships. No permanent Account-closure workflow is currently defined.

**Why.** Authentication state must not become accidental content or record
deletion.

## Questions and Pools

### Draft and Published Questions are separate

**Decision.** A Draft Question is private, mutable, unpublished, and
unversioned. Publication creates or advances a stable Published Question
lineage with immutable Question Revisions.

**Why.** Private authoring and public reuse have different access, storage, and
evidence needs.

**Consequence.** Manual C351 Draft deletion and publication remain the only
approved Draft-removal transitions. Automated abandoned-Draft cleanup is not a
current feature: Human Guidance permits it but does not set its clock,
durations, warning, recovery, reset, cancellation, or delivery-failure policy.
No placeholder warning/recovery table, API, grant, Store, worker, generated
seam, or test seam may remain before that product design is approved. The
unresolved question is: **Should PLE automate cleanup of abandoned Draft
Questions? If yes, what event starts inactivity; how long until warning; how
long is the recovery period after a successfully delivered warning; which
save/edit/publication/ownership events reset or cancel it; and what is the
outcome when warning delivery fails?** Metadata edits that do not change
Question source do not create Revisions. A substantive fork creates a new
Question ID with attribution.

### Published Question forks create a new private Draft through one server command

**Decision.** A fork begins from one exact Published Question Revision and creates one distinct,
private Draft Question owned by the active Instructor who invoked it. The server resolves the
selected source, obtains the new Question ID from the server-side cryptographically random
allocator with its public SHA-256 checksum, and writes the Draft, ownership, exact source
Revision, and immutable attribution in one transaction. Later
publication derives and preserves that exact source Revision's compatible CC license; it rejects a
requested replacement even when that replacement is otherwise supported. The client
submits no new Question ID, source facts, authorship, Draft content, or attribution payload. It
may carry an opaque idempotency key; that key is bound to the active Instructor and exact source
Revision so a retry returns the same Draft and a key reuse for another source is refused.

**Why.** A lineage row alone cannot create usable private authoring state, and client-selected
identity or attribution would make provenance and collision handling untrustworthy.

**Consequence.** `question_lineages.sql` supplies only the immutable Published-Revision source
read/pin. The later-installed authoring operation owns Draft creation, access, and immutable
fork-source storage; it cannot be placed in the earlier lineage install phase. The typed Store and
server command authorize the active Instructor, resolve the source server-side, mint the ID, and
perform the atomic operation. The Instructor UI exposes that command and opens only the returned
private Draft. Publication continues through existing Question Publication Validation; a fork
never enters the Question Library directly. The operation records no generic recovery state or
compatibility path.

**Owner.** [CONTRACTS.md](CONTRACTS.md)'s Question lineage and revision boundary;
C319 and C876-C879 implement it in the active
[Human Guidance implementation compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md).

### Bulk metadata editing is an all-or-none current-state command

**Decision.** An active vetted Instructor may update selected Published Questions' global `tags`,
`subject`, and `topic` together. These are the currently defined shared search metadata fields.
The command replaces only fields explicitly present in its closed patch; an empty tag list or a
null subject/topic intentionally clears that field. It never edits source, answer, grading,
feedback, assets, backend, Question Type, authorship, ownership, availability, or a Question
Revision.

**Why.** Human Guidance requires practical cleanup of large imports but says metadata belongs to
the Published Question as a whole, not to an immutable Revision. A whole-batch outcome avoids
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
numeric batch maximum and future addition of another *stored shared search metadata field* are
operational/schema decisions. A future field must join the same closed patch and CAS contract; no
arbitrary JSON field patch is permitted.

First publication seeds current tags once from the validated source: native `PLE authoring` and
`Pilot` tags when present, or an empty WebWork list. Thereafter tags are database-owned current
metadata. A successor preserves an intentional clear unless an authorized later metadata command
replaces it.

**Owner.** [CONTRACTS.md](CONTRACTS.md)'s Bulk Published Question metadata boundary;
C365-C368 and C893 implement it in the active
[Human Guidance implementation compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md).

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

### Library improvement activity is retained

**Decision.** Active vetted Instructors may create, read, reply to, and edit
their own text-only improvement-thread posts for available Questions and Pools.
Question owners and Sysadmins administer Question thread state and impact
notices; Pool administration is Sysadmin-only. Resolved threads and cancelled
impact notices remain retained current records.

**Why.** Reusable teaching content needs a visible stewardship conversation and
durable impact history without exposing Watch subscribers or inventing a second
content Revision system.

**Consequence.** Threads target a stable Library Object and record the exact
Revision current at creation. Impact notices may name one existing affected
Revision. Sysadmins use a read-only Library mode for ordinary content controls
while retaining the explicit administration actions above. The source core does
not close private Watch delivery: final review, major-milestone SQL/browser
proof, and four-event notification acceptance remain pending.

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
[QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md).

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
stored identities. [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md) owns the exact
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
response. See [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md).

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
authorization. The same answer decision, including current-Course completion
for Quizzes and Exams, runs before and after rendering.

**Why.** PLE owns disclosure and WeBWorK owns answer presentation. Correct answers
must not pull independently controlled responses, explanations, Hints, Worked
Solutions, or new scores into the disclosure.

**Consequence.** History exposes only the literal availability marker. Each frame
fetch independently authorizes the fixed route and uses the existing opaque
script-only preview sandbox. Generated review assets stay inline in that
protected document. No public review asset, answer parser, persisted review,
grading write, or disclosure latch is introduced. Failure is local to review;
the issued document and immutable Student Work remain intact.

**Owner.** [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md),
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md), and
[webwork_document_route.rs](../crates/server/src/webwork_document_route.rs).

### Native PLE Question JSON stays deliberately small

**Decision.** Native PLE Question JSON is private, unpublished, unversioned,
strictly validated, and static. It supports the eight named native Question
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
and reviewed libraries, without parent initialization. Its only optional
outbound message is versioned `author-content.resize`, with finite integer
dimensions clamped to the declared bounds and accepted only when
`event.source === iframe.contentWindow`; opaque-origin `null` therefore still
has a source-identity check. The parent sends no messages and accepts no
answer, URL, HTML, navigation, storage, or API command.

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
use is limited to Regular Assignments, Bonus Assignments, and Practice Question
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
[Human Guidance implementation compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md)
and their verification. A future, product-approved H5P implementation owns a
new explicit binding design; C306 is not a dormant delivery seam. Human
Guidance remains the product authority.

## Blueprint Courses

### Blueprints use Private, Public, and Archived lifecycle states

**Decision.** Creation and forks start Private. Private is owner-only and
cannot be adopted. Public is visible to vetted Instructors and adoptable.
Archived is read-only, excluded from ordinary discovery and new adoption,
visible only through explicit archived inclusion, and forkable.

**Why.** Visibility, reuse, and retirement need clear author-controlled states.

**Consequence.** Only the owner changes lifecycle state. Public may return to
Private only before any adoption. Once adopted, it remains Public unless
Archived. Archived restores to Public.

### Blueprint Saves create content Revisions only when content changes

**Decision.** A Blueprint is created Private with Revision 1. Explicit Save
creates the next immutable Revision only after a meaningful canonical content
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

**Decision.** A fork's existing immutable origin records its source Blueprint Course and exact
source Blueprint Revision as provenance, not a required comparison baseline. Human Guidance now
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

Viewing and comparison follow ordinary Blueprint visibility for both fork and source: every vetted
Instructor may view Public and Archived Blueprints; explicit Archived inclusion governs discovery,
not permission to view a known Course. Private Blueprints are owner-only on either side. Ownership
controls the fork's apply mutation. The Instructor explicitly selects which displayed changes to
bring forward; no source change is applied automatically. One request may select several related
changes. The server constructs and validates one coherent complete fork tree, then uses the ordinary
expected current Blueprint Revision Tuple compare-and-swap to save all selected content changes as
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

**Decision.** Vetted Instructors may Star or Watch Public and Archived
Blueprint Courses. Stars are visible endorsements; Watch state is private and
drives notifications about Revisions and other important changes. Forking or
adopting does not automatically Star or Watch.

**Why.** Endorsement and notification choices apply to a Blueprint lineage;
adoption and forking are separate Course-creation decisions.

**Consequence.** Stars and Watches follow the Blueprint lineage across all of
its Revisions. They are not copied into a fork or daughter Course. C409 owns
Star/unstar/count and private self Watch/unwatch state plus Revision, publish,
archive, and restore Watch fan-out; it returns no Starred-by names. C856 alone
may project exact Verified Instructor Display Names, and only to an active
vetted Instructor viewing the Star list of a Public or Archived Blueprint.
That projection excludes email, UUID, Account reference, avatar, Course and
substitute identifiers, all Watch identities/state, client-side lookup, and
Profile links.

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
from stored fractions.

### Highest Attempt and point-only scoring

**Decision.** Instructors control Attempt limits and Assessment behavior.
Regular Assignment defaults support repeated work toward success. When several
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

### One stable Ribbon frame serves role-specific work

**Decision.** The shell keeps stable page geometry, separates global context
from page tasks, and preserves every required destination even when its
collection is empty. Every required Instructor destination also remains visible
when its target is incomplete, but it is presented as unavailable rather than
as a usable link. Sign Out is in the Profile menu.

**Why.** Stable geometry, honest empty states, and visible unavailable tasks
reduce cognitive load without letting incomplete features masquerade as usable.

**Consequence.** Instructor primary tabs are Courses, Questions, and
Assessments. Their exact task rows come from Human Guidance. Student work is
collectively Coursework, while a specific item uses its Assessment Type name.

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
Sysadmins may select a provided avatar or add their own Profile image. Image
delivery is self-only: Human Guidance does not settle a broader Profile-image
privacy audience.

**Why.** The same selected-or-generic avatar must represent each user
consistently, while the allowed choice differs by Product Role. A single
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
[Human Guidance implementation compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md)
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
discoverable in real Profile Settings. Staff cross-Account Profile-image
delivery remains an explicit Human Guidance product question; the default is
self-only delivery.

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
compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md)
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
[Human Guidance implementation compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md)
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
verified Instructor destination on every claim rather than storing an address
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
resends. `NotConfigured` records a non-send failure and never reports fake
success. A late run attempts required earlier notices in due-time order, then
performs the due transition after successfully recorded failure; it stops only
the notice lane for an unknown typed Store state and always continues retention
transitions. Provider credentials are operational configuration; the Live Demo
remains `NotConfigured` and is not delivery evidence.

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

### Question Pools are current state

**Decision.** Pools are not a Revision family. Members live in
`question_pool_member`. Saves compare-and-swap the Pool Edit Number.

**Why.** Human Guidance treats Pool membership as current authored state,
not an immutable Revision history.

**Consequence.** Student Work pins Question ID, Question Revision, Pool ID,
and Pool Edit Number. Forks copy current members once.

### Library usage statistics are retained counters, not reconstructions

**Decision.** Each Published Question Revision stores `issued_count`,
`blank_count`, `answered_count`, `correct_count` (full credit),
`partial_count`, `incorrect_count` (zero credit), `credit_sum`, and
`credit_sum_sq`. Mean and standard deviation of credit derive from the
sums. A Question Pool stores `issued_count` and a per-member
`selected_count`. Difficulty is computed from current members' Question
statistics when read. Storage is per Revision; bulk Library views show
the rollup across Revisions, and the Question detail page adds the
per-Revision breakdown.

**Why.** Human Guidance asks for privacy-safe counts so Instructors can
judge how often a Question is used and how hard it is. Rebuilding
aggregates from Student Work receipts would shrink historical totals when
Attempts are Unreleased or FERPA-purged. A cached Pool difficulty column
would need membership-triggered rewrites.

**Consequence.** Counters increment at Assessment Attempt submission, so
an Unreleased or deleted in-progress Attempt contributes nothing.
`issued_count = blank_count + answered_count` and
`answered_count = correct_count + partial_count + incorrect_count`. A
blank Question is submitted with no saved response and is never sent to a
Question Backend. Every Attempt counts as one observation, including
practice Attempts after full credit. The increment receipt is Student
Work and is purged with the Attempt; the aggregate survives and is never
rebuilt from Student Work. The statistic records Revision, outcome class,
credit fraction, and `updated_on` only. A member's `selected_count` is
removed with the member and starts at zero if the member is re-added.
Member outcomes count in that Question Revision's own statistic, never in
a Pool-level copy. Instructors see each rate beside its observation
count. Students see Course-scoped class statistics through the Assessment
feedback policy; those projections are Student Work and are purged with
the Course. [FERPA_DATA_POLICY.md](FERPA_DATA_POLICY.md) lists the
collected facts.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) "Question Library object
usage statistics"; schema in `schemas/base_schema/20_tables/statistics.sql`.

### Creation clocks are `timestamptz` for enforcement and `date` for authored content

**Decision.** Student Work, sessions, events, Courses, and Accounts use a
full-precision `timestamptz` creation clock. Published Questions, Question
Pools, Blueprint Courses, their Revisions, Draft Questions, and usage
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
role names. SVG remains unsupported Question ingest. Non-image Question media
is not a current product type.

**Owner.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md),
[OBJECT_STORAGE.md](OBJECT_STORAGE.md).

## Unresolved decisions

The complete Student Ribbon task layout and the complete Sysadmin Ribbon task
layout do not have locked-in designs yet. Product documentation should not turn
implementation choices, hypothetical capabilities, tunable FERPA retention
intervals, or speculative failure machinery into additional unresolved product
questions.

See the temporary
[COMPLIANCE_SUMMARY.md](active_plans/reports/human_guidance_compliance/COMPLIANCE_SUMMARY.md)
for the corpus review.
