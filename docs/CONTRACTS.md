# Contract register

This register names the current executable boundaries of the Peptidyle Learning
Engine. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product authority and
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines the terms used here.
The base schema is the implementation authority for relational ownership,
constraints, transactions, row-level security, and grants.

## Contract rule

A change to a published contract updates its owning implementation, direct
consumers, generated types or fixtures, relevant tests, this register, and the
changelog together. A browser decoder receives `unknown`, accepts only its
closed shape, and does not supply authority. Database functions derive account,
membership, and record scope from the authenticated session and their typed
arguments.

The application has one installation and global Accounts. There is no
institution tenancy boundary. Course, Assignment, Student Work, worker, and
object operations each authorize their exact parent relationship rather than a
caller-selected role or scope.

## Published reusable content

| Boundary | Current contract | Owner and evidence |
| --- | --- | --- |
| Question lineage and revision | A Published Question is one stable lineage. A `QuestionRevisionReference` is its Question ID plus immutable positive revision number. Publication creates complete immutable Question Revision facts, including source, provenance, and required metadata. A later publication appends a revision; it does not alter an earlier one. | [question_library.rs](../crates/question_model/src/question_library.rs), [question_publication.rs](../crates/server/src/question_publication.rs), [question_lineages.sql](../schemas/base_schema/question_lineages.sql) |
| Question availability | The stable lineage has current Available or Archived availability and a qualified edit number. Available content is discoverable and selectable. Archive hides it from ordinary discovery and new selection, while authorized exact revision references continue to resolve. Restore is the ordinary inverse transition. | [question_library.rs](../crates/question_model/src/question_library.rs), [question_lineages.sql](../schemas/base_schema/question_lineages.sql) |
| Question ID | PostgreSQL stores the compact seven-character canonical Crockford Base32 value. Browser-facing display and serialization use `AAA-BBBB`; the hyphen is presentation only. The first six characters come from a cryptographically secure server source and the final character is validated by the server-held HMAC secret. Database uniqueness of the valid full value is the identity boundary. | [question_library.rs](../crates/question_model/src/question_library.rs), [question_publication.rs](../crates/server/src/question_publication.rs), [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md) |
| Blueprint lineage and revision | A Blueprint Course is a stable reusable lineage with current availability and immutable published Blueprint Revisions. A `BlueprintRevisionReference` names one exact revision. `BlueprintAssignmentSource` retains both that exact revision and the stable Blueprint Assignment Reference when Course state derives from reusable content. | [contracts.rs](../crates/question_model/src/blueprint_operations/contracts.rs), [blueprints.sql](../schemas/base_schema/blueprints.sql) |
| Blueprint Draft and publish | Creating a Blueprint Course creates a private mutable Draft and no Revision. Draft saves use the Draft Edit Number and change it only on change. Explicit publication validates and copies complete Draft content to a new immutable Blueprint Revision; a received publication request converges on its receipt, and a deliberate publication is an event even when the Draft is unchanged. The owner retains the working Draft after publication. | [contracts.rs](../crates/question_model/src/blueprint_operations/contracts.rs), [blueprint_course.rs](../crates/server/src/blueprint_course.rs), [blueprints.sql](../schemas/base_schema/blueprints.sql) |

Blueprint availability has the same browsing/new-selection rule as Question
availability: archive hides the stable lineage, and existing exact Blueprint
Revision References remain resolvable. The owner performs archive with the
availability Edit Number and explicit confirmation; restore uses the Edit
Number. There is no Blueprint collaborator contract.

## Teaching current state

| Boundary | Current contract | Owner and evidence |
| --- | --- | --- |
| Course Instance | A Course Instance is live teaching created from an exact Blueprint Revision. It owns its current compact and descriptive names, Course Term dates, membership, roster, and delivery state. Course Term is one inclusive ordered start/end-date value; it is current course state, not a revision. | [course_term.rs](../crates/question_model/src/course_term.rs), [course_core.sql](../schemas/base_schema/course_core.sql), [course_operations.sql](../schemas/base_schema/course_operations.sql) |
| Assignment | An Assignment is one stable current aggregate under a Course Instance. Its positive Assignment Edit Number is the strong `If-Match` value for save, release, and Unrelease. It owns current title, instructions, schedule, policy, normalized entries, and Question Pool Items. Every entry and pool item pins an exact Question Revision; a newer Question Revision never advances it. | [edit_number.rs](../crates/question_model/src/assignment/edit_number.rs), [assignment.rs](../crates/question_model/src/assignment.rs), [assignments.sql](../schemas/base_schema/assignments.sql) |
| Assignment release and editing | Release is a current state transition, returning the current Assignment and new ETag. A released Assignment save uses the same ETag contract and is accepted only when the resulting current state passes release validation. Accepted edits affect future Attempts; prior Student Work keeps its retained facts. | [assignment_release.rs](../crates/server/src/assignment_release.rs), [assignment_release.rs](../crates/learning-data-access/src/assignment_release.rs), [assignment_operations.sql](../schemas/base_schema/assignment_operations.sql) |
| Assignment Unrelease | An authorized Teaching Team member supplies the exact Assignment ETag and title confirmation. The database locks the Assignment, confirms Released status, reports aggregate affected Attempts, submissions, and grades, changes it to Unreleased, deletes its Student Work closure, rebuilds surviving statistics, and records one redacted audit event in the same transaction. Shared Questions, current Assignment state, membership, and the audit event survive. | [assignment_release.rs](../crates/server/src/assignment_release.rs), [unrelease.sql](../schemas/base_schema/unrelease.sql) |

`412 Precondition Failed` represents an ETag conflict, `422 Unprocessable
Entity` represents invalid resulting content or confirmation, and `409 Conflict`
represents an invalid lifecycle state. Authorization and non-resolvable targets
use the repository's non-enumerating response policy.

## Student Work and assessment evidence

An Assignment Attempt is one independent Student Work occurrence. At start it
retains the effective Assignment title, instructions, policy, activity rules,
feedback rule, and qualified sources for any accommodation-adjusted schedule or
limits. Resume, submission, grading, history, disclosure, and statistics read
that retained evidence with issued-work evidence; they do not reinterpret an
old Attempt through a later Assignment save.

Each Issued Question retains its Assignment Entry identity and position, exact
Question Revision, Question Seed, point and scoring facts, statistics
eligibility, and pool-selection source. Question Attempt presentation and
reproduction records retain the exact backend and asset bindings used for that
issued work. Their private normalized response-item bindings map each
presentation-scoped four-hex reference to the durable authored response-item
identity, so evidence readers interpret saved work without a mutable source
lookup. Saved responses are private mutable input while the Attempt is active.
Whole-Attempt finalization atomically creates immutable Question Submissions
and one typed grading Job with its pending grading row per supported Question
before it creates the Assignment Submission. Grading records and result
receipts are rooted in those submissions. Statistics derive from surviving
observation receipts.

| Boundary | Owner and evidence |
| --- | --- |
| Attempt access, start, resume, and retained evidence | [attempt_evidence.rs](../crates/question_model/src/student_work/attempt_evidence.rs), [assignment_delivery.rs](../crates/server/src/assignment_delivery.rs), [attempts.sql](../schemas/base_schema/attempts.sql), [attempt_access.sql](../schemas/base_schema/attempt_access.sql) |
| Issued Questions, interaction, and presentation | [assignment_attempt.rs](../crates/learning-data-access/src/assignment_attempt.rs), [attempt_interaction.sql](../schemas/base_schema/attempt_interaction.sql), [attempt_presentation.sql](../schemas/base_schema/attempt_presentation.sql) |
| Submission, grading, history, and statistics | [submission.rs](../crates/server/src/assignment_delivery/submission.rs), [grading.sql](../schemas/base_schema/grading.sql), [attempt_history.sql](../schemas/base_schema/attempt_history.sql), [statistics.sql](../schemas/base_schema/statistics.sql) |

The Student presentation and submission routes expose answer-free,
presentation-scoped state. Grading, source bytes, answer keys, private feedback
internals, and worker lease material stay on their server and capability
boundaries.

## Jobs, objects, and database lifecycle

| Boundary | Current contract | Owner and evidence |
| --- | --- | --- |
| Job | A Job is a short-lived, lease-controlled execution record with one immutable target. Whole-Attempt finalization creates one typed grading Job and pending grading row for each supported accepted Question Submission before the Assignment Submission. Public-asset publication creates the other Job target. State moves through ready, leased, completed, or failed under the database transition rule. | [jobs.sql](../schemas/base_schema/jobs.sql), [grading.sql](../schemas/base_schema/grading.sql) |
| Objects and assets | Object records are typed by their owning Question, workspace, Course, or Student Work relationship. Public-asset publication and delivery retain their actual object ownership; a caller cannot provide storage scope or widen an object reference. | [object_records.sql](../schemas/base_schema/object_records.sql), [question_assets.sql](../schemas/base_schema/question_assets.sql), [delivery.sql](../schemas/base_schema/delivery.sql) |
| Base installation | [install.sql](../schemas/base_schema/install.sql) is a deliberately small ordered `psql` manifest. Its domain modules directly own the current structural DDL, functions, RLS policies, grants, and constraints. Before the first human-approved production deployment, structural corrections modify their owning base module. At that cutover the SQL projection and Rust `BASE_RELEASE_IDENTITY` change together from `pre-production` to one immutable production-baseline identifier recorded in [CHANGELOG.md](CHANGELOG.md). The base then stays frozen and `schemas/migrations/` contains forward-only SQLx migrations. | [install.sql](../schemas/base_schema/install.sql), [database_coordinator.rs](../crates/project-tools/src/database_coordinator.rs), [sqlx.toml](../crates/learning-data-access/sqlx.toml) |
| Administration | `cargo tools database initialize`, `migrate`, and `verify` are the bounded administration path. SQLx records forward migrations in `ple_migration._sqlx_migrations`; runtime roles have neither DDL authority nor write access to that ledger. `ple_api.ple_schema_state` is the restricted application-visible projection of the current base release identity and SQLx ledger. It reports `pre-production` today, then the immutable production-baseline identifier established at the first human-approved cutover. | [database.rs](../crates/project-tools/src/database.rs), [database_coordinator.rs](../crates/project-tools/src/database_coordinator.rs), [foundation_roles.sql](../schemas/base_schema/foundation_roles.sql) |
| Installation data | A fresh installation uses the same production schema, then ordinarily runs `cargo tools installation-data provision`. `provision` converges the database-owned Live Demo graph and creates required cross-system facts through their ordinary owning paths. `provision --without-live-demo` omits product data explicitly. `apply` remains the narrower convergent database-owned operation, not complete Live Demo provisioning. The resulting records follow ordinary product lifecycle and deletion rules. | [installation_data.rs](../crates/project-tools/src/installation_data.rs), [install.sql](../schemas/installation_data/install.sql), [README.md](../schemas/installation_data/README.md) |

There is no separate demo schema, demo role, marker, teardown capability, or
alternate product representation.

## Explicitly absent contracts

The following retired structures have no current contract, compatibility reader,
endpoint, decoder, or reserved implementation slot:

- Assignment Revision and its entry, pool, release-snapshot, and successor
  records.
- Course Schedule Revision.
- Question Change Proposal Revision.
- Course Retention Plan Revision and retention-specific Job targets.
- Blueprint collaborators and collaborator-close publication behavior.
- Legacy read fallbacks from Student Work to mutable Assignment configuration.
- Demo-only provisioning state, receipts, schema concepts, and report records.

Future capability work introduces a complete vertical boundary-authorization,
Store, Server, schema, browser workflow, and tests-when it has an approved
product decision. It does not reserve a schema scaffold in advance.

## Shared artifact ownership

| Artifact | Owning module |
| --- | --- |
| `schemas/base_schema/` | PostgreSQL base-schema owner |
| `schemas/migrations/` | PostgreSQL forward-migration owner |
| `schemas/installation_data/` | installation-data owner |
| `generated/api/` | Rust model and `cargo tools tsgen` |
| `containers/compose.yaml` | local-stack deployment owner |
| `tests/e2e/compose.live-demo-browser.yaml` | local-stack deployment owner |

Generated TypeScript is derivative of its Rust model. Fixtures are test input,
not a runtime API or another contract owner.

## Boundary invariants

- Rust and PostgreSQL own domain truth; browser code decodes and renders the
  authorized, answer-free projection it receives.
- Published Question and Blueprint provenance always uses an exact Revision
  Reference. Current Course and Assignment configuration uses current state
  with its qualified Edit Number where concurrent writers need one.
- Student Work is independently interpretable from its retained evidence.
- Lists are bounded and cursor-based. Browser transport is same-origin and
  successful bodies are `no-store` where they contain protected current state.
- Database routines use fixed search paths, narrow privileges, forced RLS where
  appropriate, and transactions for cross-record lifecycle changes.
