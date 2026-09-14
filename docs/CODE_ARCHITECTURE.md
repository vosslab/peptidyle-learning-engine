# Code architecture

PLE is a pre-production teaching platform. Its core boundary is between reusable published
content and Course-owned teaching records: the browser receives answer-free views, while PostgreSQL
and server-side services retain authorization, answer keys, grading inputs, and Student Work.
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product authority and
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines the implementation vocabulary.

## Major components

| Component                                                       | Ownership                                                                                                                 |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| [crates/question_model/](../crates/question_model/)             | Shared typed identifiers, Question and Blueprint contracts, current Assignment state, and retained Student Work evidence. |
| [crates/domain/](../crates/domain/)                             | Pure policy, timing, validation, scoring, disclosure, and generation rules.                                               |
| [crates/learning-data-access/](../crates/learning-data-access/) | Store contracts and PostgreSQL implementations, including transaction and row-security context.                           |
| [crates/server/](../crates/server/)                             | Axum HTTP routes, authenticated request composition, authorization, and server-only service composition.                  |
| [crates/grading/](../crates/grading/)                           | Answer-bearing grading decisions; it is not a browser dependency.                                                         |
| [crates/adapters/](../crates/adapters/)                         | PLE, WeBWorK, iMathAS, QTI, and H5P integration boundaries behind typed Question operations.                              |
| [crates/objects/](../crates/objects/)                           | Typed object addresses, integrity metadata, image validation, and object-store backends.                                  |
| [crates/browser-api-contract/](../crates/browser-api-contract/) | Rust declarations used to generate browser-facing TypeScript contracts.                                                   |
| [src/](../src/)                                                 | SolidJS application, strict HTTP decoders, answer-free presentation, routes, and UI features.                             |
| [local_stack_control/](../local_stack_control/)                 | Disposable-stack lifecycle and connected acceptance orchestration.                                                        |

The Rust workspace root is [Cargo.toml](../Cargo.toml). Browser dependencies and the TypeScript
toolchain are declared in [package.json](../package.json). Generated TypeScript declarations are
derived from Rust by [crates/project-tools/src/tsgen.rs](../crates/project-tools/src/tsgen.rs);
authored decoders in [src/api/decoders/](../src/api/decoders/) validate responses at runtime.

## Database lifecycle

[schemas/base_schema/](../schemas/base_schema/) is the canonical PostgreSQL structure for a fresh
database. Its short [install.sql](../schemas/base_schema/install.sql) manifest includes domain
modules in dependency order. Each module owns the current tables, constraints, indexes, functions,
row-security policies, and grants for its domain. Cross-domain relationships live in
[cross_domain_constraints.sql](../schemas/base_schema/cross_domain_constraints.sql), rather than in
a corrective layer.

Before the first human-approved production deployment, a structural correction updates its owning
base module and is verified with a fresh database. The manifest contains DDL only. PostgreSQL 17
`psql` installs it in one transaction through the `cargo tools database initialize` coordinator.
The coordinator also provides `migrate` for SQLx forward changes and application-role `verify`.
[schemas/migrations/](../schemas/migrations/) currently contains only its directory marker; it is
reserved for timestamped, immutable forward migrations after the first
human-approved production deployment freezes the base. The rule is maintained
in [schemas/base_schema/README.md](../schemas/base_schema/README.md).

`cargo tools database initialize` creates only the schema. The separate,
convergent `schemas/installation_data/` phase then supplies product data.
`cargo tools installation-data provision` includes the complete Live Demo by
default: it runs the Pilot publication and database-owned graph, then creates
cross-system Student Work and grading effects through their owning product
paths. `--without-live-demo` skips this phase; `apply` is only the narrower
convergent SQL/Pilot graph. Neither creates a demo-only model: Accounts,
Blueprint Revision, Course, roster, released Assignment, and exact
Question Revision pins are ordinary product records.

## Content and revision model

Question and Blueprint reusable content use immutable Revisions. A Question
lineage stores current availability, while each immutable Question Revision
preserves its published content. A complete Blueprint creation atomically
creates Available Revision 1; each changed explicit Save from its current
Revision creates a successor. Blueprint browser edits remain protected local
working state until Save.

Question IDs use the seven-character compact storage form. The familiar `AAA-BBBB` spelling is
presentation-only; the first six Crockford Base32 characters are random and the final character is
validated with server-held HMAC-SHA-256 material. The detailed format and issuance boundary are in
[QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md).

Course Instance terms and Assignment configuration express current state. An Assignment has one
stable identity, status, qualified Assignment Edit Number, authored policy, entries, and Question
Pool Items. An entry or pool item pins the exact Question Revision selected for it; later Question
publication does not advance that pin. `BlueprintAssignmentSource` records exact Blueprint
provenance without creating another Revision family.

## Assessment data flow

```text
published Question Revision
  -> current Course Assignment pins exact revision
  -> authorized Student starts or resumes an Assignment Attempt
  -> Attempt and Issued Question retain effective policy, revision, seed, score, and source facts
  -> saved response and submission records retain Student Work
  -> server-only grader produces result and statistics observations
  -> answer-free history, feedback, and Gradebook readers expose allowed views
```

An existing Attempt is interpreted from its retained Attempt and Issued Question evidence, not from
later mutable Assignment state. A later released Assignment save is accepted when current release
validation passes; later Attempts use the accepted state.

## External Question Backends

Question Type is immutable author-declared educational metadata on each
Published Question Revision. An author selects it while binding a Draft
Question Source; publication retains it for discovery and presentation. A
Question Backend owns the controls and interaction structure it uses for that
type, so PLE never infers Question Type from backend HTML or response fields.

WeBWorK uses the shared external-backend lifecycle:

```text
immutable PG source + Question Seed
  -> server-only WebworkAdapter -> private renderer JSON request
  -> immutable backend document on the Question Attempt
  -> authorized same-origin document route -> sandboxed backend-owned iframe
  -> ordered opaque form pairs -> generic saved response/finalization
  -> server-only stateless renderer grade request -> normalized credit
```

The adapter sends source, path, seed, and opaque response bytes only to the
renderer. It renders once at issuance and grades once after submission. E1 is
stateless: WeBWorK has no replay record, renderer cache, or per-attempt backend
state in PLE. The browser bridge preserves ordered duplicate form names and
ordinary hidden PG answer fields without recognizing controls; renderer
credentials never enter the embed document. PLE provides only an accessible
document baseline, while WeBWorK and Question-authored CSS own the document's
structure and presentation.

The server exposes the retained document only after Student ownership and
issued-position authorization. It gives the document `no-store`, CSP, and
same-origin CORP headers. The separate public asset proxy serves only validated
`webwork2_files` and `pg_files` paths from the private renderer, with no caller
supplied origin, redirect following, query string, or arbitrary media type.

Assignment Unrelease is a database-owned operation. It checks current Teaching Team authority,
the Assignment Edit Number, and a title confirmation; changes the Assignment to Unreleased; removes
the Student Work closure rooted at that Assignment; rebuilds affected statistics; and records a
redacted audit event atomically. The Store and HTTP routes expose impact counts without exposing
Student records unnecessarily.

## Authorization and runtime boundaries

PostgreSQL is authoritative for relationships, transactions, row-level security, role grants,
concurrency checks, and destructive-operation closure. Runtime services verify and use the installed
schema; application credentials do not own DDL. The database coordinator uses the migration role
only for initialization and forward migration, while `database verify` reads the restricted
application projection.

The server resolves authenticated sessions and Teaching Team or Student relationships before Store
operations. Browser contracts are closed and decoded strictly. The browser does not receive Answer
Keys, grading inputs, private Question Sources, backend credentials, or broad database authority.

One server-owned expiry worker runs the generic Attempt evaluator and a
60-second expiry sweep. It invokes the ordinary finalization boundary for an
expired Attempt, reads immutable source only through S3 and the private
WeBWorK renderer boundary, and has no public grading state or UI contract.
iMathAS retains its separate session and receipt boundary. PostgreSQL access
remains inside `learning-data-access`, so the server crate does not own a
database driver.

## Browser and service flow

```text
SolidJS route
  -> same-origin HTTP client and strict decoder
  -> Axum route and authenticated session
  -> Store contract and PostgreSQL RLS/function boundary
  -> typed answer-free response
  -> browser state and accessible presentation
```

[src/features/blueprint_course/](../src/features/blueprint_course/) owns the Blueprint Revision
editor. [src/pages/assignment_workspace/](../src/pages/assignment_workspace/) owns instructor
Assignment editing and release surfaces. Student delivery and retained presentation flow through
[src/pages/assignment_attempt_page.tsx](../src/pages/assignment_attempt_page.tsx) and related
components. Shared navigation and capability admission live in [src/ribbon/](../src/ribbon/).
For backend-owned delivery,
[crates/server/src/webwork_document_route.rs](../crates/server/src/webwork_document_route.rs)
serves the retained document,
[crates/server/src/webwork_asset_proxy.rs](../crates/server/src/webwork_asset_proxy.rs)
serves its bounded assets, and
[src/components/question_response_controls/backend_owned_document.tsx](../src/components/question_response_controls/backend_owned_document.tsx)
hosts it and captures the generic response.

## Verification boundaries

Fast deterministic checks live in [tests/](../tests/). Rust checks validate the workspace;
TypeScript checks validate the browser code and decoders. Database and service acceptance live in
[tests/e2e/](../tests/e2e/) and use a disposable PostgreSQL stack. Browser journeys and screenshots
are separate rendered evidence, coordinated by [local_stack_control/](../local_stack_control/).

The commands in [README.md](../README.md) are the contributor entry points. Passing a focused
check establishes only that check's stated boundary. [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md)
distinguishes permanent regression tests from connected and visual evidence.

## Extension points

- Add structural DDL to the owning [schemas/base_schema/](../schemas/base_schema/) module while the
  pre-production baseline remains editable.
- Add a timestamped SQLx migration in [schemas/migrations/](../schemas/migrations/) only after the
  first human-approved production deployment freezes the base.
- Add reusable domain types in [crates/question_model/](../crates/question_model/) or
  [crates/domain/](../crates/domain/) before adapting them to storage or HTTP.
- Add PostgreSQL access through [crates/learning-data-access/](../crates/learning-data-access/) and
  expose it through a bounded route in [crates/server/](../crates/server/).
- Add browser DTO decoding in [src/api/decoders/](../src/api/decoders/) alongside the HTTP client
  and feature that consume it.
- Add ordinary installation data in `schemas/installation_data/` only
  when PostgreSQL owns the complete data invariant.

## Evidence scope

- Connected fresh-install, repeated installation-data, and full browser-journey acceptance remain
  separate gates; this document does not claim their completion.
