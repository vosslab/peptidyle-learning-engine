# File structure

This map gives contributors the shortest route to the owner of a behavior.
[CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) explains how the pieces cooperate;
[CONTRACTS.md](CONTRACTS.md) indexes the durable rules they must preserve.
Release plans and dated status reports live under [active_plans/](active_plans/)
and are intentionally separate from this file map.

## Top-level layout

```text
.
+- crates/              Rust product crates and repository tools
+- src/                 SolidJS and TypeScript browser application
+- schemas/             PostgreSQL migrations
+- containers/          Local Compose configuration and container build files
+- content/             Checked-in teaching content and bounded pilot material
+- pipeline/            Browser and WebAssembly build steps
+- tests/               Hygiene, Node, Playwright, and E2E checks
+- docs/                Durable documentation and active planning artifacts
+- devel/               Focused developer-maintenance commands
+- tools/               Focused repository utilities
+- generated/           Ignored generated contract and fixture projections
+- dist/                Ignored built browser application
+- dist_wasm/           Ignored built WebAssembly output
+- Cargo.toml           Rust workspace manifest
+- package.json         Browser tooling manifest
+- build.sh             Full local build entry point
+- check_codebase.sh    Main lint, formatting, and test gate
+- launch_local_stack.sh Local Podman stack launcher
`- run_playwright_tests.sh Browser test entry point
```

`OTHER_REPOS/` contains reference snapshots. It is not a source import path,
container build context, or runtime dependency.

## Rust workspace

| Path                                                            | Owns                                                                                                  |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| [crates/question_model/](../crates/question_model/)             | Question types, capabilities, identities, and public presentation schemas.                            |
| [crates/domain/](../crates/domain/)                             | Attempt state, policies, seeded generation, timing inputs, and answer-free validation.                |
| [crates/grading/](../crates/grading/)                           | Answer keys, checkers, and correctness decisions.                                                     |
| [crates/objects/](../crates/objects/)                           | Typed object-store interface, key construction, checksums, and S3-compatible backends.                |
| [crates/learning-data-access/](../crates/learning-data-access/) | Store contracts, in-memory and PostgreSQL implementations, RLS, migrations, and conformance coverage. |
| [crates/adapters/native/](../crates/adapters/native/)           | First-party generated questions and flat-question source compilation.                                 |
| [crates/adapters/webwork/](../crates/adapters/webwork/)         | Private standalone-renderer protocol, safe projection, cache, and grading delegation.                 |
| [crates/adapters/qti/](../crates/adapters/qti/)                 | Bounded QTI parsing, profile mapping, and private grading handoff.                                    |
| [crates/adapters/h5p/](../crates/adapters/h5p/)                 | H5P practice import and capability declaration.                                                       |
| [crates/adapters/imathas/](../crates/adapters/imathas/)         | iMathAS provider and broker boundary.                                                                 |
| [crates/export/](../crates/export/)                             | Print model plus PDF and DOCX writers.                                                                |
| [crates/wasm/](../crates/wasm/)                                 | `wasm-bindgen` facade over answer-free domain behavior.                                               |
| [crates/server/](../crates/server/)                             | Axum API, auth, course and run routes, worker, and dependency composition.                            |
| [crates/project-tools/](../crates/project-tools/)               | Repository-only code generation, fixture, pilot-content validation, migration, and E2E seed commands. |

Cargo package directories use hyphens; Rust imports use underscores. For
example, the directory `learning-data-access` is imported as
`learning_data_access`.

## Learning data access

```text
crates/learning-data-access/
+- src/
|  +- contracts/       Store and capability contracts
|  +- in_memory/       Database-free capability implementations
|  +- postgres/        PostgreSQL capability implementations
|  +- course_roster/   Roster and invitation data behavior
|  +- session.rs       Authentication-session contract
|  +- account_identity.rs PLE account and identity contract
|  +- qti.rs           Private QTI registry and grader capability
|  +- retention.rs     Retention policy and record types
|  +- lib.rs           Public facade and stable re-exports
|  +- in_memory.rs     In-memory composition facade
|  `- postgres.rs      PostgreSQL composition facade
`- tests/
   +- conformance/     Backend-neutral behavior cases
   +- fixtures/        Small safe fixture evidence
   `- postgres_*_live.rs Disposable PostgreSQL acceptance gates
```

When a persistence capability changes, read its contract, both implementations,
and the matching conformance test. This structure keeps database details out of
domain, adapter, and browser code.

## Server application

```text
crates/server/src/
+- auth/                       Passwordless email, passkey, and session routes
+- catalog/                    Catalog query and publication behavior
+- course/                     Course, roster, invitation, and assignment routes
+- run/                        Attempt issue, prefetch, submission, and grading routes
+- workspace/                  Authoring workspace behavior
+- flat_question_publication/  Native publication routes and tests
+- qti_*/                      QTI import, conversion, publication, and runtime paths
+- course_appearance/          Course presentation and banner handling
+- worker/                     Durable background-job runtime
+- composition/                Concrete dependency assembly
+- asset.rs                    Authorized object delivery
+- webwork_backend/            PLE-owned PG source and replay integration
+- lib.rs                      Server library facade
`- main.rs                     API or worker process entry point
```

Route modules own HTTP behavior. `composition/` chooses concrete stores,
identity providers, object storage, and question adapters; it does not absorb
business behavior from the route modules.

## Browser application

```text
src/
+- api/             Strict decoders, HTTP client, generated contracts, and mocks
+- auth/            Account and course-session browser state
+- components/      Reusable prompt, response, feedback, and accessibility UI
+- features/        Capability-owned browser logic
+- pages/           Route-level views and page-specific state
+- wasm/            Shared domain WebAssembly facade and Solid context
+- app.tsx          Application shell
+- routes.ts        Route definitions
`- main.tsx         Browser entry point
```

`src/components/responses/` contains response-family controls, while
`src/components/response_widget/` contains shared keyboard and external-tool
extensions. `src/features/course_appearance/`,
`src/features/flat_question_authoring/`, and
`src/features/qti_profile_import/` keep those larger browser capabilities
separate from generic route pages.

## Question sources and engines

`content/` holds checked-in content. `content/pilot/chapter_1_assignments.yaml`
owns the reviewed Genetics and Biochemistry Chapter 1 inventory;
`content/pilot/sources/` retains licensed source evidence and
`content/pilot/flat/` contains the four curated PLE flat v2 payloads. The
earlier bounded PGML renderer fixture remains under `content/pilot/webwork/`.
`crates/project-tools/src/pilot_content.rs` validates that source corpus, while
`crates/project-tools/src/e2e_seed/chapter_one.rs` publishes its exact two-by-four matrix through
the production PostgreSQL and object-store contracts. The browser and PLE API never run source
from `OTHER_REPOS/`.

The WeBWorK reference material is deliberately divided:

- `OTHER_REPOS/pg/` documents the PG/PGML engine.
- `OTHER_REPOS/webwork-pg-renderer/` mirrors the separate renderer project
  that PLE calls as a service.
- `OTHER_REPOS/webwork2/` documents the full WeBWorK homework application,
  which PLE does not run.

The service image and probe configuration live under
[containers/webwork/](../containers/webwork/). The PLE adapter lives in
[crates/adapters/webwork/](../crates/adapters/webwork/), and PLE-specific
source/replay selection lives in [crates/server/src/webwork_backend/](../crates/server/src/webwork_backend/).

## Database and containers

```text
schemas/
`- migrations/       Ordered forward SQL migrations

containers/
+- compose.yaml       Normal local services and private networks
+- compose.smtp.yaml  Optional external SMTP-provider overlay
+- Containerfile.api  Shared API and worker image, built once by API
+- Containerfile.gateway Gateway image
+- Caddyfile          Same-origin browser and API gateway rules
+- env.example        Safe environment template
`- webwork/           Renderer semantic probe
```

The default local services are PostgreSQL, MinIO, API, worker, gateway, and a
private standalone renderer. PostgreSQL and MinIO use named volumes; the other
service containers can be rebuilt from configuration. See
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md) for service roles and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for commands.

## Tests and generated output

```text
tests/
+- test_*.py          Fast repository-policy and documentation checks
+- test_*.mjs         Deterministic browser-contract checks without a browser
+- playwright/        Built-browser interaction and accessibility tests
+- e2e/               Generic disposable whole-system runners
+- walkthrough/       Teaching-loop entry points and fixed child processes
|  `- walklib/        Importable runner configuration, contracts, and lifecycle
`- fixtures/          Small checked-in fixture evidence

generated/
+- api/               Generated TypeScript contracts
`- fixtures/          Generated fixture projections
```

`dist/`, `dist_wasm/`, `target/`, `test-results/`, and Playwright report
directories are reproducible ignored output. The checked-in fixtures under
`tests/fixtures/` remain source evidence and should be changed deliberately.

## Documentation map

- [README.md](../README.md) is the newcomer entry point and first verified
  workflow.
- [INSTALL.md](INSTALL.md), [USAGE.md](USAGE.md),
  [DEVELOPMENT.md](DEVELOPMENT.md), and [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
  describe operation and contributor workflows.
- [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) and [STUDENT_GUIDE.md](STUDENT_GUIDE.md)
  document the visible local no-email teaching loop with real-stack screenshots.
- [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md), [CONTRACTS.md](CONTRACTS.md),
  [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md), and
  [MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md) describe the
  learning model.
- [API_CONTRACTS.md](API_CONTRACTS.md),
  [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md), and
  [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md) describe data
  flow and backend boundaries.
- [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md),
  [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md),
  [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), and
  [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md) describe identity, roster,
  authorization, and data handling.
- [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md),
  [OBJECT_STORAGE.md](OBJECT_STORAGE.md),
  [CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md), and
  [FAILURE_RECOVERY.md](FAILURE_RECOVERY.md) describe durable operations.
- [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md) and
  [COLOR_CONTRAST_ACCESSIBILITY.md](COLOR_CONTRAST_ACCESSIBILITY.md) describe
  accessibility requirements.
- [active_plans/](active_plans/) contains in-flight scope, audits, decisions,
  workstreams, and dated reports.

## Where to add work

- Put a new domain rule in its focused module under `crates/domain/src/`.
- Put persistence behavior in a data-access contract plus its implementations
  and conformance coverage when both stores support it.
- Put a new API endpoint in its owning server capability module.
- Put browser behavior in the owning `src/api/`, `src/features/`,
  `src/components/`, or `src/pages/` module rather than a catch-all file.
- Put an adapter capability under `crates/adapters/` and keep grading outside
  the WebAssembly dependency closure.
- Put a forward database change in `schemas/migrations/`; preserve earlier
  applied migrations as history.
- Put a durable design or contract document in `docs/` and an in-flight plan or
  status artifact in the appropriate `docs/active_plans/` subdirectory.
