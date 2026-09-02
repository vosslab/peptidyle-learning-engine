# File structure

This map points contributors to the owner of a behavior. [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md)
describes the boundaries; [CONTRACTS.md](CONTRACTS.md) indexes durable contracts;
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns PLE concept meaning and
authority relationships; [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) owns cross-language
spelling.

## Top-level layout

```text
.
+- crates/                 Rust product crates and repository tools
+- src/                    SolidJS browser application
+- schemas/migrations/     Forward PostgreSQL schema
+- generated/              Ignored generated TypeScript declarations and fixtures
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
```

OTHER_REPOS/ contains reference snapshots only. It is not a runtime,
container, or source-import path.

## Rust workspace

| Path                                                            | Owns                                                                                                     |
| --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| [crates/question_model/](../crates/question_model/)             | Question, identity, assignment, course-term, BlueprintCourse, adoption, and browser-safe contract types. |
| [crates/domain/](../crates/domain/)                             | Pure timing, policy, disclosure, Assignment Attempt, scoring, generation, and validation.                |
| [crates/grading/](../crates/grading/)                           | Answer-bearing checkers and correctness decisions; server-only.                                          |
| [crates/learning-data-access/](../crates/learning-data-access/) | Store contracts, Memory conformance, PostgreSQL persistence, migrations, RLS, locks, and live oracles.   |
| [crates/server/](../crates/server/)                             | Axum routes, authentication, authorization, worker composition, and API assembly.                        |
| [crates/objects/](../crates/objects/)                           | Typed Object Addresses, checksums, image validation, and object-store backends.                          |
| [crates/adapters/](../crates/adapters/)                         | PLE, QTI, iMathAS, and WeBWorK Question Backend adapters, plus H5P Package Import.                       |
| [crates/wasm/](../crates/wasm/)                                 | The answer-free Rust-to-browser WebAssembly facade.                                                      |
| [crates/export/](../crates/export/)                             | PDF/DOCX export models and writers.                                                                      |
| [crates/project-tools/](../crates/project-tools/)               | TypeScript generation, fixtures, migrations, pilot content, and E2E seed tooling.                        |
| crates/acceptance-runtime/                                      | Disposable acceptance manifests and capability-specific database URL handoff.                            |

Package directories use hyphens; Rust module imports use underscores.

## Canonical course paths

The reusable and delivery aggregates have separate paths:

```text
crates/question_model/src/
+- blueprint_course.rs       BlueprintCourse tree and `BlueprintCourseView` readers
+- blueprint_operations.rs    Source, target, preview, apply, and receipt contracts
`- blueprint_operations/      Focused exact-operation contract modules
```

BlueprintCourse is one ordered module/assignment tree with one aggregate
revision. Its exact public question members resolve to immutable
QuestionRevisionReference pins. CourseInstance is not another source tree: the
Blueprint-operation boundary creates it under an exact CourseId, records the
immutable Blueprint parent and applied revision, and owns private delivery
state. New upstream assignments appear in daughter instances as unreleased.

The current paired legacy files and SQL table families are SD1 migration inputs
only. The immutable
schemas/migrations/2026081837_blueprint_alpha_curriculum.sql and accepted
successors remain historical evidence and are not renamed or edited to hide
their origin. The fresh SD1 migration epoch is tracked in
[active_plans/implementation_status.md](active_plans/implementation_status.md);
its course/curriculum range is planned at 2026082913-2026082916, with
protected authorization-function/RLS/grant helpers at 2026082929-2026082932.

## Learning data access

No Blueprint Course or Blueprint-operation Store implementation currently
exists under `crates/learning-data-access/`. The future Store boundary will own
the six exact operations, their idempotency, receipts, and Assignment Import Repair. It
will never grant a public Blueprint reader access to a private CourseInstance.

## Server application

```text
crates/server/src/
+- auth/                     Account session and seeded Live Demo browser boundary
+- composition.rs            Production database and session composition
+- health.rs                 Readiness probe support
+- http_security.rs          Uniform dynamic-response security headers
+- request_lifecycle.rs      Process-wide safe request lifecycle handling
+- application.rs            Executable application assembly
+- lib.rs                    Current server-core module boundary
`- main.rs                   Production binary entry point
```

The current executable surface intentionally stops at global Account sessions and
the deployment-gated seeded Live Demo entry. Course, Question Library, delivery,
and worker routes remain downstream reconstruction work and are not represented
as mounted server modules.
reference, revision, query, and body decoding. CourseInstance routes require
the exact destination course and current equal Teaching Team Member authority.

## Browser application

```text
src/
+- api/
|  +- blueprint_course.ts                 BlueprintCourse client contract
|  +- blueprint_operations.ts              Blueprint-operation client contract
|  +- http_client/blueprint_course.ts     Same-origin BlueprintCourse requests
|  +- http_client/blueprint_operations.ts Preview/apply/receipt requests
|  +- decoders/blueprint_course.ts        Strict BlueprintCourse DTO decoder
|  `- decoders/                           Other strict DTO decoders
+- features/
|  +- blueprint_course/                    One BlueprintCourse workspace/editor
|  `- blueprint_operations/                Blueprint-operation workflow stylesheet
+- pages/
|  +- blueprint_course_route_page.tsx          Blueprint Course list route composition
|  +- blueprint_course_detail_route_page.tsx   Blueprint Course detail route composition
|  `- (no Blueprint-operation page is mounted)
+- components/                                Shared answer-free and accessibility UI
+`- routes.ts                                Executable route map
```

The intended browser workspace has one BlueprintCourse list, detail, editor,
and nested module/assignment picker. It presents draft owner/collaborator
states and the vetted-Instructor published `BlueprintCourseView` without a second product
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

```text
schemas/migrations/          Ordered forward SQL; accepted files are immutable
containers/                  Compose, API/gateway images, private renderer
deploy/opentofu/             AWS network, compute, database, storage, IAM, and policy
crates/objects/              Typed public-assets/private-content/student-records/temp-processing
```

PostgreSQL stores policy-bearing relationships, BlueprintCourse and
CourseInstance records, attempts, submissions, summaries, jobs, and audit
events. Object storage holds bounded source and binary bytes. The production
API, workers, and publisher use separate capability profiles.

## Tests and generated output

```text
tests/
+- test_*.py                  Fast deterministic repository-policy checks
+- test_*.mjs                 Browser-contract and model checks without a browser
+- playwright/e2e/*.spec.ts   Production HTTPS browser journeys
+- e2e/                       Disposable PostgreSQL, service, lifecycle, and publication checks
`- fixtures/                  Small durable fixture evidence

generated/
+- api/                       Ignored Rust-derived TypeScript
`- fixtures/                 Ignored generated fixture outputs
```

Permanent tests protect behavior that can regress: tree ordering, exact pins,
authorization, strict decoding, adoption exclusions, unreleased propagation,
and answer-free browser reader data. Graphify and source/migration inventories are
one-time evidence. PostgreSQL, browser, process, migration, and rendered visual
checks stay in their named E2E or human-review lanes. See
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

Build output such as dist/, dist_wasm/, target/, and test-results/ is reproducible
ignored state. Committed screenshots under docs/screenshots/ are historical
visual reference, not source contracts. The former screenshot manifest and
publisher are absent; a restored browser owner will own fresh visual evidence.

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

- Add a reusable content rule to crates/question_model/src/blueprint_course.rs;
  update both reusable Store implementations and conformance cases.
- Add source-to-instance behavior to blueprint_operations with a typed preview,
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
