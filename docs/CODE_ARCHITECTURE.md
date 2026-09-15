# Code architecture

PLE separates reusable teaching content from Course-owned Student records. The
browser receives role-appropriate, answer-safe views; PostgreSQL and
server-side services retain authorization, private Question source, backend
state, grading inputs, and FERPA-protected Student Work.

[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product authority. Current source
paths and identifiers document the implementation, including gaps where
`assignment`, `available`, or response-finalization names have not yet been
migrated.

## Major components

| Component | Ownership |
| --- | --- |
| [crates/question_model/](../crates/question_model/) | Shared typed content, current Assessment implementation, and Student Work structures |
| [crates/domain/](../crates/domain/) | Pure policy, timing, validation, score calculation, and disclosure rules |
| [crates/learning-data-access/](../crates/learning-data-access/) | Store contracts, PostgreSQL transactions, and row-security context |
| [crates/server/](../crates/server/) | HTTP routes, authenticated composition, authorization, and server-only dependencies |
| [crates/grading/](../crates/grading/) | Native answer-bearing grading code; never a browser dependency |
| [crates/adapters/](../crates/adapters/) | Native, WeBWorK, iMathAS, and QTI boundaries behind Question Backend operations |
| [crates/objects/](../crates/objects/) | Typed object identity, integrity, validation, and storage backends |
| [crates/browser-api-contract/](../crates/browser-api-contract/) | Rust declarations used to generate browser-facing TypeScript contracts |
| [src/](../src/) | SolidJS shell, strict decoders, role-specific pages, and answer-safe interaction |
| [local_stack_control/](../local_stack_control/) | Disposable-stack lifecycle and connected acceptance orchestration |

The Rust workspace is rooted at [Cargo.toml](../Cargo.toml); browser tooling is
declared in [package.json](../package.json). Generated TypeScript derives from
Rust declarations, while authored runtime decoders reject unexpected wire data.

## Product-domain boundaries

```text
Draft Question --publish--> Published Question + immutable Revisions
Question Pool -------------> stable Pool + immutable Revisions

Blueprint Course (Private/Public/Archived)
  +-- immutable changed-content Revisions
  +-- Blueprint Assessments
               |
               | adopt exact Public Blueprint Revision
               v
Course Instance
  +-- equal co-Instructor and Student relationships
  +-- Course Instance Assessments (current state)
        +-- Assessment Attempts and FERPA-protected Student Work
```

Blueprints contain no Students, dates, time zones, or relative schedules. A
Course Instance can instead start empty. Adoption copies current Blueprint
Assessments. New Blueprint Revisions are offered to daughter Course Instances
for Instructor review and approval, and changes to existing Assessments are
never applied silently. A newly added Blueprint Assessment is automatically
copied as an Unreleased Course Instance Assessment. Published Questions,
published Question Pools, and Blueprint Courses have immutable Revision
families.

Forks retain their source Blueprint and Revision so later source changes can be
discovered and selectively applied. Blueprint Course Change Proposals compare
canonical Blueprint JSON, leave acceptance with the receiving owner, and create
a receiving Blueprint Revision only when accepted. They never directly change
a daughter Course Instance.

A Course Instance keeps its own deliberately entered short and long names and
at least one assigned Instructor. Publishing its reusable structure creates a
new Private Blueprint lineage without copying Students, Course dates, releases,
Student Work, or other delivery state.

## Assessment data flow

```text
Assessment fixes Question or Pool Revision selection
  -> authorized Student starts or resumes an Assessment Attempt
  -> backend renders one answer-free Question with opaque state
  -> Student saves complete responses and navigates all Questions
  -> Student or deadline submits the whole Assessment Attempt
  -> unanswered Questions receive zero and count as incorrect without backend work
  -> backend credit fractions remain immutable
  -> PLE calculates scores from current Question point values
  -> highest submitted Assessment Attempt score is used
  -> policy discloses permitted results and feedback
```

Saving changes only the working response. The only submission workflow closes
the whole Assessment Attempt. Current internal `question_submission` rows or
`/assignment-attempts/` routes are implementation structures beneath this
target product boundary; their names do not define product actions.

Student interaction checks Attempt expiration, and background processing
ensures an expired Attempt is submitted even after the Student leaves. Both
paths converge on the same idempotent whole-Assessment submission operation.

## Question Backend flow

```text
trusted exact Question source + Revision + randomization/backend state
  -> registered Question Backend
  -> answer-free native presentation or opaque document + opaque state
  -> typed native response or bounded opaque response
  -> backend-owned evaluation and feedback
  -> immutable credit fraction stored by PLE
```

PLE never infers Question Type from backend controls or implements a second
parser for them. WeBWorK form names, values, hidden fields, and interaction
semantics remain opaque. Native PLE Question JSON is private, unpublished,
unversioned, static, and strictly validated; author JavaScript is isolated and
untrusted.

If a real backend needs deferred technical completion, that mechanism remains
inside the adapter boundary. Human Guidance does not define a public grading
job, Retry action, regrading lifecycle, or mutable result.

## Database lifecycle

[schemas/base_schema/](../schemas/base_schema/) is the canonical fresh-install
structure. Its short manifest includes domain modules in dependency order and
installs atomically. Before production baseline approval, a structural fix
changes the owning module. Later production changes use reviewed forward
migrations.

Structure and ordinary installation data are separate. Live Demo records use
the normal domain model. Omitting the optional Live Demo teaching graph does
not remove the normal Genetics Question bundle.

See [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) and
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md).

## Authorization boundaries

The server resolves the authenticated Account and current Product Role, then
checks the exact Course, Student, workspace, or Blueprint-owner relationship.
PostgreSQL repeats the protected predicate through forced RLS and narrow
functions in the same transaction.

All current co-Instructors are equal. The Course creator has no extra
privilege. Sysadmin support access to FERPA-protected records is deliberate,
scoped, and recorded rather than ambient.

## Browser flow

```text
SolidJS page
  -> same-origin client and strict decoder
  -> authenticated server route
  -> Store and PostgreSQL authorization boundary
  -> typed, role-appropriate response
  -> accessible stable-layout interface
```

The Instructor Ribbon uses Courses, Questions, and Assessments. Student
delivery appears under Coursework, and a particular item uses its Assessment
Type name. Sign Out belongs in the Profile menu. Required backed destinations
remain visible with honest empty states; unimplemented capabilities are not
shown as usable controls.

Current source directories such as `src/pages/assignment_workspace/` document
where legacy implementation lives. New product documentation and UI use
Assessment terminology.

## Storage and retention

Typed object records bind logical identity, data class, owner scope, media type,
and checksum. The server constructs physical paths and authorized delivery;
browsers do not name buckets or raw keys.

The Course Instance becomes Inactive six months after creation. That limit caps
Assessment deadline movement so Course reuse cannot indefinitely delay FERPA
retention and deletion, but becoming Inactive does not itself delete Student
records. The latest Assessment deadline starts the FERPA retention clock. An
idempotent background pass supports later notice, FERPA archive, recovery during
the retention period, and permanent deletion while preserving Course metadata,
Assessment definitions, Questions, and settings. Exact job/table shapes and
FERPA retention durations are not product decisions here.

## Extension points

- Add durable domain types before storage and transport adapters.
- Add PostgreSQL access through the learning-data-access boundary.
- Add routes through authenticated server composition and closed DTOs.
- Add a Question Backend only after its opaque source, render, response,
  grading, failure, and secret boundaries are complete.
- Add browser features only for capabilities admitted by the current role and
  backed by real routes.
- Add background processing only for an exact decided behavior; a generic job
  framework is not feature authority.

## Verification boundaries

Fast deterministic tests, Rust/TypeScript checks, connected PostgreSQL tests,
live backend probes, browser journeys, and visual captures establish different
evidence. Passing one layer does not claim another. See
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).
