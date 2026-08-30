# File structure

This map points contributors to the owner of a behavior. [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md)
describes the boundaries; [CONTRACTS.md](CONTRACTS.md) indexes durable contracts;
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns PLE concept meaning and
authority relationships; [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) owns cross-language
spelling.

## Top-level layout

~~~text
.
+- crates/                 Rust product crates and repository tools
+- src/                    SolidJS browser application
+- schemas/migrations/     Forward PostgreSQL schema
+- generated/              Ignored generated TypeScript and fixture projections
+- content/                Checked-in teaching content and pilot material
+- containers/             Podman Compose and service images
+- deploy/opentofu/        AWS infrastructure and policy tests
+- tests/                  Offline, Node, Playwright, and disposable E2E checks
+- docs/                   Durable references and active plans
+- local_stack_control/    Typed local-stack and acceptance lifecycle
+- devel/                  Developer-maintenance commands
+- tools/                  Focused repository utilities
+- Cargo.toml              Rust workspace manifest
+- package.json            Browser tooling manifest
+- build.sh                Full local build entry point
+- check_rust.sh           Rust gate
+- check_codebase.sh       TypeScript and browser gate
+- run_live_demo.sh        Live-demo lifecycle front door
- run_playwright_tests.sh  Production-browser entry point
~~~

OTHER_REPOS/ contains reference snapshots only. It is not a runtime,
container, or source-import path.

## Rust workspace

| Path | Owns |
| --- | --- |
| [crates/question_model/](../crates/question_model/) | Question, identity, assignment, course-term, BlueprintCourse, adoption, and browser-safe contract types. |
| [crates/domain/](../crates/domain/) | Pure timing, policy, disclosure, run, scoring, generation, and validation. |
| [crates/grading/](../crates/grading/) | Answer-bearing checkers and correctness decisions; server-only. |
| [crates/learning-data-access/](../crates/learning-data-access/) | Store contracts, Memory conformance, PostgreSQL persistence, migrations, RLS, locks, and live oracles. |
| [crates/server/](../crates/server/) | Axum routes, authentication, authorization, worker composition, and API assembly. |
| [crates/objects/](../crates/objects/) | Typed object keys, checksums, image validation, and object-store backends. |
| [crates/adapters/](../crates/adapters/) | Native, QTI, H5P, iMathAS, and WeBWorK adapters. |
| [crates/wasm/](../crates/wasm/) | The answer-free Rust-to-browser WebAssembly facade. |
| [crates/export/](../crates/export/) | PDF/DOCX export models and writers. |
| [crates/project-tools/](../crates/project-tools/) | TypeScript generation, fixtures, migrations, pilot content, and E2E seed tooling. |
| crates/base-course-installation/ | Base Course request, receipt, recipe, and deterministic installation orchestration. |
| crates/acceptance-runtime/ | Disposable acceptance manifests and capability-specific database URL handoff. |

Package directories use hyphens; Rust module imports use underscores.

## Canonical course paths

The reusable and delivery aggregates have separate paths:

~~~text
crates/question_model/src/
+- reusable_curriculum.rs       BlueprintCourse tree and projections
+- curriculum_adoption.rs       Source, target, preview, apply, and receipt contracts
`- curriculum_adoption/          Focused adoption contract modules

crates/learning-data-access/src/
+- contracts/reusable_curriculum.rs       ReusableCurriculumStore
+- contracts/curriculum_adoption.rs       CurriculumAdoptionStore
+- in_memory/reusable_curriculum.rs       Deterministic BlueprintCourse adapter
+- in_memory/curriculum_adoption/         Adoption conformance adapter
+- postgres/reusable_curriculum.rs        PostgreSQL BlueprintCourse adapter
`- postgres/curriculum_adoption/         PostgreSQL adoption and bridge modules
~~~

BlueprintCourse is one ordered module/assignment tree with one aggregate
revision. Its exact public question members resolve to immutable
ProblemVersionRef pins. CourseInstance is not another source tree: the
adoption boundary materializes it under an exact CourseId, records the
immutable Blueprint parent and applied revision, and owns private delivery
state. New upstream assignments appear in daughter instances as unreleased.

The current paired legacy files and SQL table families are SD1 migration inputs
only. The immutable
schemas/migrations/2026081837_blueprint_alpha_curriculum.sql and accepted
successors remain historical evidence and are not renamed or edited to hide
their origin. The fresh SD1 migration epoch is tracked in
[active_plans/implementation_status.md](active_plans/implementation_status.md);
its course/curriculum range is planned at 2026082913-2026082916, with
broker/RLS/grant helpers at 2026082929-2026082932.

## Learning data access

~~~text
crates/learning-data-access/
+- src/
|  +- contracts/       Store and capability contracts
|  |  +- reusable_curriculum.rs  One BlueprintCourse Store contract
|  |  `- curriculum_adoption.rs  Separate source-to-instance operations
|  +- in_memory/       Test-support-gated deterministic adapters
|  |  +- reusable_curriculum.rs
|  |  `- curriculum_adoption/
|  |     +- state.rs, course_structure.rs, destination.rs
|  |     `- receipt_evidence.rs
|  +- postgres/        Production PostgreSQL adapters
|  |  +- reusable_curriculum.rs
|  |  `- curriculum_adoption/
|  |     `- bridge/
|  - lib.rs, in_memory.rs, postgres.rs
- tests/              Conformance and disposable PostgreSQL suites
~~~

The reusable Store owns BlueprintCourse list, get, replacement, publication
projection, and permitted deletion. The adoption Store owns fork, assignment
instantiation, whole-course instantiation, rollover, term shift, controlled
update, idempotency, receipts, and reconciliation. It never grants a public
Blueprint reader access to a private CourseInstance.

## Server application

~~~text
crates/server/src/
+- auth/                     Account, session, passkey, and preflight behavior
+- course/                   Course, membership, Student, assignment, and Gradebook routes
+- catalog/                  Shared published-question discovery and publication
+- reusable_curriculum.rs    BlueprintCourse HTTP route family
`- curriculum_adoption/     CourseInstance adoption, rollover, and update routes
+- route_policy.rs           Method and route security policy
+- composition/              Concrete Store, database, worker, object, and adapter assembly
+- run/                      Attempt, submission, disclosure, and external-tool routes
+- worker/                   Generic durable-job runtime
+- accepted_submission_worker.rs  Sealed private grading execution
+- public_asset_publication_worker/  Dedicated public-asset publisher
`- main.rs                   API, worker, or publisher process entry point
~~~

Authentication and approved-Instructor or course-membership preflight precede
reference, revision, query, and body decoding. CourseInstance routes require
the exact destination course and current equal co-Instructor authority.

## Browser application

~~~text
src/
+- api/
|  +- reusable_curriculum.ts                 BlueprintCourse client contract
|  +- curriculum_adoption.ts                 Adoption client contract
|  +- http_client/reusable_curriculum.ts     Same-origin BlueprintCourse requests
|  +- http_client/curriculum_adoption.ts     Preview/apply/receipt requests
|  +- decoders/reusable_curriculum.ts        Strict BlueprintCourse DTO decoder
|  `- decoders/curriculum_adoption/         Strict adoption DTO decoders
+- features/
|  +- reusable_curriculum/                    One BlueprintCourse workspace/editor
|  `- curriculum_adoption/                  Destination-specific staged workflow
+- pages/
|  +- curriculum_route_page.tsx               Workspace route composition
|  +- curriculum_detail_route_page.tsx        Detail route composition
|  `- curriculum_adoption_live_page.tsx     CourseInstance adoption route
+- components/                                Shared answer-free and accessibility UI
+`- routes.ts                                Executable route map
~~~

The intended browser workspace has one BlueprintCourse list, detail, editor,
and nested module/assignment picker. It presents draft owner/collaborator
states and the vetted-Instructor published projection without a second product
branch. Adoption selects an operation and destination: one assignment into an
existing CourseInstance, a whole BlueprintCourse into a new CourseInstance, or
an explicit fork into a new BlueprintCourse.

## Generated contracts

crates/project-tools/src/tsgen.rs generates TypeScript from Rust contract
roots into ignored generated/api/. cargo tools tsgen and build.sh are the
generation entry points. Generated modules are derivative and must not be
hand-edited. Rust Serde owns field spelling and closed DTO shape; authored
decoders under src/api/decoders/ enforce runtime strictness.

Legacy paired generated names are retained only in the SD1 migration inventory
until regeneration after the Rust contract cutover. No client may accept an old
reference or route as a compatibility alias.

## Content, storage, and deployment

content/ holds reviewed teaching content. crates/project-tools/src/pilot_content.rs
validates it and crates/project-tools/src/e2e_seed/ publishes bounded fixtures
through production contracts. Adapters under crates/adapters/ keep source
format and provider behavior behind typed capabilities.

~~~text
schemas/migrations/          Ordered forward SQL; accepted files are immutable
containers/                  Compose, API/gateway images, private renderer
deploy/opentofu/             AWS network, compute, database, storage, IAM, and policy
crates/objects/              Typed public-assets/private-content/student-records/temp-processing
~~~

PostgreSQL stores policy-bearing relationships, BlueprintCourse and
CourseInstance records, attempts, submissions, summaries, jobs, and audit
events. Object storage holds bounded source and binary bytes. The production
API, workers, and publisher use separate capability profiles.

## Tests and generated output

~~~text
tests/
+- test_*.py                  Fast deterministic repository-policy checks
+- test_*.mjs                 Browser-contract and model checks without a browser
+- playwright/e2e/*.spec.ts   Production HTTPS browser journeys
+- e2e/                       Disposable PostgreSQL, service, lifecycle, and publication checks
`- fixtures/                  Small durable fixture evidence

generated/
+- api/                       Ignored Rust-derived TypeScript
`- fixtures/                 Ignored generated fixture projections
~~~

Permanent tests protect behavior that can regress: tree ordering, exact pins,
authorization, strict decoding, adoption exclusions, unreleased propagation,
and answer-free projections. Graphify and source/migration inventories are
one-time evidence. PostgreSQL, browser, process, migration, and rendered visual
checks stay in their named E2E or human-review lanes. See
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

Build output such as dist/, dist_wasm/, target/, and test-results/ is reproducible
ignored state. Committed screenshots under docs/screenshots/ are published
evidence, not source contracts; their manifest is tests/e2e/browser_screenshot_corpus.json.

## Documentation map

- [README.md](../README.md): newcomer entry point and first workflow.
- [INSTALL.md](INSTALL.md), [USAGE.md](USAGE.md), [DEVELOPMENT.md](DEVELOPMENT.md),
  [TROUBLESHOOTING.md](TROUBLESHOOTING.md): operation and contribution.
- [CONTRACTS.md](CONTRACTS.md), [API_CONTRACTS.md](API_CONTRACTS.md),
  [SECURITY_MODEL.md](SECURITY_MODEL.md), [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md):
  durable API, security, and database rules.
- [LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md) and
  [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md): local production-shaped stack.
- [active_plans/](active_plans/): active scope, dependency order, audits, and status.
- [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md): evidence classes and required gates.

## Where to add work

- Add a reusable content rule to crates/question_model/src/reusable_curriculum.rs;
  update both reusable Store implementations and conformance cases.
- Add source-to-instance behavior to curriculum_adoption with a typed preview,
  command, authorization, and immutable receipt.
- Add schema only through the status-owned allocation in
  active_plans/implementation_status.md; preserve applied migrations.
- Add routes in the owning server module and register method policy in
  route_policy.rs.
- Regenerate generated/api/, then update strict decoders and typed clients.
- Add visible behavior to the owning feature/page/component; keep delivery and
  FERPA decisions server-authoritative.
- Add operational lifecycle behavior to local_stack_control/ and disposable
  evidence to its closed owner policy.
