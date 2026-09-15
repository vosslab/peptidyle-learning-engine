# Contract register

This register names the target product boundaries of the Peptidyle Learning
Engine. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product authority and
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines the terms used here.
The linked base-schema and source paths are implementation evidence, not product
authority. Some retain pre-compliance `assignment` and Blueprint `available`
identifiers; those names record implementation gaps and do not supersede this
contract.

## Contract rule

A change to a published contract updates its owning implementation, direct
consumers, generated types or fixtures, relevant tests, this register, and the
changelog together. A browser decoder receives `unknown`, accepts only its
closed shape, and does not supply authority. Database functions derive account,
membership, and record scope from the authenticated session and their typed
arguments.

The application has one installation and global Accounts. There is no
institution tenancy boundary. Course, Assessment, Student Work, worker, and
object operations each authorize their exact parent relationship rather than a
caller-selected role or scope.

An Account Time Zone is a self-owned exact IANA display preference. The Student
read and update boundary accepts no Account identity and repeats active Student
authorization in PostgreSQL. Roster import marks only a newly created Student
Account for a one-time default; invitation acceptance copies the inviting
Instructor's zone unless the Student already chose a zone. Existing Accounts
keep their preference, and changing a zone never moves an absolute deadline.

## Reusable content

| Boundary                       | Current contract                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Owner and evidence                                                                                                                                                                                                      |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Question lineage and revision  | A Published Question is one stable lineage. A `QuestionRevisionReference` is its Question ID plus immutable positive revision number. Publication creates complete immutable Question Revision facts, including source, provenance, and required metadata. A later publication appends a revision; it does not alter an earlier one.                                                                                                                                                                     | [question_library.rs](../crates/question_model/src/question_library.rs), [question_publication.rs](../crates/server/src/question_publication.rs), [question_lineages.sql](../schemas/base_schema/question_lineages.sql) |
| Question discovery             | A Published Question is discoverable and selectable by vetted Instructors. Archive hides it from ordinary discovery and new selection while authorized exact Revision references continue to resolve. Current implementation may call the discoverable state `Available`; that name is not a separate product object or Revision state. | [question_library.rs](../crates/question_model/src/question_library.rs), [question_lineages.sql](../schemas/base_schema/question_lineages.sql) |
| Question ID                    | PostgreSQL stores the compact eight-character canonical Crockford Base32 value. Browser-facing display and serialization use `AAAA-ZBBB`; the hyphen is presentation only. Seven identity characters come from a cryptographically secure server source and the middle check character is validated by the server-held HMAC secret. Database uniqueness of the valid full value is the identity boundary.                                                                                                 | [question_library.rs](../crates/question_model/src/question_library.rs), [question_publication.rs](../crates/server/src/question_publication.rs), [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md)                            |
| Blueprint lineage and revision | A Blueprint Course is a stable reusable lineage created Private with Revision 1. A `BlueprintRevisionReference` names one exact immutable Revision; a changed explicit Save based on its current Revision creates the next one, while a canonical no-op returns the current Revision with `changed: false`. Blueprint Assessment provenance retains that exact Revision and the stable Blueprint Assessment reference when Course state derives from reusable content. | [contracts.rs](../crates/question_model/src/blueprint_operations/contracts.rs), [blueprints.sql](../schemas/base_schema/blueprints.sql)                                                                                 |

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

A fork retains its source Blueprint and Revision so later source changes can be
discovered and selectively brought into the fork. A Blueprint Course Change
Proposal presents canonical JSON differences to the receiving owner; accepted
changes create a new receiving Blueprint Revision and reach daughters only
through the normal update workflow.

## Teaching current state

| Boundary                       | Current contract                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Owner and evidence                                                                                                                                                                                                                    |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Course Instance                | A Course Instance starts empty or adopts an exact Public Blueprint Revision, atomically creating every Assessment with its Questions, pools, and reusable settings, fresh identities, Unreleased state, and unset dates. It owns deliberately entered names, Course Term dates, equal co-Instructor memberships, roster, and delivery state; it always has at least one assigned Instructor. Course Term is current Course state, not a Revision. | [course_term.rs](../crates/question_model/src/course_term.rs), [course_core.sql](../schemas/base_schema/course_core.sql), [course_operations.sql](../schemas/base_schema/course_operations.sql) |
| Assessment                     | An Assessment is one stable current aggregate under a Course Instance. Its positive Edit Number is the strong `If-Match` value for save, Assessment Properties save, release, and Unrelease. It owns current title, instructions, dates, settings, ordered Questions, and Question Pools. Every Question and Pool item pins an exact Question Revision; a newer Question Revision never advances it. | [edit_number.rs](../crates/question_model/src/assignment/edit_number.rs), [assignment.rs](../crates/question_model/src/assignment.rs), [assignments.sql](../schemas/base_schema/assignments.sql) |
| Assessment release and editing | Release is a current state transition, returning the current Assessment and new ETag. A released Assessment save uses the same ETag contract and is accepted only when the resulting current state passes release validation. Accepted edits affect future Attempts; prior Student Work keeps its retained facts. | [assignment_release.rs](../crates/server/src/assignment_release.rs), [assignment_release.rs](../crates/learning-data-access/src/assignment_release.rs), [assignment_operations.sql](../schemas/base_schema/assignment_operations.sql) |
| Assessment Unrelease           | An authorized Teaching Team member supplies the exact Assessment ETag and title confirmation. The database locks the Assessment, confirms Released status, changes it to Unreleased, and permanently deletes its Student Work. Shared Questions, current Assessment state, and Course membership survive. | [assignment_release.rs](../crates/server/src/assignment_release.rs), [unrelease.sql](../schemas/base_schema/unrelease.sql) |

An Instructor may deliberately publish reusable Course Instance structure as a
new Blueprint Course. The new Blueprint contains reusable content only; it does
not carry Students, Course dates, releases, Student Work, or other delivery
state.

`412 Precondition Failed` represents an ETag conflict, `422 Unprocessable
Entity` represents invalid resulting content or confirmation, and `409 Conflict`
represents an invalid lifecycle state. Authorization and non-resolvable targets
use the repository's non-enumerating response policy.

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
work, including a seed only for a backend that uses one. Native PLE Question
JSON is static and receives no random seed. Private normalized response-item bindings map each
presentation-scoped four-hex reference to the durable authored response-item
identity, so evidence readers interpret saved work without a mutable source
lookup. Saved responses are private mutable input while the Attempt is active.
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
percentage calculation. Pilot grade export is CSV or TSV point data.

| Boundary                                             | Owner and evidence                                                                                                                                                                                                                                                                |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Attempt access, start, resume, and retained evidence | [attempt_evidence.rs](../crates/question_model/src/student_work/attempt_evidence.rs), [assignment_delivery.rs](../crates/server/src/assignment_delivery.rs), [attempts.sql](../schemas/base_schema/attempts.sql), [attempt_access.sql](../schemas/base_schema/attempt_access.sql) |
| Issued Questions, interaction, and presentation      | [assignment_attempt.rs](../crates/learning-data-access/src/assignment_attempt.rs), [attempt_interaction.sql](../schemas/base_schema/attempt_interaction.sql), [attempt_presentation.sql](../schemas/base_schema/attempt_presentation.sql)                                         |
| Submission, grading, history, and statistics         | [submission.rs](../crates/server/src/assignment_delivery/submission.rs), [grading.sql](../schemas/base_schema/grading.sql), [attempt_history.sql](../schemas/base_schema/attempt_history.sql), [statistics.sql](../schemas/base_schema/statistics.sql)                            |

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
| Background work    | Public-asset publication may use an internal Job. A background process also ensures expired Assessment Attempts are submitted. Neither creates a Student or Instructor grading workflow, attention state, retry control, or polling UI. Human Guidance does not establish backend-grading polling as a product contract. | [jobs.sql](../schemas/base_schema/jobs.sql), [grading.sql](../schemas/base_schema/grading.sql) |
| Objects and assets | Object records are typed by their owning Question, workspace, Course, or Student Work relationship. Public-asset publication and delivery retain their actual object ownership; a caller cannot provide storage scope or widen an object reference.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | [object_records.sql](../schemas/base_schema/object_records.sql), [question_assets.sql](../schemas/base_schema/question_assets.sql), [delivery.sql](../schemas/base_schema/delivery.sql)                     |
| Base installation  | [install.sql](../schemas/base_schema/install.sql) is a deliberately small ordered `psql` manifest. Its domain modules directly own the current structural DDL, functions, RLS policies, grants, and constraints. Before the first human-approved production deployment, structural corrections modify their owning base module. At that cutover the SQL projection and Rust `BASE_RELEASE_IDENTITY` change together from `pre-production` to one immutable production-baseline identifier recorded in [CHANGELOG.md](CHANGELOG.md). The base then stays frozen and `schemas/migrations/` contains forward-only SQLx migrations.                                                                                                                                                                                                                                                                                                                                                             | [install.sql](../schemas/base_schema/install.sql), [database_coordinator.rs](../crates/project-tools/src/database_coordinator.rs), [sqlx.toml](../crates/learning-data-access/sqlx.toml)                    |
| Administration     | `cargo tools database initialize`, `migrate`, and `verify` are the bounded administration path. SQLx records forward migrations in `ple_migration._sqlx_migrations`; runtime roles have neither DDL authority nor write access to that ledger. `ple_api.ple_schema_state` is the restricted application-visible projection of the current base release identity and SQLx ledger. It reports `pre-production` today, then the immutable production-baseline identifier established at the first human-approved cutover.                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | [database.rs](../crates/project-tools/src/database.rs), [database_coordinator.rs](../crates/project-tools/src/database_coordinator.rs), [foundation_roles.sql](../schemas/base_schema/foundation_roles.sql) |
| Installation data  | A fresh installation uses the same production schema and publishes the shipped Genetics Blueprint Course. It includes the complete Live Demo by default. `provision --without-live-demo` omits the optional Live Demo Accounts, Course, and activity while retaining Genetics. The resulting records follow ordinary product lifecycle and deletion rules. | [installation_data.rs](../crates/project-tools/src/installation_data.rs), [install.sql](../schemas/installation_data/install.sql), [README.md](../schemas/installation_data/README.md) |

There is no separate demo schema, demo role, marker, teardown capability, or
alternate product representation.

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
  Reference. Current Course and Assessment configuration uses current state
  with its qualified Edit Number where concurrent writers need one.
- Student Work is independently interpretable from its retained evidence.
- Lists are bounded and cursor-based. Browser transport is same-origin and
  successful bodies are `no-store` where they contain protected current state.
- Database routines use fixed search paths, narrow privileges, forced RLS where
  appropriate, and transactions for cross-record lifecycle changes.
