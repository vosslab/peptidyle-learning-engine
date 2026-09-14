# Database structure

This document describes the canonical PostgreSQL structure for a fresh PLE
installation. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) remains the product
authority, [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines the
terms used here, and [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md)
describes the authorization model in more detail. This document explains how
the checked-in database expresses that model; it is not a migration catalogue.

## One canonical structural build

[schemas/base_schema/install.sql](../schemas/base_schema/install.sql) is a
small, ordered `psql` manifest. It includes named domain-family modules in
dependency order. The database administration command runs it with PostgreSQL
17 `psql -X --set=ON_ERROR_STOP=1 --single-transaction`, so a failed fresh
install leaves no partial PLE structure.

Each module owns the current DDL for its domain: tables, constraints, indexes,
functions, row-level-security policies, and grants. The manifest itself has no
domain DDL. A cross-domain relationship belongs in
[cross_domain_constraints.sql](../schemas/base_schema/cross_domain_constraints.sql),
and the restricted runtime projection belongs last in
[api_compatibility.sql](../schemas/base_schema/api_compatibility.sql). This
keeps the base readable without creating a sequence of corrective layers.

Before the first human-approved production deployment, the base schema is
editable source. A structural correction changes its owning module and is
verified with a clean database build. At that cutover, the checked-in SQL
projection and Rust `BASE_RELEASE_IDENTITY` change together from
`pre-production` to one immutable production-baseline identifier, recorded in
[CHANGELOG.md](CHANGELOG.md). The base then stays frozen. Timestamped SQLx files
in [schemas/migrations/](../schemas/migrations/) carry each later structural
change forward. SQLx records those changes in `ple_migration._sqlx_migrations`;
it does not define the pre-production design. The directory is empty today
because no post-cutover change exists.

The administration coordinator holds one advisory lock. `initialize` checks
only whether `ple_api.ple_schema_state` exists, installs the base when it does
not, validates the expected release identity, and verifies the projection. The
`pre-production` identity makes that path base-only: `migrate` rejects and
forward SQLx execution is guarded below the command boundary. A frozen identity
uses the existing apply-and-verify path for recognized forward migrations. SQLx
owns dirty-ledger, changed-checksum, and unknown-version rejection. The base
manifest also rejects an unrelated user schema or persistent `public` relation
before creating PLE schemas, so initialization remains a clean dedicated
database operation rather than a repair mechanism.

## Schemas, roles, and database seams

The base creates four application schemas with separate no-login owners:

| Schema        | Responsibility                                                                                          |
| ------------- | ------------------------------------------------------------------------------------------------------- |
| `ple_data`    | Shared product records, stable lineages, current teaching configuration, and durable public facts.      |
| `ple_private` | Account-private state and Student Work, including attempts, saved responses, and presentation bindings. |
| `ple_audit`   | Append-only, deliberately limited audit evidence.                                                       |
| `ple_api`     | Narrow, authenticated database operations and application-safe readers.                                 |

`ple_migration` is separate and contains only the SQLx ledger. The platform
bootstrap creates the complete PLE role graph. The base validates that graph,
then creates schemas, objects, and explicit grants. Runtime login roles receive
only their intended capabilities; they are not schema owners and do not receive
general DDL authority.

Tables are closed to `PUBLIC`, use explicit grants, and use forced row-level
security where access must be scoped. Database functions establish the
operation seams for authenticated work. Privileged functions use fixed search
paths and narrow grants rather than relying on callers to assemble a safe
transaction. `ple_api.ple_schema_state` is the application-readable,
read-only projection of the current base release identity and SQLx forward
ledger. Its identity is `pre-production` today; the first human-approved
production cutover installs the matching immutable production-baseline
identifier described above.

`ple_private.account_time_zone` stores one exact installed-IANA name per Account.
Student profile procedures derive the active Student from the installed session
and accept no Account identifier. A Student Account created by Course Roster
Import carries a private pending-default flag; its first accepted invitation
copies the recorded inviting Instructor's zone and clears the flag. A Student
choice also clears it, so later acceptance cannot overwrite that choice, and an
existing Student Account is never marked for a new default.

## Questions and Blueprints

A Published Question has a stable lineage in `ple_data.published_question`.
Its canonical identifier is the compact seven-character Crockford Base32 value;
the hyphenated `AAA-BBBB` form is presentation only. A Question Revision is an
immutable `(question_id, revision_number)` identity. Publication, stewardship,
source bindings, authorship, licensing, citations, assets, and the lineage's
availability events retain the facts that make an exact Revision interpretable.

Question Type is immutable, author-declared educational metadata on the
Published Question Revision. The author selects it on the mutable Draft
Question Source Binding, and publication copies that value into
`ple_data.question_revision`. Search, filtering, and labels use that retained
metadata. An external Question Backend may use any valid controls or document
structure for the declared type; PLE does not infer Question Type from backend
output.

Availability is current state on the stable Question lineage, qualified by an
Availability Edit Number and append-only transition events. Archiving excludes
the lineage from ordinary discovery and new selection while preserving
authorized resolution of exact existing Revision references. A new Revision
does not reset lineage availability.

A Blueprint Course follows the same stable-lineage pattern. Complete valid
creation atomically produces its Available lineage and Revision 1. Its complete
reusable structure appears only in immutable Revision rows. A changed explicit
Save based on the current Revision inserts one successor; canonical content
that is unchanged returns the current Revision with `changed: false`. The
unversioned canonical pre-production encoding, its checksum, and exact stored
aggregate equality make that decision. Replays resolve their existing receipt.
Blueprint short name, long name, and availability are lineage metadata under
one opaque ETag, so archive and restore do not break exact Revision resolution
or create a Revision.

`BlueprintAssignmentSource` records provenance with one stable Blueprint
Assignment reference and one exact Blueprint Revision reference. It is
provenance, not a third Revision family.

## Courses and Assignments

A Course Instance holds its current Course Term dates and course identity
directly, with origin, membership, roster, and course-operation records around
it.

An Assignment is one current aggregate with a qualified Assignment Edit Number
and lifecycle status. Its normalized Assignment Entries and Question Pool Items
are current teaching configuration. Every fixed entry and pool item pins an
exact Question Revision, so publishing a newer Question Revision never moves an
Assignment silently. A guarded save changes the aggregate and its normalized
children as one accepted current state. Release is a lifecycle transition;
accepted released edits affect later Attempts after release validation
succeeds.

Generic jobs and object cleanup remain technical facilities. Forced Question
Correction retains the evidence required by its implemented workflow.

Jobs remain internal technical records. Public-asset publication is an existing
Job workload. The Phase 2 worker is a single generic expiry worker: it shares
the ordinary Attempt evaluator and performs a 60-second expiry sweep. It reads
immutable source only through S3 and the private WeBWorK renderer boundary.
iMathAS retains its separate session and receipt boundary. The worker creates no
public grading state, attention count, polling surface, or Instructor grading
action. A completed immutable Grading Result cannot be reopened or replaced.

## Student Work as retained evidence

Before Student Work exists, `read_student_assignment_access` and
`list_released_live_student_assignments` use one statement timestamp and the
same private Assignment Start Decision function. Both project only the current
Student's effective available, due, close, whole-Attempt time-limit, Attempt-
limit, and late-work values together with the Student Account's IANA display
zone. Released Assignments scheduled for the future remain on the landing;
their server decision is `not_yet_available`. Accommodation identity never
crosses either reader.

`ple_private.assignment_attempt` is the Student Work root. At start it retains
the effective title, instructions, availability, due and close instants,
whole-Attempt time limit, one immutable `expires_at` equal to the earlier of
retained close and start plus retained time limit, attempt limit, completion, late-work, feedback,
reuse, variation, ordering, and navigation rules. When an accommodation affects
an effective value, the Attempt also retains its qualified accommodation source
and Edit Number. Later current Assignment saves therefore cannot reinterpret
existing work; a later Attempt receives the current accepted configuration.

`issued_question` retains the Assignment Entry identity and issue position, the
exact Question Revision, a Question Seed, point value, scoring rule, statistics
eligibility, per-question limits, and pool-selection provenance. Pool selection
records are owned by the Attempt and retain their selected exact Revision
identities. Question Attempts, saved responses, submissions, grading,
presentation, history, correction targets, and statistics observations extend
that root through relational ownership.

Presentation bindings retain the exact reproducibility and delivery facts,
including generated-parameter/replay details and selected ready asset rendition
metadata. Their private normalized response-item bindings map each
presentation-scoped four-hex reference to its durable authored response-item
identity, so retained work remains interpretable without a mutable source
lookup. Object Records name verified object facts such as address, checksum,
size, media type, data class, and owner relationship. Question source and asset
relationships refer to those exact records; Student Work retains the
presentation binding it used rather than re-resolving mutable current assets.

For a WeBWorK-issued position, that immutable binding also retains one nonempty
`backend_document` with the explicit `webwork_presentation` capability. The
database accepts no backend document for PLE-native presentations and none for
non-presented positions. `read_student_assignment_attempt_backend_document`
returns only the exact document for the owning Student, open Attempt, and
issued position; it excludes the source, seed, response, and backend state.
There is no WeBWorK replay table, renderer cache, or per-attempt backend state.
The shared saved-response and whole-Attempt finalization lifecycle stores the
bounded opaque backend response without decoding PG fields.

Whole-Attempt finalization creates immutable Question Submissions for supported
saved responses. A Question Backend that completes immediately commits its
immutable normalized credit outcome and receipt through the ordinary submission
path; backend-specific polling remains internal where needed.

Student submission and deadline expiry share the private saved-response evidence
writer. The server-owned `expires_at` blocks changes after expiry. Student
operations that could change the Attempt enforce that expiry rule, and narrow
background execution finalizes expired Attempts that require no later Student
request. Reads remain read-only. At deadline, saved supported responses receive
submissions while unanswered Questions become `closed_at_deadline`; readers
resolve that state as zero. A zero-response Attempt completes at zero with no
Question Submission. `assignment_submission.finalization_kind` distinguishes
`student` from `deadline`; only the Student kind carries an authorizing Account.
An expired save is returned as a refusal only after its expiry transition commits,
so the incoming payload cannot roll back or overwrite finalization.

## Unrelease

Unrelease is a single database-owned Assignment operation. It locks the
Assignment before checking current Teaching Team authority, released state,
the expected Assignment Edit Number, and exact title confirmation. It reports
aggregate impact counts, returns the Assignment to `unreleased`, advances the
Edit Number, and deletes the Student Work closure rooted at that Assignment's
Attempts in the same transaction.

Root-oriented foreign-key cascades remove dependent issued questions, pool
selections, question attempts, saved responses, presentation bindings,
submissions, grading records, backend exchanges, statistics
observations, and correction-target links. Shared Question Revisions, assets,
current Assignment configuration, Course records, and membership survive. The
operation rebuilds affected Question Revision statistics from surviving
observations and writes an immutable redacted audit event containing only the
actor, Assignment, aggregate counts, outcome, and time. A dedicated no-login
executor owns the deletion capability; ordinary Student Work roles cannot
mutate or delete that evidence.

## Structure and installation data

The base manifest is DDL only. After structure and services are ready,
`cargo tools installation-data provision` creates the complete ordinary Live
Demo. It first runs the convergent Pilot publication and database-owned teaching
graph through [schemas/installation_data/install.sql](../schemas/installation_data/install.sql),
then uses the owning application paths for cross-system Student Work and grading
effects. `cargo tools installation-data apply` is the narrower database-owned
operation; it is not complete Live Demo provisioning.

The Live Demo uses ordinary schema and product records. In production, the
short-lived audited administration environment runs `provision` after the API,
worker, publisher, object storage, and browser origin are ready. An installation
owner can instead choose `cargo tools installation-data provision
--without-live-demo` before data is created. OpenTofu creates the private RDS
service but leaves this product-data step to the audited workflow; it receives
neither database nor application secrets and creates no one-shot provisioning
subsystem. Database-owned final state belongs in the data manifest. Effects
that genuinely belong to object storage, publication, grading backends, or
workers continue through their owning path.

## Verification boundary

The database administration path proves a fresh atomic install, compatible
replay, SQLx forward state, and the restricted application projection. Connected
PostgreSQL acceptance additionally exercises RLS, capability boundaries,
current Assignment and retained Student Work behavior, archive/restore,
Blueprint Revision Save, and Unrelease. Fast tests protect stable value and
transport contracts; they do not replace a connected database build. See
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the evidence categories.
