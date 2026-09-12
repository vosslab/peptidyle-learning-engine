# Code architecture

PLE is a pre-production teaching platform. Its core boundary is between reusable published
content and Course-owned teaching records: the browser receives answer-free views, while PostgreSQL
and server-side services retain authorization, answer keys, grading inputs, and Student Work.
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product authority and
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines the implementation vocabulary.

## Major components

| Component | Ownership |
| --- | --- |
| [crates/question_model/](../crates/question_model/) | Shared typed identifiers, Question and Blueprint contracts, current Assignment state, and retained Student Work evidence. |
| [crates/domain/](../crates/domain/) | Pure policy, timing, validation, scoring, disclosure, and generation rules. |
| [crates/learning-data-access/](../crates/learning-data-access/) | Store contracts and PostgreSQL implementations, including transaction and row-security context. |
| [crates/server/](../crates/server/) | Axum HTTP routes, authenticated request composition, authorization, and server-only service composition. |
| [crates/grading/](../crates/grading/) | Answer-bearing grading decisions; it is not a browser dependency. |
| [crates/adapters/](../crates/adapters/) | PLE, WeBWorK, iMathAS, QTI, and H5P integration boundaries behind typed Question operations. |
| [crates/objects/](../crates/objects/) | Typed object addresses, integrity metadata, image validation, and object-store backends. |
| [crates/browser-api-contract/](../crates/browser-api-contract/) | Rust declarations used to generate browser-facing TypeScript contracts. |
| [src/](../src/) | SolidJS application, strict HTTP decoders, answer-free presentation, routes, and UI features. |
| [local_stack_control/](../local_stack_control/) | Disposable-stack lifecycle and connected acceptance orchestration. |

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
Blueprint Draft and Revision, Course, roster, released Assignment, and exact
Question Revision pins are ordinary product records.

## Content and revision model

Only published Question content and published Blueprint content use immutable Revisions. A
Question lineage stores current availability, while each immutable Question Revision preserves its
published content. A Blueprint lineage has current availability and a private mutable Blueprint
Draft with its own Edit Number. Explicit publication copies that Draft into a new immutable Blueprint
Revision.

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

## Browser and service flow

```text
SolidJS route
  -> same-origin HTTP client and strict decoder
  -> Axum route and authenticated session
  -> Store contract and PostgreSQL RLS/function boundary
  -> typed answer-free response
  -> browser state and accessible presentation
```

[src/features/blueprint_course/](../src/features/blueprint_course/) owns the Blueprint Draft
workspace. [src/pages/assignment_workspace/](../src/pages/assignment_workspace/) owns instructor
Assignment editing and release surfaces. Student delivery and retained presentation flow through
[src/pages/assignment_attempt_page.tsx](../src/pages/assignment_attempt_page.tsx) and related
components. Shared navigation and capability admission live in [src/ribbon/](../src/ribbon/).

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
