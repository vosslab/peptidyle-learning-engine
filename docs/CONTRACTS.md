# Contract register

This register names the target product boundaries of the Peptidyle Learning
Engine. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product authority and
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines the terms used here.
The linked base-schema and source paths are implementation evidence, not product
authority. Generic teaching objects are Assessments. Assignment appears only in
the three Assessment Type names. Blueprint lifecycle states are Private, Public,
and Archived.

## Contract rule

A change to a published contract updates its owning implementation, direct
consumers, generated types or fixtures, relevant tests, this register, and the
changelog together. A browser decoder receives `unknown`, accepts only its
closed shape, and does not supply authority. Database functions derive account,
membership, and record scope from the authenticated session and their typed
arguments.

A public ID is the one universal, canonical human-facing identifier for a PLE
object that needs one. Store and use the exact same ID in the database, Rust,
JSON, URLs, object storage, hashes, logs, and browser UI.
Preserve the canonical ID exactly across system boundaries. Parsing,
serialization, API transport, persistence, and display do not add, remove,
reformat, or translate characters.
Public IDs include their embedded checksum character, and entry validation
checks it against every other uppercase canonical-ID character, including any
prefix and excluding only separators and the checksum position, without changing
the canonical value. The checksum is the high five bits of public unsalted
SHA-256 digest byte 0, mapped through the Crockford alphabet.
Public IDs use the Crockford Base32 alphabet and one canonical uppercase ASCII
form. Human entry may normalize lowercase Crockford characters, `O` or `o` to
`0`, and `I`, `i`, `L`, or `l` to `1` before canonical syntax and checksum
validation; Question-ID entry may also restore its canonical hyphen. `X` in a
format denotes one cryptographically random Crockford Base32 character; `Z`
denotes the stored, calculated checksum character. Generation enforces global uniqueness across
every public-ID type, retries random collisions, and never reuses an issued ID,
including after deletion or archival. Published Questions and Question Pools
share one global `XXXX-ZXXX` namespace: a value identifies either object,
never both.
Blueprint Course `BPXXXXXXXZ`, Course Instance `CIXXXXXXXZ`, Assessment `AXXXXXXXZ`,
and Account `UXXXXXXXZ` references use seven random Crockford Base32 characters
plus embedded checksum `Z`; `Z` is a calculated placeholder, not a literal.
Internal, non-user-facing objects use native UUID identifiers; a public ID
exists only for a human-facing workflow that needs it.

The application has one installation and global Accounts. There is no
institution tenancy boundary. Course, Assessment, Student Work, worker, and
object operations each authorize their exact parent relationship rather than a
caller-selected role or scope.

### DD-A9-01: direct preproduction Assessment cutover

Before the first production deployment, every generic `assignment` identifier
changes directly to Assessment in the schema, model, Store/server contracts,
routes, DTOs, API, and Blueprint JSON. This preserves public `AXXXXXXXZ` IDs,
UUID values, and the five Assessment Type enum values; Assignment remains only
in the three Type display names. The canonical JSON names are `assessment`,
`id` or nested `assessmentId` / `courseId` for those public IDs,
`assessmentAttempt`, `assessmentEntry`,
`assessmentStatus`, `assessments`, and `blueprint_assessment_id`.
Canonical decoders accept only those names. There is no parallel `reference`
property.

This is one preproduction rebuild, not a compatibility migration. It permits
no SQL view, alias, dual DTO, dual decoder, dual import, dual API, or
mixed-nomenclature reader. A fresh installation is canonical; an offline
developer export-transform-import may assist that resettable rebuild, but
there is no runtime legacy Blueprint import. A failed rollout restores the
previous code and matching resettable preproduction database together.

There is no legacy browser redirect or API compatibility layer. The
preproduction rebuild uses canonical Assessment paths and APIs directly.

Account Settings is a self-owned, all-Product-Role exact IANA display
preference. `GET` / `PUT /api/account/settings` has the closed
`{ "timeZone": "exact IANA name" }` shape and accepts no Account, Course, or
Product Role selector. PostgreSQL derives the active Account from the
authenticated session and atomically reads or updates that Account's
preference. Roster import marks only a newly created Student Account for a
one-time default; invitation acceptance copies the inviting Instructor's zone
unless the Student already chose a zone. Existing Accounts keep their
preference. A change updates display and an Instructor's later wall-clock date
entry interpretation; it never moves an absolute stored deadline.

Profile Settings owns avatar selection and Profile-image work. Instructor
Profile displays the current Account time zone and links to Account Settings;
it does not edit the preference. Account Settings exposes no passkey, email,
TOTP, recovery, Account-status, or session control. Required Student and
Instructor passwordless authentication and multiple Student passkeys remain
Accounts-and-roles work. Only self-service enumeration, revocation,
re-authentication, identity-proofed recovery, notification, and
session-termination semantics remain unresolved pending a separate decision.
C15 remains the separate Sysadmin TOTP session contract, with no self-service
TOTP management or recovery.

## Accounts and profiles

PLE-provided avatars are a versioned first-party static catalog. The canonical
source-controlled `assets/avatar_catalog/` holds the manifest, original SVGs,
and `PROVENANCE`; a deterministic generator derives the Rust registry,
TypeScript catalog, and SQL seed. Catalog IDs are stable ASCII values that are
never renamed or reused. Each has a `selectable` flag: a retired avatar remains
renderable but is no longer selectable, and `provided_avatar.is_selectable`
enforces that boundary.

The catalog admits only a safe, bounded SVG grammar and contains original
abstract toy-brick, color, and pattern art licensed CC-BY-4.0, with no LEGO
marks or copied minifigure art. The browser receives static same-origin
fingerprinted app assets, not a catalog-list API. The self-choice API is
self-only; the server and database authorize the generated seed. An unknown
stored ID renders the safe generic avatar and starts catalog-drift repair,
never a selectable fallback.

The generated registry, TypeScript catalog, SQL seed, and fingerprinted assets
are one deploy and rollback unit. A fresh install must accept that unit.
Temporary generator, schema, unknown-ID, and fresh-install checks prove the
rebuild and are removed at plan closeout unless a check earns permanent status
under `PYTEST_STYLE.md`. A retained retired-ID renderable/not-selectable test
protects the durable catalog contract; its failure means repair the generator,
seed, or authorization boundary before closure.

Staff cross-Account Profile-image delivery is an explicit unresolved product
question. Self-only delivery is the default. C40 owns the provided-avatar
catalog and reusable picker contributor. C819 owns role-neutral Profile
Settings authorization; C820 owns the real `/profile` route and page
integration. The catalog and picker may be completed before those routes, but
no C40 Human Guidance bullet closes until C819 and C820 make Student selection
discoverable in real Profile Settings.

| Boundary | Owner and evidence |
| --- | --- |
| First-party provided-avatar catalog | C40, with C819/C820 route dependencies, in the active [Human Guidance implementation compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md); generated registry, TypeScript catalog, SQL seed, and fresh-install verification |

## Authentication and sessions

Primary authentication creates an opaque pending-MFA state when the
database-derived Product Role is Sysadmin; it creates no authenticated session.
PostgreSQL alone derives that role and atomically requires and consumes one
unused, short-lived, Account- and browser-bound TOTP attestation while creating
the Sysadmin session. The one-use transition rolls back with session creation:
a failed transition creates no session, and a consumed attestation cannot be
used again. Student and Instructor session behavior remains unchanged.

The server validates the 30-second TOTP counter, rejects replay, rate-limits
attempts, and never logs a TOTP value or seed. The TOTP seed is encrypted at
rest under a wrapping key. These requirements apply OWASP ASVS 2.1.1,
2.2.1--2.2.3, 2.3.1 and 2.3.3, 6.1.3, 6.3.1 and 6.3.4, 6.4.3 and 6.4.4,
6.5.1, 6.5.3, 6.5.5, and 6.5.8, 7.2.1--7.2.4, 7.4.1 and 7.4.3, and
7.5.1. No recovery or self-service TOTP management contract exists.

The browser receives only the pending-MFA and completion boundary; it never
receives a seed or a fixed demonstration secret. The local controller may
provision a genuine operating-system-CSPRNG seed and write a restricted,
ignored, mode-0600 operator artifact for the Live Demo, logging only its path.
A separate local authenticator consumes that artifact. It is not a browser
credential and does not bypass MFA.

| Boundary | Owner and evidence |
| --- | --- |
| Sysadmin TOTP session transition | C15 in the active [Human Guidance implementation compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md); future authentication Store, server, and base-schema boundaries |
| Account Settings time zone | C819-C823 in the active [Human Guidance implementation compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md); closed browser decoder, server validation, and one self-derived PostgreSQL transaction |

## Reusable content

| Boundary                       | Current contract                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Owner and evidence                                                                                                                                                                                                      |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Question lineage and revision  | A Published Question is one stable lineage. A `PublishedQuestionRevisionTuple` is its `PublishedQuestionId` plus immutable positive revision number. Publication creates complete immutable Question Revision facts, including source, provenance, license, and required metadata. A later publication appends a revision; it does not alter an earlier one. A fork publication derives and preserves the exact source Revision license rather than accepting a replacement. | [question_library.rs](../crates/question_model/src/question_library.rs), [question_publication.rs](../crates/server/src/question_publication.rs), [question_lineages.sql](../schemas/base_schema/50_functions/question_lineages.sql), [question_publication_operations.sql](../schemas/base_schema/50_functions/question_publication_operations.sql) |
| Question discovery             | A Published Question is discoverable and selectable by vetted Instructors. Archive hides it from ordinary discovery and new selection while authorized exact Revision Tuples continue to resolve. Current implementation may call the discoverable state `Available`; that name is not a separate product object or Revision state. | [question_library.rs](../crates/question_model/src/question_library.rs), [question_lineages.sql](../schemas/base_schema/50_functions/question_lineages.sql) |
| Question ID                    | `XXXX-ZXXX` is the canonical Question ID value at every persisted and transmitted boundary. Its hyphen makes the value immediately recognizable as a Question ID. Human entry may omit that hyphen and use the closed Crockford aliases before canonicalization, syntax validation, checksum validation, and lookup. Seven identity characters come from a cryptographically secure server source and the first character after the hyphen is the embedded public SHA-256 checksum character. Global public-ID registry reservation is the identity boundary. | [question_library.rs](../crates/question_model/src/question_library.rs), [question_publication.rs](../crates/server/src/question_publication.rs), [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md)                            |
| Question Pool                  | A reusable Published Pool has one server-minted public ID. Membership is current state on an Edit Number, not a Pool Revision family. A change compare-and-swaps that Edit Number. Pool membership is backend neutral. Each Assessment-owned Pool entry/fork has one positive `selection_count` and copies current members once. Student Work pins Question ID, Question Revision, Pool ID, and Pool Edit Number. | [question_pools.sql](../schemas/base_schema/50_functions/question_pools.sql), [assessments.sql](../schemas/base_schema/50_functions/assessments.sql), [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) |
| Bulk Published Question metadata | An active vetted Instructor may atomically replace only the current shared `tags`, `subject`, and `topic` for a bounded nonempty distinct set of canonical Published Question IDs. Every selected ID supplies its exact metadata Edit Number; stale, invalid, unauthorized, unavailable, duplicate, or oversized selection changes none and produces only a whole outcome. First publication seeds tags once from validated native `PLE authoring`/`Pilot` source tags or an empty WebWork list; thereafter tags are database-owned current metadata, and a successor preserves an intentional clear. Source, answer, grading, feedback, assets, backend, Question Type, authorship, ownership, availability, and immutable Question Revisions are excluded. | C365-C368 and C893 in the active [Human Guidance implementation compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md) |
| Discipline lifecycle | A Discipline has one stable UUID. Sysadmins create, rename, retire, and restore it; there is no delete transition. Retirement removes it from new choices while existing exact references remain visible and discoverable with retired status. Copy and inheritance may retain an exact referenced retired Discipline, while new use requires an active row. Shared row locks serialize retirement against concurrent new use. | [content_classification_operations.sql](../schemas/base_schema/50_functions/content_classification_operations.sql), [content_classification.rs](../crates/server/src/content_classification.rs), [content_disciplines_page.tsx](../src/pages/content_disciplines_page.tsx) |
| Library improvement activity | Active vetted Instructors may create and reply to retained text-only improvement threads on an available Question or Pool and edit only their own posts. Question owners and Sysadmins may resolve or reopen Question threads and manage Question impact notices; Pool administration is Sysadmin-only. Records remain after resolution or cancellation. Sysadmins read Library content without ordinary Instructor mutation controls and use the same retained activity view for administration. This source boundary is distinct from private Watch delivery, whose four-event acceptance remains open. | [library_discussions.sql](../schemas/base_schema/60_policies/library_discussions.sql), [library_discussion_operations.sql](../schemas/base_schema/50_functions/library_discussion_operations.sql), [library_discussion.rs](../crates/server/src/library_discussion.rs), [library_discussion_panel.tsx](../src/components/library_discussion_panel.tsx) |
| BiologyProblems.org algorithmic catalog | Each BiologyProblems.org WeBWorK family records one canonical algorithmic author source (official PG/PGML or generator), produces exactly one canonical algorithmic PG/PGML file and ordinary Published Question lineage, and replaces its generated static variants. Algorithmic Questions ordinarily stand alone, but an Instructor may deliberately Pool distinct similar algorithms when selection is useful; Pool selection and backend-native variation remain independent. Only after per-family source acceptance, representative deterministic render/grade proof, and expected-current Blueprint Revision CAS may current placements change. A later Pool membership edit removes redundant generated variants while intentional distinct-algorithm Pools remain. Replaced static Question lineages, redundant Pool lineages, and generated source copies retire only through this forward path. Exact immutable Question Revisions, Blueprint pins, Pool IDs, Pool Edit Numbers, and Student Work remain resolvable; no history is deleted, repurposed, or raw-SQL-rewritten. The shipped 119 static banks remain unmigrated; the manifest's 13 algorithmic-source definitions, including HLA, are migration inputs, not runtime proof. | C824-C841 in the active [Human Guidance implementation compliance plan](active_plans/active/human_guidance_implementation_compliance_plan.md); future bundled content manifest, Question publication, Pool, and Blueprint publication boundaries |
| Blueprint lineage and revision | A Blueprint Course is a stable reusable lineage created Private with Revision 1. A `BlueprintRevisionTuple` names one exact immutable Revision; a changed explicit Save based on its current Revision creates the next one, while a canonical no-op returns the current Revision with `changed: false`. Blueprint Assessment provenance retains that exact Revision and the stable Blueprint Assessment ID when Course state derives from reusable content. | [contracts.rs](../crates/question_model/src/blueprint_operations/contracts.rs), [blueprints.sql](../schemas/base_schema/50_functions/blueprints.sql)                                                                                 |

Blueprint Courses use Private, Public, and Archived lifecycle states. Private is
owner-only and cannot be adopted. Public is shared and adoptable; only a Public
Blueprint with no adoptions may return to Private. Archived remains visible to
vetted Instructors through explicit historical discovery, can be forked, and
cannot be adopted. The owner can restore it to Public. Short name, long name,
and lifecycle state are current lineage metadata and never create a Revision.

Adoption creates independent Course Instance Assessment copies. The retained
Blueprint relationship makes newer Blueprint Revisions visible for daughter
Course Instructor review and approval. Existing Assessment changes are never
silently applied. When a Blueprint Assessment is newly added, PLE automatically
creates an Unreleased copy in each daughter Course Instance.

For one retained daughter Assessment, `GET` and `POST`
`/api/course-instances/{course_instance_id}/assessments/{assessment_id}/blueprint-update`
derive a review from the current parent Blueprint Revision Tuple and explicitly
apply its reusable content. The read exposes the current and proposed ordered
Fixed Question Revision pins together with sibling Pool ID and Pool Edit Number
fields; Apply accepts only the expected parent Blueprint Revision Tuple and
daughter Assessment Edit Number. The trusted Store reauthorizes and locks
parent, Course, and Assessment, compares reusable semantics before minting
fresh owned Pool forks, and preserves Course dates, release status, origin
pins, and existing Student Work.
No offer, approval receipt, comparison baseline, or update table is persisted.
This contributor does not provide whole-Course discovery, review, or correspondence.

A fork retains its source Blueprint and Revision so later source changes can be
discovered and selectively brought into the fork. A Blueprint Course Change
Proposal presents canonical JSON differences to the receiving owner; accepted
changes create a new receiving Blueprint Revision and reach daughters only
through the normal update workflow.

## Teaching current state

| Boundary                       | Current contract                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Owner and evidence                                                                                                                                                                                                                    |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Course Instance                | A Course Instance starts empty or adopts an exact Public Blueprint Revision, atomically creating every Assessment with its Questions, pools, and reusable settings, fresh identities, Unreleased state, and unset dates. It owns deliberately entered names, Course Term dates, equal co-Instructor memberships, roster, and delivery state; it always has at least one assigned Instructor. Course Term is current Course state, not a Revision. | [course_term.rs](../crates/question_model/src/course_term.rs), [course_core.sql](../schemas/base_schema/50_functions/course_core.sql), [course_operations.sql](../schemas/base_schema/50_functions/course_operations.sql) |
| Assessment                     | An Assessment is one stable current aggregate under a Course Instance. Its positive Assessment Edit Number is the strong HTTP `If-Match` encoding for save, Assessment Properties save, release, and Unrelease. It owns current title, instructions, dates, settings, ordered Questions, and Question Pools. Every Question and Pool item pins an exact Question Revision; a newer Question Revision never advances it. | [edit_number.rs](../crates/question_model/src/assessment/edit_number.rs), [assessment.rs](../crates/question_model/src/assessment.rs), [assessments.sql](../schemas/base_schema/50_functions/assessments.sql) |
| Assessment release and editing | Release is a current state transition, returning the current Assessment and new quoted Assessment Edit Number. A released Assessment save uses the same Assessment Edit Number contract and is accepted only when the resulting current state passes release validation. Accepted edits affect future Attempts; prior Student Work keeps its retained facts. | [assessment_release.rs](../crates/server/src/assessment_release.rs), [assessment_release.rs](../crates/learning-data-access/src/assessment_release.rs), [assessment_operations.sql](../schemas/base_schema/50_functions/assessment_operations.sql) |
| Course Blueprint update review | An authorized Course Instructor may lazily derive one current-parent Course summary for adopted Assessments only. It classifies changed, matching, removed-source, Type-mismatch, and automatically-added correspondences; direct local Assessments are excluded. Each changed adopted Assessment uses the existing explicit detail Apply with parent Blueprint Revision Tuple and Assessment Edit Number CAS. It preserves local dates, status, origin, and existing Student Work; equivalent content is a no-op and changed Pools receive fresh owned forks. It has no persisted offers, receipts, baselines, or stored update state, and does not close a whole-Course lifecycle. | [assessment_blueprint_update.rs](../crates/learning-data-access/src/postgres/assessment_blueprint_update.rs), [assessment_blueprint_updates.sql](../schemas/base_schema/50_functions/assessment_blueprint_updates.sql), [assessment_release.rs](../crates/server/src/assessment_release.rs), [course_blueprint_update_review.tsx](../src/pages/course_blueprint_update_review.tsx) |
| Assessment Unrelease           | An authorized Teaching Team member supplies the exact Assessment Edit Number and title confirmation. The database locks the Assessment, confirms Released status, changes it to Unreleased, and permanently deletes its Student Work. Shared Questions, current Assessment state, and Course membership survive. | [assessment_release.rs](../crates/server/src/assessment_release.rs), [unrelease.sql](../schemas/base_schema/50_functions/unrelease.sql) |

**Create Blueprint from Course Instance** creates a distinct actor-owned Private
Blueprint Course at Revision 1 from an existing Course Instance's reusable
structure. It records immutable source provenance and that unchanged source
Course Instance as the new Blueprint's first Adoption. The new Blueprint does
not carry Students, Course dates, releases, Student Work, or other delivery
state, and the source Course Instance remains the same addressable teaching
Course.

`412 Precondition Failed` represents a stale qualified Edit Number or
Revision Number, `422 Unprocessable Entity` represents invalid resulting
content or confirmation, and `409 Conflict` represents an invalid
lifecycle state. Authorization and non-resolvable targets use the
repository's non-enumerating response policy.

### Scoped support authority

Only a current active Instructor membership for the exact Course may issue a
time-scoped Student-roster repair capability to a named Sysadmin. The capability records its
issuer, recipient, exact Course/Student roster scope, purpose, expiry, revocation, and each use;
use rechecks that authority and never creates Instructor membership or a broader record grant.
This implemented Student-roster boundary does not close the separate open Course or content support
requirements.

## Student Work and assessment evidence

An Assessment Attempt is one independent Student Work occurrence. At start it
retains the effective Assessment title, instructions, settings, feedback rule,
and qualified sources for any accommodation-adjusted schedule or limits.
Resume, response interpretation, history, disclosure, and statistics read that
retained evidence with issued-work evidence; a later Assessment save does not
replace it. Score calculation is the explicit exception: it combines stored
credit fractions with current Assessment Question point values.

Each Issued Question retains its Assessment position, exact
Question Revision, point and scoring facts, statistics
eligibility, and pool-selection source. Attempt-position presentation and
backend-evidence records retain the exact backend and asset bindings used for that
work. Reproduction is explicitly tagged `Static` or `Seeded`. Native
`pleQuestionJson` is `Static`: it retains no `QuestionSeed` or
generated-parameter hash. Its PLE-server-generated presentation nonce binds
only presentation-scoped response-item references and authored choice ordering;
it is neither a source nor author-JavaScript seed. `Seeded` backends retain
their backend-owned seed and generated-parameter hash only when used. The
descriptor evidence checksum v4 binds the tag and applicable facts: a static
descriptor rejects a seed/hash, while a seeded descriptor binds them. Current
production delivery maps native PLE JSON to `Static` and WeBWorK to `Seeded`.
iMathAS and H5P are desired but deferred Backends and are not current
implementation requirements. H5P has no delivered binding or retained binding
seam: C870 removed the placeholder. Any later Backend moved out of Human
Guidance's Deferred section must define its reproduction contract before
implementation. Private normalized response-item bindings map
each presentation-scoped four-hex reference to the durable authored
response-item identity, so evidence readers interpret saved work without a
mutable source lookup. Saved responses are private mutable input while the
Attempt is active.

An author may name only a closed server registry library identifier, currently
`rdkit`; an author never supplies a URL, CDN domain, local path, package
version, or asset digest. The server resolves the current reviewed RDKit.js
JS/WASM pair and derives its exact unversioned local runtime paths. The isolated
document has no general network capability: its only dependency fetch is the
current registry-selected RDKit WASM path required by the reviewed JS bootstrap.
Its opaque sandbox CSP permits `'unsafe-eval'` only for that current official
RDKit loader; this does not broaden the main PLE CSP or add a network source.
The closed `libraries: []` branch has no runtime tags, `connect-src 'none'`,
nonce-only `script-src`, and no `'unsafe-eval'`.

The answer-free descriptor retains only source and closed library IDs. It does
not bind a package version, asset digest, cache key, or historical dependency
identity into reproduction evidence. C901 supplies the current local runtime;
C902 refreshes it under the repository's latest-dependency policy. This is
never authorization for an author-selected version or path.
At start, a timed Attempt retains one immutable expiry instant: the earlier of
its retained close instant and start plus retained time limit. The Student's
whole-Attempt action and server-owned expiry finalization use one ordinary
submission path. It finalizes all complete saved Question responses together,
leaves other Questions visibly unanswered, and stores one immutable credit
fraction for each complete saved response evaluated by its Question Backend.
Each unanswered Question contributes zero credit and counts as incorrect
without being sent to a backend. The retained per-position evidence does not
define another Student action or lifecycle.
Repeated finalization is idempotent, and a late payload cannot replace accepted
or saved work.

Score reads calculate each Attempt from current Assessment Question point
values and use the highest submitted Assessment Attempt score. PLE has no
separate Question weights, Grade Categories, Course Grade Scheme, or Course
percentage calculation. Pilot grade export is CSV or TSV point data. The current Course
co-Instructor downloads the same authorized Gradebook projection through the fixed seven-column
attachment contract in [API_CONTRACTS.md](API_CONTRACTS.md#gradebook-point-export). Exports preserve
missing versus zero scores, protect spreadsheet text cells, and leave Course weighting to the home
LMS.

| Boundary                                             | Owner and evidence                                                                                                                                                                                                                                                                |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Attempt access, start, resume, and retained evidence | [attempt_evidence.rs](../crates/question_model/src/student_work/attempt_evidence.rs), [assessment_delivery.rs](../crates/server/src/assessment_delivery.rs), [assessment_attempts.sql](../schemas/base_schema/50_functions/assessment_attempts.sql), [assessment_attempt_access.sql](../schemas/base_schema/50_functions/assessment_attempt_access.sql) |
| Issued Questions, interaction, and presentation      | [assessment_attempt.rs](../crates/learning-data-access/src/assessment_attempt.rs), [assessment_attempt_interaction.sql](../schemas/base_schema/50_functions/assessment_attempt_interaction.sql), [assessment_attempt_presentation.sql](../schemas/base_schema/50_functions/assessment_attempt_presentation.sql)                                         |
| Submission, grading, history, and statistics         | [submission.rs](../crates/server/src/assessment_delivery/submission.rs), [grading.sql](../schemas/base_schema/50_functions/grading.sql), [assessment_attempt_history.sql](../schemas/base_schema/50_functions/assessment_attempt_history.sql), [statistics.sql](../schemas/base_schema/20_tables/statistics.sql)                            |

The tagged-reproduction cutover changes the preproduction base schema and its
model, Store/server, and Student Work readers together, then reinitializes a
fresh database. No seed sentinel, null ambiguity, compatibility reader, or
compatibility shim is allowed. The server builds the tagged descriptor before
one database transaction validates and writes the Question Attempt,
presentation binding, and response-item evidence; any later failure rolls the
transaction back without partial Student Work. One permanent public native
no-seed contract test may be promoted only after every
[PYTEST_STYLE.md](PYTEST_STYLE.md) permanent-test checklist item is yes;
otherwise it remains temporary and is removed. The vertical database matrix is
always temporary: it covers tagged acceptance and native seed/hash rejection,
then is removed. If this coordinated cutover fails before the first production
baseline, rollback restores the previous pre-production code and base schema
together and reinitializes the disposable database; no mixed code/schema
deployment is valid.

Before a new Attempt starts, Assessment access and the Student Course landing
must apply the same decision. The current Rust-owned
`StudentAssignmentDecisionSummary` is implementation evidence. PostgreSQL
evaluates each read once, applies only the current Student's effective policy,
and returns UTC-millisecond instants plus that Student's IANA display zone. The
browser may format those values but cannot grant access or identify the
accommodation that produced an effective value.

Student operations use the server-owned expiry instant before allowing further
changes. Reads remain reads: context, Gradebook, result-page, and export reads
do not finalize Student Work or call a Question Backend. Context returns the
exact expiry instant and Student display zone plus a remaining duration derived
from the same row and evaluation instant. The browser's monotonic countdown is
display-only. Student interaction checks expiration, and background processing
ensures an expired Attempt is submitted even after the Student leaves.

The Student presentation and submission routes expose answer-free,
presentation-scoped state. Grading, source bytes, answer keys, private feedback
internals, and worker lease material stay on their server and capability
boundaries.

## Jobs, objects, and database lifecycle

| Boundary           | Current contract                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Owner and evidence                                                                                                                                                                                          |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Background work    | Public-asset publication may use an internal Job. A background process also ensures expired Assessment Attempts are submitted. Neither creates a Student or Instructor grading workflow, attention state, retry control, or polling UI. Human Guidance does not establish backend-grading polling as a product contract. | [jobs.sql](../schemas/base_schema/20_tables/jobs.sql), [grading.sql](../schemas/base_schema/50_functions/grading.sql) |
| Objects and assets | Object records are typed by their owning Question, workspace, Course, or Student Work relationship. Public-asset publication and delivery retain their actual object ownership; a caller cannot provide storage scope or widen an object reference.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | [object_records.sql](../schemas/base_schema/50_functions/object_records.sql), [question_images.sql](../schemas/base_schema/20_tables/question_images.sql), [delivery.sql](../schemas/base_schema/50_functions/delivery.sql)                     |
| Base installation  | [install.sql](../schemas/base_schema/install.sql) is a deliberately small ordered `psql` manifest. Its domain modules directly own the current structural DDL, functions, RLS policies, grants, and constraints. Before the first human-approved production deployment, structural corrections modify their owning base module. At that cutover the SQL projection and Rust `BASE_RELEASE_IDENTITY` change together from `pre-production` to one immutable production-baseline identifier recorded in [CHANGELOG.md](CHANGELOG.md). The base then stays frozen and `schemas/migrations/` contains forward-only SQLx migrations.                                                                                                                                                                                                                                                                                                                                                             | [install.sql](../schemas/base_schema/install.sql), [database_coordinator.rs](../crates/project-tools/src/database_coordinator.rs), [sqlx.toml](../crates/learning-data-access/sqlx.toml)                    |
| Administration     | `cargo tools database initialize`, `migrate`, and `verify` are the bounded administration path. SQLx records forward migrations in `ple_migration._sqlx_migrations`; runtime roles have neither DDL authority nor write access to that ledger. `ple_api.ple_schema_state` is the restricted application-visible projection of the current base release identity and SQLx ledger. It reports `pre-production` today, then the immutable production-baseline identifier established at the first human-approved cutover.                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | [database.rs](../crates/project-tools/src/database.rs), [database_coordinator.rs](../crates/project-tools/src/database_coordinator.rs), [foundation_roles.sql](../schemas/base_schema/70_grants/foundation_roles.sql) |
| Installation data  | A fresh installation uses the same production schema and publishes the shipped Genetics Blueprint Course. It includes the complete Live Demo by default. `provision --without-live-demo` omits the optional Live Demo Accounts, Course, and activity while retaining Genetics. The resulting records follow ordinary product lifecycle and deletion rules. | [installation_data.rs](../crates/project-tools/src/installation_data.rs), [install.sql](../schemas/installation_data/install.sql), [README.md](../schemas/installation_data/README.md) |

There is no separate demo schema, demo role, marker, teardown capability, or
alternate product representation.

### Course Banner source, promotion, and delivery

After EXIF orientation, an accepted Course Banner is a complete, positive still
PNG, JPEG, or WebP for which `u64(width) == 5 * u64(height)`; incomplete,
trailing, and polyglot inputs are rejected. The server and `ple_api`
upload-staging boundary both enforce that oriented exact 5:1 rule. The existing
8 MiB source-byte limit and 20-million-pixel limit apply. There is no minimum
source dimension; 1280 by 256 is guidance only. Valid sources therefore
include 5 by 1, smaller 5:1 sources, and 2560 by 512 sources.

The server retains the immutable source and atomically promotes exactly one
lossless-WebP semantic `Banner` rendition: 1280 by 256 and at most 2 MiB.
Every smaller valid source is upscaled and every larger valid source is
downscaled with aspect-preserving scaling, no crop, and no pad. `Hero` and
`Card` have no direct preproduction model, object, schema, Store, server, or
client contract. Each promotion mints a fresh opaque `CourseBannerId`;
the private rendition object identity is deterministically derived from the
Course, that reference, and the sole `Banner` discriminator. The browser
receives it only through one same-origin delivery route with `no-store`; it
receives no storage key and cannot select a rendition.

This is a whole preproduction code-and-schema cutover. If it fails, restore the
previous code and base schema together and rebuild disposable fixtures. No
alias, compatibility reader, mixed representation, or partial promotion is
valid. C813 owns layout around the one delivered Banner representation.

Use temporary proof for ratio acceptance, no-crop/no-pad promotion, and
all-or-nothing promotion, then remove it. A permanent behavior oracle is
permitted only if every [PYTEST_STYLE.md](PYTEST_STYLE.md) criterion is yes and
its failure is release-blocking because it detects a ratio, crop, or
partial-promotion regression. It must not constrain the exact 1280 dimensions,
source dimensions, private enum shape, saga slots, or filter implementation.

| Boundary | Owner |
| --- | --- |
| Course Banner contract | [course_appearance.rs](../crates/question_model/src/course_appearance.rs) and this contract |

## Explicitly absent contracts

The following retired structures have no current contract, compatibility reader,
endpoint, decoder, or reserved implementation slot:

- Assessment Revision and its entry, pool, release-snapshot, and successor
  records.
- Course Schedule Revision.
- Question Change Proposal Revision.
- Course Retention Plan Revision. Retention follows the current Course policy
  without adding a Revision family; exact job targets are implementation detail.
- Blueprint collaborators and collaborator-close Revision Save behavior.
- Legacy read fallbacks from Student Work to mutable Assessment configuration.
- Demo-only provisioning state, receipts, schema concepts, and report records.

Future capability work introduces a complete vertical boundary-authorization,
Store, Server, schema, browser workflow, and tests-when it has an approved
product decision. It does not reserve a schema scaffold in advance.

## Shared artifact ownership

| Artifact                                   | Owning module                      |
| ------------------------------------------ | ---------------------------------- |
| `schemas/base_schema/`                     | PostgreSQL base-schema owner       |
| `schemas/migrations/`                      | PostgreSQL forward-migration owner |
| `schemas/installation_data/`               | installation-data owner            |
| `generated/api/`                           | Rust model and `cargo tools tsgen` |
| `containers/compose.yaml`                  | local-stack deployment owner       |
| `tests/e2e/compose.live-demo-browser.yaml` | local-stack deployment owner       |

Generated TypeScript is derivative of its Rust model. Fixtures are test input,
not a runtime API or another contract owner.

## Boundary invariants

- Rust and PostgreSQL own domain truth; browser code decodes and renders the
  authorized, answer-free projection it receives.
- Question and Blueprint provenance always uses an exact Revision
  Tuple. Current Course and Assessment configuration uses current state
  with its qualified Edit Number where concurrent writers need one.
- Student Work is independently interpretable from its retained evidence.
- Lists are bounded and cursor-based. Browser transport is same-origin and
  successful bodies are `no-store` where they contain protected current state.
- Database routines use fixed search paths, narrow privileges, forced RLS where
  appropriate, and transactions for cross-record lifecycle changes.
