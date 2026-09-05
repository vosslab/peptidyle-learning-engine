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
+- docs/                   Durable references, history, and bounded active work
+- local_stack_control/    Typed local-stack and acceptance lifecycle
+- invitation_mailer/      Temporary attended Mail.app sender implementation
+- launchers/              User-facing entry points and the aggregate gate wrapper
+- devel/                  Developer-maintenance commands
+- tools/                  Focused repository utilities
+- Cargo.toml              Rust workspace manifest
+- package.json            Browser tooling manifest
+- invitation_mailer.yaml  Recipient-domain allowlist and throttle for the mailer
+- build.sh                Full local build entry point
+- check_rust.sh           Rust gate
+- check_codebase.sh       Fast TypeScript/Node typecheck, lint, format, and test gate
+- run_live_demo.sh        Live-demo lifecycle front door
`- run_playwright_tests.sh Retained real-stack-input wrapper; not a current acceptance entry point
```

OTHER_REPOS/ contains reference snapshots only. It is not a runtime,
container, or source-import path.

## Rust workspace

| Path                                                            | Owns                                                                                                                                                                                              |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [crates/question_model/](../crates/question_model/)             | Question, identity, assignment, course-term, BlueprintCourse, Blueprint-operation, and browser-safe contract types.                                                                               |
| [crates/domain/](../crates/domain/)                             | Pure timing, policy, disclosure, Assignment Attempt, scoring, generation, and validation.                                                                                                         |
| [crates/grading/](../crates/grading/)                           | Answer-bearing checkers and correctness decisions; server-only.                                                                                                                                   |
| [crates/learning-data-access/](../crates/learning-data-access/) | Focused Account Session, authentication, Assignment Attempt, Question Source, object-record, grading-operation, pagination, iMathAS Question Backend Session, and PostgreSQL persistence modules. |
| [crates/server/](../crates/server/)                             | Axum routes, authentication, authorization, worker composition, and API assembly.                                                                                                                 |
| [crates/objects/](../crates/objects/)                           | Typed Object Addresses, checksums, image validation, and object-store backends.                                                                                                                   |
| [crates/adapters/](../crates/adapters/)                         | PLE, iMathAS, and WeBWorK Question Backend adapters, QTI Import, and H5P Package support behind the shared Question operations.                                                                   |
| [crates/wasm/](../crates/wasm/)                                 | The answer-free Rust-to-browser WebAssembly facade.                                                                                                                                               |
| [crates/export/](../crates/export/)                             | PDF/DOCX export models and writers.                                                                                                                                                               |
| [crates/project-tools/](../crates/project-tools/)               | TypeScript generation, fixtures, migrations, pilot content, and E2E seed tooling.                                                                                                                 |
| crates/acceptance-runtime/                                      | Disposable acceptance manifests and capability-specific database URL handoff.                                                                                                                     |

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

The current paired legacy files and SQL table families are terminology-migration
inputs only. The immutable
schemas/migrations/2026081837_blueprint_alpha_curriculum.sql and accepted
successors remain historical evidence and are not renamed or edited to hide
their origin. The checked-in pre-production migration sequence and forward allocation rule are
documented in [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).

## Learning data access

The current module inventory is:

```text
crates/learning-data-access/src/
+- assignment_attempt.rs                 Assignment Attempt Store contract
+- authentication_ceremony.rs            email and passkey ceremony contracts
+- authentication_email.rs               normalized authentication email values
+- grading_operations.rs                 Instructor Grading Operation Store contract
+- imathas_question_backend_session/     iMathAS Question Backend Session contracts and Memory support
+- object_record.rs                      workspace Question Source object records
+- pagination.rs                         cursor and page contracts
+- question_source.rs                    Draft source resolution and Question Publication Store contracts
+- session.rs                            Account Session Store contract
`- postgres/                             current PostgreSQL connection, migration, Account Session, Assignment Attempt, Question Source, object-record, and iMathAS Session modules
```

No Blueprint Course or Blueprint-operation Store implementation currently
exists under `crates/learning-data-access/`. The future Store boundary will own
Create Course from Blueprint, Fork Blueprint Course, Copy Assignment from
Blueprint, Apply Blueprint Update, Copy Course for New Term, and Shift Course
Dates; it will use exact operation identities and request checksums, retain receipts,
and keep Assignment Import Repair bounded to derived state. It will preserve
the boundary between public Blueprint readers and private CourseInstances.

## Server application

```text
crates/server/src/
+- auth/                     Account session and seeded Live Demo browser boundary
+- composition.rs            Production database and session composition
+- health.rs                 Readiness probe support
+- http_security.rs          Uniform dynamic-response security headers
+- question_publication.rs  Server-only new-lineage Question Publication coordinator and Question ID issuer
+- request_lifecycle.rs      Process-wide safe request lifecycle handling
+- application.rs            Executable application assembly
+- lib.rs                    Current server-core module boundary
`- main.rs                   Production binary entry point
```

The current executable surface intentionally stops at global Account sessions and
the deployment-gated seeded Live Demo entry. Course, Question Library, delivery,
publication, and worker routes remain downstream reconstruction work. The implemented server-only
`question_publication` Service composes the authorized draft-source read, verified immutable object
copy, Question ID issuance, and atomic P1 Store without expanding the executable route surface.
reference, revision, query, and body decoding. CourseInstance routes require
the exact destination course and current equal Teaching Team Member authority.

## Browser application

```text
src/
+- application_shell.tsx                    Persistent shell, content origin, skip-link/focus boundary, and Ribbon mount
+- ribbon/                                  Catalog/schema, capability admission, scope, selection/pending state, and fixed-row presentation
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
|  `- (no Blueprint-operation Browser Surface exists)
+- components/                                Shared answer-free and accessibility UI
+`- routes.ts                                Executable route map
```

The intended browser workspace has one BlueprintCourse list, detail, editor,
and nested module/assignment picker. It presents draft owner/collaborator
states and the vetted-Instructor published `BlueprintCourseView` without a second product
branch. Blueprint operations have exact outcomes: Copy Assignment from
Blueprint places one Assignment in an existing Course Instance, Create Course
from Blueprint creates a new Course Instance, and Fork Blueprint Course creates
a new Blueprint Course.

`devel/generate_ribbon_destination_ledger.mjs` maintains only the machine-owned
section of `docs/ux/RIBBON_DESTINATION_LEDGER.md` from the Ribbon catalog and
capability registry. Its `--check` mode is the deterministic maintenance check;
the editorial section remains human-owned.

## Generated contracts

crates/project-tools/src/tsgen.rs generates TypeScript from Rust contract
roots into ignored generated/api/. cargo tools tsgen and build.sh are the
generation entry points. Generated modules are derivative and must not be
hand-edited. Rust Serde owns field spelling and closed DTO shape; authored
decoders under src/api/decoders/ enforce runtime strictness.

Legacy paired generated names are retained only in the terminology-migration
inventory until regeneration after the Rust contract cutover. No client may
accept an old reference or route as a compatibility alias.

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
+- test_*.mjs                 Fast Node behavior, contract, and model checks without a browser
+- e2e/                       Non-browser production-build and disposable service acceptance; e2e_run_all.sh owns its declared checks
+- playwright/ribbon_*.mjs    Focused compiled-Chromium structural, responsive, and visual evidence; not production acceptance
`- fixtures/                  Small durable fixture evidence

generated/
+- api/                       Ignored Rust-derived TypeScript
`- fixtures/                 Ignored generated fixture outputs
```

Permanent tests protect behavior that can regress: tree ordering, exact pins,
authorization, strict decoding, Blueprint-operation authorization boundaries,
unreleased propagation, answer-free browser reader data, and deterministic Ribbon
model behavior. Graphify and source/migration inventories are one-time evidence.
`tests/e2e/e2e_run_all.sh` owns the current non-browser production-build E2E
checks. The focused compiled-Chromium Ribbon scripts exercise supplied fixture
content and are visual/structural evidence only. They do not substitute for the
separately unclaimed human-input real-stack browser suite, which must serve the
production bundle through the local HTTPS stack and create product state through
visible PLE controls. PostgreSQL, process, migration, and rendered visual checks
stay in their named E2E or human-review lanes. See
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
- Execution-only working plans, audits, and reports: not durable authority.
- [archive/](archive/): retired plans, dated reviews, and historical status evidence.
- [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md): evidence classes and required gates.

## Where to add work

- Add a reusable content rule to `crates/question_model/src/blueprint_course.rs`.
- Add Blueprint-operation persistence with a typed preview, command,
  authorization, request-retry binding, and immutable receipt.
- Add schema only through the forward allocation rule in
  [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md); preserve applied migrations.
- Add routes in the owning server module and register method policy in
  route_policy.rs.
- Regenerate generated/api/, then update strict decoders and typed clients.
- Add visible behavior to the owning feature/page/component; keep delivery and
  FERPA decisions server-authoritative.
- Add operational lifecycle behavior to local_stack_control/ and disposable
  evidence to its closed owner policy.
