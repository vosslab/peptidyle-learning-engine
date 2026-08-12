# Code architecture

## Overview

Peptidyle Learning Engine (PLE) is a question-agnostic learning platform. A
SolidJS browser client renders server-issued question presentations; a Rust API
owns identity, authorization, attempts, grading, and durable records. Focused
Rust crates keep question generation, answer-bearing grading, persistence, and
external engines separate.

This is the high-level map. [FILE_STRUCTURE.md](FILE_STRUCTURE.md) maps paths,
[CONTRACTS.md](CONTRACTS.md) indexes durable contracts, and
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) explains their rationale. Current
release state belongs in [active_plans/](active_plans/), not in this document.

## System shape

```text
browser
  SolidJS application + domain WebAssembly
        |
        | same-origin requests and browser-safe JSON
        v
gateway --> Rust API --> PostgreSQL
                 |          |
                 |          `-> tenant-owned learning records and jobs
                 v
             object store
                 |
                 `-> immutable content and protected artifacts

Rust API --> private external question engines
              `-> standalone webwork-pg-renderer
```

The browser is a presentation and interaction client. It can validate format,
display a timer, and provide accessible controls, but it does not receive
answer keys or make authoritative grading or timing decisions.

## Core guarantees

- **Server-owned assessment authority.** Answer keys, correctness, timing
  verdicts, feedback disclosure, completion, and grade changes remain in Rust
  server code and durable storage. [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md)
  and [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) define this
  boundary.
- **Compile-time answer separation.** `crates/grading/` is outside the
  dependency closure of `crates/wasm/`, so the browser bridge cannot import the
  grading implementation. [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md)
  defines adapter responsibilities.
- **Course-scoped disclosure.** Immutable published content is distinct from
  tenant-owned courses, memberships, enrollments, runs, and grades. PostgreSQL
  forces row-level security for tenant-owned records. See
  [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) and
  [DATABASE_TENANCY.md](DATABASE_TENANCY.md).
- **Attempt-bound presentations.** The server issues a browser-safe question
  envelope with a presentation nonce, digest, and rendered-item identities.
  The submitted response identifies the issued presentation rather than a
  durable answer key. See [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).
- **Replaceable infrastructure.** Domain code depends on models, while storage,
  object, identity, and question-engine implementations sit behind focused
  interfaces and the server composition root.

## Major components

| Component                | Location                                                                                  | Responsibility                                                                                                                |
| ------------------------ | ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Question model           | [crates/question_model/](../crates/question_model/)                                       | Question taxonomy, identifiers, capabilities, browser-safe presentation values, and response schemas.                         |
| Domain                   | [crates/domain/](../crates/domain/)                                                       | Run state, policy evaluation, seeded generation rules, timing inputs, and validation without a database or wall clock.        |
| Grading                  | [crates/grading/](../crates/grading/)                                                     | Answer-bearing checkers and correctness decisions; server-side only.                                                          |
| Native adapter           | [crates/adapters/native/](../crates/adapters/native/)                                     | First-party generated questions and static flat-question compilation.                                                         |
| External adapters        | [crates/adapters/](../crates/adapters/)                                                   | Bounded QTI, H5P, iMathAS, and WeBWorK boundaries with declared capabilities.                                                 |
| Learning data access     | [crates/learning-data-access/](../crates/learning-data-access/)                           | Store contracts plus in-memory and PostgreSQL implementations, migrations, RLS context, and conformance coverage.             |
| Object storage           | [crates/objects/](../crates/objects/)                                                     | Typed immutable object records and S3-compatible backends.                                                                    |
| Server                   | [crates/server/](../crates/server/)                                                       | Axum routes, authentication, authorization, adapter selection, worker composition, and browser DTOs.                          |
| Browser and WebAssembly  | [src/](../src/) and [crates/wasm/](../crates/wasm/)                                       | SolidJS interaction layer and an answer-free browser bridge to shared domain logic.                                           |
| Export and project tools | [crates/export/](../crates/export/) and [crates/project-tools/](../crates/project-tools/) | Print/export generation and repository-only code generation, migration, fixture, pilot-content validation, and seed commands. |

Each Cargo crate declares its permitted dependencies explicitly. In particular,
`crates/domain/` depends only on the question model, and the server is the
composition root that chooses concrete adapters and stores.

## Assessment flow

The normal automatic-grading path is an ownership transition, not a browser
calculation:

```text
author draft
  -> immutable published problem version
  -> course assignment references that version
  -> authorized learner starts or resumes a run
  -> server issues one attempt-bound public presentation
  -> browser submits a compact response plus request identity
  -> server validates, grades, and persists the authoritative result
  -> server returns only feedback allowed by assignment policy
```

`crates/server/src/run/` coordinates issue, prefetch, submission, manual
grading, and external-tool paths. `crates/learning-data-access/` commits run,
submission, score, and summary changes transactionally. The browser decodes
the response schema and selects the matching response widget; it does not
choose a grading backend or derive a correct response.

The flat-question adapter supports the PLE JSON authoring model, including
single and multiple choice, fill-in, numeric, multi-blank, matching, ordering,
and hotspot questions. [INPUT_FORMATS.md](INPUT_FORMATS.md) and
[QUESTION_MODEL.md](QUESTION_MODEL.md) define supported authoring and response
models. File-upload responses remain fail-closed until the server can issue a
tenant-, learner-, and attempt-bound upload capability.

## External question engines

Adapters own the boundary between PLE and each question source. They normalize
the source into PLE's question and grading contracts, while PLE continues to
own assignments, attempts, authorization, records, and feedback policy.

The WeBWorK integration calls the separate, stateless
`webwork-pg-renderer` service over a private network. It is a PG/PGML render
and grade engine, not a second assignment platform. The adapter accepts only
reviewed upstream response shapes, projects browser-safe HTML, and keeps source
and grading calls on the server. [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md)
documents the integration.

The reviewed first-release content is a repository-owned input to those same production
boundaries, not a browser fixture. `cargo tools pilot-content` validates its human-readable
two-chapter manifest and source/compiler contracts. The host-only `e2e-seed --chapter-one-pilot`
command publishes immutable PGML and flat source objects, protected flat grading material,
catalog versions, courses, four-item assignments, and roster-derived enrollments. Its JSON output
uses `P-...-v1` as the display identity and retains UUIDs only for machine routing.

The three WeBWorK reference trees have distinct purposes:

- `OTHER_REPOS/pg/` is reference material for the PG/PGML engine.
- `OTHER_REPOS/webwork-pg-renderer/` is a reference snapshot of the maintained
  standalone renderer project.
- `OTHER_REPOS/webwork2/` is reference material for the full WeBWorK course
  application.

`OTHER_REPOS/` is comparison material only. The local stack uses the external
renderer image named in its environment, not a build context or mounted copy
from one of these snapshots.

## Identity, courses, and authorization

PLE accounts use opaque `UserId` values. Email authentication establishes or
recovers account access, while passkeys provide an additional browser-bound
authentication method. Email and display information are mutable account or
course-roster attributes; they are not database primary keys. An invitation can
be claimed only after authentication, and course membership then provides the
instructor or learner access appropriate to that course.

`crates/server/src/auth/` owns passwordless email and passkey HTTP behavior.
`crates/server/src/course/` owns course access, roster, invitation, and
assignment routes. [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md) and
[ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md) define the account and roster
contract; [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) defines the
non-disclosure and course-scoping rules.

## Storage and background work

The data-access crate exposes behavior-level Store contracts. Its in-memory
implementation supports fast behavior and conformance tests; its PostgreSQL
implementation uses transactions, explicit tenant context, and schema-owned
constraints for durable production records.

PostgreSQL stores relationships, policy-bearing records, sessions, attempts,
submissions, summaries, jobs, and audit events. The object store holds immutable
content, protected learner artifacts, exports, and temporary processing bytes.
Object records include typed keys and checksums, so an HTTP request cannot turn
arbitrary text into an object-store path. See [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md),
[OBJECT_STORAGE.md](OBJECT_STORAGE.md), and [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md).

The worker claims durable jobs. It performs background work such as exports,
imports, retention, scoring maintenance, and item analysis. A worker restart
does not erase the job record or educational result because both remain in
shared storage.

## Browser architecture

The browser application in [src/](../src/) is organized by API, authentication,
reusable components, capabilities, pages, and the shared WebAssembly facade.
Generated TypeScript contracts are decoded at the API edge before they enter
Solid components. Response widgets use native controls first and add the PLE
keyboard extensions described in [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md).

`crates/wasm/` exposes answer-free domain helpers for deterministic formatting,
parameter work, response validation, and timer support. The API remains
authoritative whenever the browser and server could disagree.

## Local stack

[launch_local_stack.sh](../launch_local_stack.sh) builds and starts the
developer stack described by [containers/compose.yaml](../containers/compose.yaml).
The long-running services are gateway, API, worker, PostgreSQL, MinIO, and the
private standalone PG renderer. One-shot services provision buckets and
runtime-readable secrets. PostgreSQL and MinIO keep durable state in named
volumes; API, worker, gateway, and renderer containers are replaceable.

The gateway is the only browser entry point. The API joins the gateway, data,
and renderer networks; the renderer has no host port and no SQL database.
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md) explains service
roles, while [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) documents
startup, rebuild, health, and shutdown.

## Testing and verification

- [check_codebase.sh](../check_codebase.sh) runs the repository's TypeScript,
  Rust formatting, lint, and test gates.
- [tests/](../tests/) contains fast repository and Node behavior checks.
- Rust unit and integration tests live beside the crates they exercise.
- [tests/playwright/](../tests/playwright/) exercises built browser behavior and
  accessibility over HTTP.
- [tests/e2e/](../tests/e2e/) holds slower disposable PostgreSQL, replica,
  WebAssembly, local-stack, and exact Chapter 1 publication evidence.
- [tests/walkthrough/](../tests/walkthrough/) owns the teaching-loop runner,
  fixed child processes, and importable `walklib/` configuration, contracts,
  subprocess, and lifecycle behavior. The historical E2E paths are thin
  compatibility launchers. Its default run ends after the instructor-created Genetics assignment
  and focused J1-J5/J8 student journey; the isolated Chapter 1 release gate owns the all-eight
  browser sweep. Browser journeys remain independently readable under `tests/playwright/`.
- Learning data-access capabilities use a contract, an in-memory implementation,
  a PostgreSQL implementation, and conformance coverage where both backends
  should agree.

[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) distinguishes permanent tests,
one-time acceptance evidence, and human review. A passing plan or document is
not evidence that an unrun deployment path works.

## Extension points

- Add question behavior behind the appropriate adapter and declare its
  capabilities in the question model.
- Add persistence behavior as a Store contract plus matching in-memory,
  PostgreSQL, and conformance owners when the behavior is shared.
- Add HTTP behavior to its server route module; keep
  [crates/server/src/composition.rs](../crates/server/src/composition.rs) for
  dependency assembly.
- Add browser behavior in an owning API, feature, page, or component module;
  use generated contracts instead of a parallel handwritten wire model.
- Add a WeBWorK response family only after defining its safe PLE projection,
  grading handoff, and accessible browser interaction.
- Add forward-only schema changes under [schemas/migrations/](../schemas/migrations/)
  and validate them with the database tooling and relevant live oracle.

## Known gaps

- Verify each production deployment's SMTP provider, DNS configuration, secret
  rotation, backup, recovery, and monitoring against its operator environment;
  the repository provides integration boundaries, not hosted-provider accounts.
- Verify every newly supported external question shape against its actual
  renderer or import source before claiming compatibility beyond the reviewed
  adapter contract.
