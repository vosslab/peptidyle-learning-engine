# Code architecture

PLE separates reusable teaching content from Course-owned Student records. The
browser receives role-appropriate, answer-safe views; PostgreSQL and
server-side services retain authorization, private Question source, backend
state, grading inputs, and FERPA-protected Student Work.

[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product authority. Current source
paths document the implementation. Assignment appears only in the three
Assessment Type names `regular_assignment`, `practice_question_assignment`, and
`bonus_assignment`. The generic teaching object is Assessment
(`ple_data.assessment` in
[assessment.sql](../schemas/base_schema/20_tables/assessment.sql)).

## Major components

| Component | Ownership |
| --- | --- |
| [crates/question_model/](../crates/question_model/) | Shared typed content, current Assessments, Question and Blueprint Revisions, current Question Pools, and Student Work structures |
| [crates/domain/](../crates/domain/) | Pure policy, timing, validation, score calculation, and disclosure rules |
| [crates/learning-data-access/](../crates/learning-data-access/) | Store contracts, PostgreSQL transactions, and row-security context |
| [crates/server/](../crates/server/) | HTTP routes, authenticated composition, authorization, and server-only dependencies |
| [crates/grading/](../crates/grading/) | Native answer-bearing grading code; never a browser dependency |
| [crates/adapters/](../crates/adapters/) | Native, WeBWorK, and iMathAS boundaries behind Question Backend operations |
| [crates/objects/](../crates/objects/) | Typed object identity, integrity, validation, and storage backends |
| [crates/browser-api-contract/](../crates/browser-api-contract/) | Rust declarations used to generate browser-facing TypeScript contracts |
| [src/](../src/) | SolidJS shell, strict decoders, role-specific pages, and answer-safe interaction |
| [src/appearance/](../src/appearance/) | One `AppearanceOwner` resolves Themes and display mode, then applies them to `<html>` |
| [local_stack_control/](../local_stack_control/) | Disposable-stack lifecycle and connected acceptance orchestration |

The Rust workspace is rooted at [Cargo.toml](../Cargo.toml); browser tooling is
declared in [package.json](../package.json). Generated TypeScript derives from
Rust declarations, while authored runtime decoders reject unexpected wire data.

## Product-domain boundaries

```text
Draft Question --publish--> Published Question + complete Question Revision records
Question Pool -------------> unordered set of Question Revision Tuples + metadata + Edit Number

Blueprint Course (Public; Active/Archived lifecycle)
  +-- complete Blueprint Revision records
  +-- Blueprint Assessments
               |
               | adopt exact Active Blueprint Revision
               v
Course Instance
  +-- equal co-Instructor and Student relationships
  +-- Course Instance Assessments (current state)
        +-- Assessment Attempts and FERPA-protected Student Work
              pins Question ID, Question Revision, Pool ID, Pool Edit Number
```

Blueprints contain no Students, dates, time zones, or relative schedules. A
Course Instance can instead start empty. Adoption copies current Blueprint
Assessments. New Blueprint Revisions are offered to daughter Course Instances
for Instructor review and approval, and changes to existing Assessments are
never applied silently. A newly added Blueprint Assessment is automatically
copied as an Unreleased Course Instance Assessment. Published Questions and
Blueprint Courses use numbered Revisions for saved content. Each Revision is a complete record;
permitted metadata edits preserve its Revision Number. A Pool contains an unordered set of
Question Revision Tuples plus its metadata. Saving changes advances its Edit Number.

Forks retain their source Blueprint and Revision so later source changes can be
discovered and selectively applied. Blueprint Course Change Proposals compare
canonical Blueprint JSON, leave acceptance with the receiving owner, and create
a receiving Blueprint Revision only when accepted. They never directly change
a daughter Course Instance.

A Course Instance keeps its own deliberately entered short and long names and
at least one assigned Instructor. Publishing its reusable structure creates a
new Active Blueprint lineage without copying Students, Course dates, releases,
Student Work, or other delivery state.

Account, Course Instance, Assessment, Blueprint Course, Published Question, and
Question Pool store identity as the public ID primary key. There is no parallel
UUID primary key for those objects. Browser JSON uses `id` (and nested
`courseId` / `assessmentId`) for those public IDs. Assessment Attempts, entries,
and sessions remain UUID with JSON field `id`. Content-addressed pairs such as
`QuestionRevisionTuple` keep that name; they are not a second stored object
key.

## Assessment data flow

```text
Assessment selects a fixed Question Revision or a current Pool
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

Saving updates the working row on
`ple_private.assessment_attempt_saved_response` in
[assessment_attempt.sql](../schemas/base_schema/20_tables/assessment_attempt.sql).
Whole-Assessment submission finalizes those rows in place. HTTP lives under
`/api/assessment-attempts/`. Current Assessment Pool entries pin Pool ID only;
issue-time Student Work also stores Pool Edit Number plus the selected Question
ID and Question Revision.

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
structure. [install.sql](../schemas/base_schema/install.sql) is a small ordered
`psql` manifest. Source is layered by kind, then domain:

- [00_roles.sql](../schemas/base_schema/00_roles.sql)
- [10_types.sql](../schemas/base_schema/10_types.sql)
- [15_table_check_functions.sql](../schemas/base_schema/15_table_check_functions.sql)
- [20_tables/](../schemas/base_schema/20_tables/) (`CREATE TABLE` and `COMMENT ON` only)
- [22_reference_grants.sql](../schemas/base_schema/22_reference_grants.sql)
- [30_constraints.sql](../schemas/base_schema/30_constraints.sql)
- [40_indexes.sql](../schemas/base_schema/40_indexes.sql)
- [50_functions/](../schemas/base_schema/50_functions/)
- [60_policies.sql](../schemas/base_schema/60_policies.sql) (includes [60_policies/](../schemas/base_schema/60_policies/))
- [70_grants.sql](../schemas/base_schema/70_grants.sql) (includes [70_grants/](../schemas/base_schema/70_grants/))

A domain does not own tables, functions, and grants in one file. Before
production baseline approval, a structural fix changes the owning layer file.
Later production changes use reviewed forward migrations.

Structure and ordinary installation data are separate. Live Demo records use
the normal domain model. Omitting the optional Live Demo teaching graph does
not remove the normal Genetics Question bundle.

See [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) and
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md).

## Authorization boundaries

The server resolves the authenticated Account and current User Role, then
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

Instructor Assessment edit, release, and Unrelease UI lives in
[src/pages/assessment_workspace/](../src/pages/assessment_workspace/). Student
Coursework entry lives on Student Course landing and Assessment Attempt pages.

## UI composition

[src/application_shell.tsx](../src/application_shell.tsx) owns the persistent
signed-in shell: course-theme variables, Ribbon, breadcrumb prelude, route
scope provider, focus movement, and the main content boundary.
[src/ribbon/](../src/ribbon/) supplies the Ribbon catalog, route scope, and
typed model. [src/route_access_boundary.tsx](../src/route_access_boundary.tsx)
admits browser routes from the route contract; server routes and PostgreSQL
remain the authorization boundary for each operation and resource.

[src/components/page_frame/page_frame.tsx](../src/components/page_frame/page_frame.tsx) owns route
content width, standard vertical spacing, page-level actions, and the content
stack. The Ribbon and breadcrumbs identify the page. Reading width is the
default; a route may declare `fullWidth` for dense content without coupling
that choice to Ribbon navigation. A `contentClass` hook is only for a
task-specific arrangement inside that stack.
Page route compositions own PageFrame and route identity. The Library Search, Browse, My Questions,
and My Pools pages compose LibraryWorkspace; Blueprint list and detail pages compose their feature
workspaces and own their frames. Feature navigation stays with the domain workflow when it owns the
reactive state.
[src/components/data_sheet/](../src/components/data_sheet/) provides the shared DataSheet
presentation. `DataSheetBlock` owns Compact, List, Cards, and Visual Boxes, with shared
sorting, filtering, selection, record actions, and stable-ID drag/drop interactions.
Compact and List use `DataSheetTable`, which owns column configuration and independent
temporary widths, visibility, and order for each table mode. Its `HeaderRow` forwards
sort/filter input to the shared owners, and `RecordItem` supplies record content in all
four modes. `DataSheetControls` invokes the same presentation/table state for surrounding
mode, query, paging, count, and bulk controls.
[record_family.css](../src/components/data_sheet/record_family.css) owns shared collection
states and table skin, including internal horizontal scrolling. Pool and Assessment
owners provide editor-specific commands and retain membership and saved order.

The [shared search modules](../src/components/search_block/) connect collection consumers.
`createSearchState` is the sole reactive controller and lifecycle owner for search query state,
numbered paging, requests, selection, retry, and disposal. `search_definition.ts` contains shared
search definitions and types. `SearchBlock` adapts controller state and places supplied criteria and controls
above or beside one required `DataSheetBlock`; `DataSheetBlock` mounts `DataSheetControls` before
the record region and owns presentation and empty states. Query owners apply scope, authorization,
filters, sorting, and counts to the complete collection before paging. Question, Pool, and Blueprint
discovery use shared page sizes of 50, 100, or 250 records.

`createLocalCompleteSearch` adapts a complete reactive rows accessor with optional loading, error,
and source-refetch callbacks. It returns `{ state, collection: { fields, query } }`; callers declare
fields once and pass the collection binding to `DataSheetBlock`. `DataSheetQuery` carries optional
sorting and filtering bindings without a paging-kind discriminator. The Library assembly in
[question_library_search_definition.ts](../src/features/question_library/question_library_search_definition.ts) builds
that same state and collection binding for [LibraryWorkspace](../src/features/question_library/library_workspace.tsx), the Assessment content picker, and
QuestionPoolAvailableQuestions while each caller keeps its own scope, permissions, actions, edit
guards, and classification. Reorder bindings identify records by stable IDs. Consumers retain
their domain commands and source lifecycle. See
[SEARCH_PAGE_ARCHITECTURE.md](SEARCH_PAGE_ARCHITECTURE.md) for the implemented boundary and
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) for its decisions.
The combined Library query uses one globally paged Question/Pool server union. `/library/{id}`
resolves the authorized object kind before dispatching to Question or Pool detail. The Question
picker retains its ordered domain tray; the Assessment content picker selects Questions or one Pool
through the mixed Library definition. See
[SHARED_SEARCH_PAGE_SCHEMATIC.md](archive/SHARED_SEARCH_PAGE_SCHEMATIC.md).
Pool editor sorting is display-only; the shared reorder component does not make Pool membership
ordered. [question_specs_alignment_report.md](archive/reports/question_specs_alignment_report.md)
records related implementation gaps.

Published WeBWorK Question thumbnails are immutable publication artifacts tied to the exact
published Revision. The private seed-zero renderer, publication/correction, importer/backfill, and
zero-capture browsing paths passed bounded M19-M22 acceptance. See
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) and the active plan for ownership and evidence;
final integrated/runtime gates remain separate.

Pages compose these pieces around their domain state. For example,
[src/features/blueprint_course/blueprint_course_detail_workspace.tsx](../src/features/blueprint_course/blueprint_course_detail_workspace.tsx)
holds a locally editable Blueprint draft until explicit Save creates the next
Revision. [src/pages/assessment_workspace/assessment_workspace_questions_view.tsx](../src/pages/assessment_workspace/assessment_workspace_questions_view.tsx)
owns the Assessment-entry draft and its explicit save. Student response
controls, including [src/components/question_response_controls/ordering.tsx](../src/components/question_response_controls/ordering.tsx),
retain their response-specific timing and status behavior.

## Storage and retention

Typed object records bind logical identity, data class, owner scope, media type,
and checksum. The server constructs physical paths and authorized delivery;
browsers do not name buckets or raw keys.

The current storage composition accepts only an explicitly configured,
authenticated disposable-local MinIO topology through a typed S3-compatible
adapter. It retains the four Object Storage Areas and separate publisher
credentials, but does not implement cloud production support.

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

## Known gaps

- Unused leftover [src/pages/assignment_access/](../src/pages/assignment_access/)
  is not Student Coursework UI.
- Published Question `availability` still uses the `available` enum label.
