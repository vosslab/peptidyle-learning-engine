# File structure

This map gives contributors the shortest route to the owner of a behavior.
[CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) explains how the pieces cooperate;
[CONTRACTS.md](CONTRACTS.md) indexes durable rules. Release plans and dated
status reports live under [active_plans/](active_plans/) and remain separate
from this file map.

## Top-level layout

```text
.
+- crates/               Rust product crates and repository tools
+- src/                  SolidJS and TypeScript browser application
+- schemas/              Forward PostgreSQL migrations
+- containers/           Local Podman Compose configuration and container files
+- deploy/opentofu/      AWS infrastructure-as-code and policy tests
+- content/              Checked-in teaching content and bounded pilot material
+- pipeline/             Browser and WebAssembly build steps
+- tests/                Hygiene, Node, Playwright, and E2E checks
+- docs/                 Durable documentation and active planning artifacts
+- devel/                Focused developer-maintenance commands
+- tools/                Focused repository utilities
+- generated/            Ignored generated contract and fixture projections
+- Cargo.toml            Rust workspace manifest
+- package.json          Browser tooling manifest
+- build.sh              Full local build entry point
+- check_codebase.sh     Vendored TypeScript and browser gate
+- check_rust.sh         Repository-owned Cargo and Rust gate
+- local_stack.py        Public local Podman controller entry point
+- local_stack_control/  Local Podman controller package and focused private lifecycle modules
   +- acceptance_lanes.py Ordered aggregate acceptance-lane owner
   +- chapter_one.py     Private atomic Chapter 1 publication boundary
   +- lifecycle.py       Typed lifecycle sequencing for start, validate, and restart
   +- local_environment.py Default-only private environment bootstrap
   +- browser_suite_developer.py Fixed production-browser developer owner
   +- browser_suite_lease.py      Shared developer/browser lease boundary
   +- browser_suite_reset.py      Fixed-owner resource reset and cleanup proof
   +- private_files.py   Atomic private-file creation and replacement boundary
   +- private_state.py   Descriptor-anchored repository-target E2E state owner
   +- renderer.py        Selected renderer OCI provenance and probe boundary
   +- _consumer_cli.py   Private disposable-consumer adapter
`- run_playwright_tests.sh Browser test entry point
```

`OTHER_REPOS/` contains reference snapshots. It is not a source import path,
container build context, or runtime dependency.

## Rust workspace

| Path                                                            | Owns                                                                                                                  |
| --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| [crates/question_model/](../crates/question_model/)             | Question types, assignment teaching/local-time values, mandatory course-term values, capabilities, identifiers, learner-progress projections, and public presentation schemas. |
| [crates/domain/](../crates/domain/)                             | Attempt state, policies, pure learner-disclosure evaluation, seeded generation, timing inputs, and answer-free validation.                                |
| [crates/grading/](../crates/grading/)                           | Answer keys, checkers, and correctness decisions.                                                                     |
| [crates/objects/](../crates/objects/)                           | Typed object-store interface, four bucket domains, checksums, image validation, and MinIO/S3 backends.                |
| [crates/learning-data-access/](../crates/learning-data-access/) | Store contracts, in-memory and PostgreSQL implementations, RLS, capability roles, migrations, and conformance tests.  |
| `crates/base-course-installation/`                               | Focused product crate, imported as `base_course_installation`, for typed Base Course request/receipt, recipe, and deterministic orchestration. |
| [crates/adapters/native/](../crates/adapters/native/)           | First-party generated questions and flat-question source compilation.                                                 |
| [crates/adapters/webwork/](../crates/adapters/webwork/)         | Private renderer protocol, safe projection, cache, and grading delegation.                                            |
| [crates/adapters/qti/](../crates/adapters/qti/)                 | Bounded QTI parsing, profile mapping, and private grading handoff.                                                    |
| [crates/adapters/h5p/](../crates/adapters/h5p/)                 | H5P practice import and capability declaration.                                                                       |
| [crates/adapters/imathas/](../crates/adapters/imathas/)         | iMathAS provider and broker boundary.                                                                                 |
| [crates/export/](../crates/export/)                             | Print model plus PDF and DOCX writers.                                                                                |
| [crates/wasm/](../crates/wasm/)                                 | `wasm-bindgen` facade over answer-free domain behavior.                                                               |
| [crates/server/](../crates/server/)                             | Axum API, auth, authorization, broker, ordinary worker, dedicated public-asset publisher, and dependency composition. |
| [crates/project-tools/](../crates/project-tools/)               | Direct `base-course` CLI adapter plus repository-only code generation, fixture, pilot-content validation, migration, and E2E seed commands. |

Cargo package directories use hyphens; Rust imports use underscores. For
example, `learning-data-access` is imported as `learning_data_access`.

`learning-data-access` remains the sole SQL, PostgreSQL-lock,
durable install-state, migration, and Store owner. The installer has no HTTP route or server-start
hook; `project-tools` adapts it directly for `cargo tools base-course`. Its evidence is deliberately
small: pure product-crate tests for typed request/receipt/recipe convergence, the existing LDA
PostgreSQL live oracle for schema and locking, and the existing
`tests/e2e/e2e_live_demo_baseline.py` for the connected lifecycle. No second product-specific
PostgreSQL harness or exhaustive live matrix is planned.

## Learning data access

```text
crates/learning-data-access/
+- src/
|  +- contracts/       Store and capability contracts
|  |  `- catalog.rs    Catalog query contract and HMAC continuation codec
|  |  `- curriculum_adoption.rs Revision-bound `CurriculumAdoptionStore` contract
|  +- in_memory/       Database-free capability implementations
|  |  +- catalog.rs    Catalog state projection, pagination, and snapshot assembly
|  |  `- catalog_search.rs Portable ranked-search admission and fixed-point scoring helpers
|  |  +- curriculum_adoption/ Dedicated B2 adoption state, destination materialization, atomic operations, and focused behavior tests
|  |  |  +- state.rs   Private baselines, provenance envelopes, receipts, and adoption outcomes
|  |  |  +- destination.rs Assignment/course materialization and reusable-meaning replacement
|  |  |  +- operations/ Preview/apply flows shared by instantiation, rollover, term shift, and controlled updates
|  |  |  `- tests/      Focused invalid-state and recovery behavior cases
|  |  `- reusable_curriculum/source_snapshot.rs Trusted exact-pin and assignment-source snapshot resolution
|  +- postgres/        PostgreSQL implementations and connection attestation
|  |  `- catalog/search.rs Canonical ranked full-text and word-similarity search
|  |  `- course_gradebook.rs Course-grade scheme, totals, and export implementation
|  |  `- effective_policy_receipts.rs Sealed effective-policy receipt persistence and reconstruction
|  |  `- assignment_records/learner_disclosure.rs Closed five-field disclosure-column decoder
|  |  `- item_analysis/learner_class_statistics.rs Learner-safe current course analysis projection
|  +- in_memory/course_policy.rs Atomic teaching-settings mutation and current policy resolution
|  +- postgres/course_policy.rs PostgreSQL teaching-settings CAS, lifecycle gate, and receipt update
|  +- activity.rs      Actor-scoped learner reads and activity ownership
|  +- assignment_revision.rs Checked conversion between the canonical domain revision and stored BIGINT
|  +- external_tool.rs External broker leases, dispatch, and finalization contracts
|  +- feedback.rs      Private current disclosure receipt and learner-projection inputs
|  +- jobs.rs          Durable job and publication-outbox contracts
|  +- publication_validation/ Published content and asset registry validation
|  +- lib.rs           Public facade and stable re-exports
|  +- in_memory.rs     In-memory composition facade
|  `- postgres.rs      PostgreSQL composition facade
`- tests/
   +- conformance/     Backend-neutral behavior cases
   +- fixtures/        Small safe fixture evidence
   `- postgres_*_live.rs Disposable PostgreSQL acceptance gates, including ignored course-term, catalog Store, course-grade, disclosure, and plan suites
```

Within `crates/question_model/src/`, `assignment/revision.rs` owns the sole assignment-revision
value and canonical decimal wire shape. `curriculum_adoption.rs` owns normalized B2 reusable meaning,
target-term relative-schedule resolution, typed DST corrections, and course schedule revisions;
`curriculum_adoption/contracts.rs` and its `contracts/` children own bounded answer-free previews,
preview-derived commands, exact assignment-definition source views, recovery decisions, and completed
receipt projections. `curriculum_adoption/contracts/assignment_source.rs` is the exact source locator
for one Blueprint or one positioned Alpha assignment.

When a persistence capability changes, update its contract, both
implementations, and matching conformance evidence. Actor-scoped learner
methods belong here rather than only in HTTP route checks. PostgreSQL connection
construction and migrations own production login-profile, capability-role, and
forced-RLS verification.

The catalog continuation codec is injected into both Store compositions from
the server secret. The in-memory search helpers intentionally provide portable
admission, ranking, and snapshot behavior; PostgreSQL owns its canonical
full-text and word-similarity predicates, ranked CTE, and database snapshot.
Forward migration
`schemas/migrations/2026081401_ranked_catalog_discovery.sql`
adds the monotonic publication/disclosure boundary, normalized search
projection, discovery indexes, and forced-RLS disclosure broker.

## Server application

```text
crates/server/src/
+- auth/                             Passwordless email, passkey, session, and request-boundary behavior
+- catalog/                           Catalog query and immutable publication behavior
+- course/                            Course, roster, invitation, assignment, and grade routes
|  +- assignments/learner.rs           Learner-safe assignment detail projection
|  +- assignments/teaching_settings.rs Instructor local-time teaching-settings CAS route
|  `- gradebook.rs                     Course-grade scheme, compact totals, and CSV export routes
+- run/                               Attempt issue, prefetch, submission, current disclosure redaction, and external-tool routes
+- workspace/                         Authoring workspace behavior
+- flat_question_publication/         Native publication routes and tests
+- public_asset_publication_worker/   Outbox handler and conditional registry activation
+- qti_*/                             QTI import, conversion, publication, and runtime paths
+- composition/                       Concrete API, worker, and publisher dependency assembly
+- asset.rs                           Authorized object delivery
+- request_lifecycle.rs               Request draining and shutdown coordination
+- http_security.rs                   HTTP response security boundary
+- webwork_backend/                   PLE-owned PG source and replay integration
+- worker/                            Generic durable-job runtime
+- lib.rs                             Server library facade
`- main.rs                            API, worker, or publisher process entry point
```

Route modules own HTTP behavior. `composition/` selects concrete stores,
identity providers, object storage, and question adapters. The ordinary worker
and `--public-asset-publisher` use separate composition paths and database
authority.

## Browser application

```text
src/
+- api/             Strict decoders, HTTP client, and generated contracts
|  `- decoders/assignment_policy.ts Exact five-field assignment-policy decoder
+- auth/            Account and course-session browser state
+- components/      Reusable prompt, response, feedback, and accessibility UI
+- features/        Capability-owned browser logic
+- pages/           Route-level views and page-specific state
|  +- assignment_editor_policy_panel.tsx Instructor disclosure-policy controls
|  +- assignment_teaching_operations_panel.tsx Lifecycle, instructions, schedule, limits, and late behavior
|  `- assignment_overview_page.tsx Learner-safe instructions, resolved delivery, and score-state view
+- learner_progress.ts Server-derived score-state display copy; never derives policy or timing
+- wasm/            Shared domain WebAssembly facade and Solid context
+- app.tsx          Application shell
+- routes.ts        Route definitions
`- main.tsx         Browser entry point
```

`src/components/responses/` contains response-family controls and
`src/components/response_widget/` contains shared keyboard and external-tool
extensions. The browser has no object-store credentials, answer keys, or
authority to issue a grading verdict.

## Question sources and engines

`content/` holds checked-in content. `content/pilot/chapter_1_assignments.yaml`
owns the reviewed Genetics and Biochemistry Chapter 1 inventory;
`content/pilot/sources/` retains licensed source evidence and
`content/pilot/flat/` contains curated PLE flat payloads. The bounded PGML
fixture is under `content/pilot/webwork/`.

`crates/project-tools/src/pilot_content.rs` validates the source corpus, while
`crates/project-tools/src/e2e_seed/chapter_one.rs` publishes the exact matrix
through production PostgreSQL and object-store contracts. The browser and API
never execute source from `OTHER_REPOS/`.

The service image and probe configuration are in
[containers/webwork/](../containers/webwork/). The PLE adapter is in
[crates/adapters/webwork/](../crates/adapters/webwork/), and PLE-specific
source and replay selection is in
[crates/server/src/webwork_backend/](../crates/server/src/webwork_backend/).

## Database, storage, containers, and deployment

```text
schemas/
`- migrations/        Ordered forward SQL migrations, including auth, RLS, external fences, publication outbox, 2026081401 ranked catalog discovery, and 2026081805 assignment learner-disclosure policy

containers/
+- compose.yaml       Common local and disposable topology, private networks, hardening, and one-shot setup
+- compose.smtp.yaml  Optional external SMTP-provider overlay
+- Containerfile.api  Shared API, worker, and publisher image
+- Containerfile.gateway Gateway image
+- Caddyfile          Same-origin browser and API gateway rules
+- env.example        Safe environment template
`- webwork/           Renderer semantic probe

deploy/opentofu/
+- network.tf         VPC, subnet, endpoint, and security-group topology
+- edge.tf            CloudFront, WAF, TLS edge behavior, and ALB origin controls
+- compute.tf         ECS API, worker, and publisher tasks and IAM roles
+- database.tf        RDS and database network configuration
+- storage.tf         Four S3 domains, KMS keys, and object policies
+- *.tf               Variables, locals, outputs, backend, and observability
+- tests/             OpenTofu policy assertions
`- DATABASE_PROVISIONING.md Production login and capability-role provisioning procedure
```

The default local services are PostgreSQL, MinIO, API, ordinary worker,
gateway, and a private standalone renderer. PostgreSQL and MinIO use named
volumes; the other service containers can be rebuilt from configuration.

`python3 local_stack.py` is the operator-facing controller. Its focused
`local_stack_control/` modules own the local stack's build, bootstrap, migration,
seed, renderer, restart, validation, and semantic-readiness sequence directly.
The package is organized by
concern: `models.py` declares typed targets and inspected resources;
`process.py` provides the command boundary; `compose.py` resolves providers and
target environments; `env_file.py` validates safe environment-file metadata;
`discovery.py` reads label-derived Podman topology; `status.py` derives
readiness; `cleanup.py` constructs scoped stop/reset plans; `commands.py` owns
operator operations; and `consumer.py` limits disposable E2E ownership.

`local_stack_control/_consumer_cli.py` is intentionally narrower than the public controller.
It accepts a private, owner-specific manifest and only runs scoped Compose
actions or the matching scoped cleanup plan. The controller's default mutation
target is `containers`; its disposable adapter does not provide arbitrary
Podman or Compose-project access.

`crates/objects/` maps typed keys to `public-assets`, `private-content`,
`student-records`, and `temp-processing`. In the OpenTofu target, each has an
individual S3 bucket and KMS key. The public-asset publisher is the only
dedicated process that turns pending private source into an active public
asset; it has its own task role, execution role, and database login contract.

## Tests and generated output

```text
tests/
+- test_*.py          Fast repository-policy and documentation checks
+- test_local_stack_control.py Offline typed local-stack controller contracts
+- test_*.mjs         Deterministic browser-contract checks without a browser
+- playwright/        Production-browser scenarios and private live-validation helpers
|  `- e2e/*.spec.ts    Catalog-owned scenarios selected by run_playwright_tests.sh
+- e2e/               Generic disposable whole-system runners
|  +- `compose.live-demo-browser.yaml` Owner-locked disposable production-auth/TLS E2E overlay; not an operator production deployment
|  `- `Caddyfile.live-demo-browser` Owner-locked disposable production-auth/TLS E2E gateway; not an operator production deployment
`- fixtures/          Small checked-in fixture evidence

generated/
+- api/               Generated TypeScript contracts
`- fixtures/          Generated fixture projections
```

`dist/`, `dist_wasm/`, `target/`, `test-results/`, and Playwright report directories are reproducible
ignored output. `dist/` is the production browser artifact used by the fixed developer session,
Playwright, screenshot capture, service oracles, and connected acceptance. Those consumers share the
`ple-live-demo-browser` lifecycle and seeded production authentication. Checked-in fixtures under
`tests/fixtures/` are source evidence and should change deliberately.

Committed visual evidence lives under `docs/screenshots/`, organized by role and access boundary:

```text
docs/screenshots/
+- instructor/       Desktop professor evidence at 1280 by 800 or larger
+- student/           Allowed learner surfaces across the student viewport matrix
|  `- access/         Student denial and no-transport access evidence
`- shared/            Evidence shared by instructor and student surfaces
```

`tests/e2e/browser_screenshot_corpus.json` is the sole screenshot ownership authority.
`tests/playwright/ui_corpus_manifest.ts` and
`tests/e2e/e2e_browser_screenshot_contract.py` strictly consume its artifact
names, routes, roles, pipelines, viewports, and evidence purposes. The
directories describe evidence boundaries. A retained image is not canonical
acceptance evidence until V1 captures it from the real origin and its
provenance verifier and visual review pass.

`source source_me.sh && python3 local_stack.py acceptance` is the explicit live aggregate entry
point. `local_stack_control/commands.py` owns conflict preflight and environment sanitization;
`local_stack_control/acceptance_lanes.py` then runs the maintained browser and real-stack lanes in
a fixed fail-fast order. These opt-in commands are live acceptance evidence, not part of the fast
offline `pytest tests/` suite.

`tests/e2e/e2e_database_baseline.sh` selects the ignored PostgreSQL catalog
Store, disclosure, and qualitative plan suites by exact test name. It creates
the `DATABASE_BASELINE` profile of the fixed `ple-live-demo-browser` shared
lease and project. It is live acceptance evidence, not a fast offline test.

`./capture_screenshots.sh` is the explicit publication gate when UI, corpus,
or viewport changes require fresh visual evidence. `./all_test.sh` validates
the aggregate behavior and contracts without rewriting checked-in screenshots;
both commands use the same fixed stack.

## Documentation map

- [README.md](../README.md) is the newcomer entry point and first verified
  workflow.
- [INSTALL.md](INSTALL.md), [USAGE.md](USAGE.md),
  [DEVELOPMENT.md](DEVELOPMENT.md), and [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
  document operation and contributor workflows.
- [USER_ROLES.md](USER_ROLES.md), [SECURITY_MODEL.md](SECURITY_MODEL.md),
  [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
  [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md), and
  [DATABASE_TENANCY.md](DATABASE_TENANCY.md) document security boundaries.
- [OBJECT_STORAGE.md](OBJECT_STORAGE.md),
  [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md), and
  [CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md) document durable
  storage and state transitions.
- [LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md),
  [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md), and
  [MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md) document runtime topology.
- [active_plans/](active_plans/) contains in-flight scope, audits, decisions,
  workstreams, and dated reports.

## Where to add work

- Put a domain rule in its focused module under `crates/domain/src/`.
- Put persistence behavior in a data-access contract, its implementations, and
  conformance tests when both stores support it.
- Put a learner-visible endpoint in the owning server capability module and
  use an actor-scoped store operation for learner data.
- Put a new durable job handler in the server worker subsystem; use a separate
  process, database capability, and IAM role when its authority differs from
  the ordinary worker.
- Put a forward database change in [schemas/migrations/](../schemas/migrations/);
  preserve applied migrations as history.
- Put normal local-stack lifecycle policy in `local_stack_control/`; use
  `python3 local_stack.py` for its public command. Keep initialization, migration,
  seeding, and semantic startup behavior in focused typed Python modules.
- Put a disposable E2E lifecycle owner in `local_stack_control/consumer.py`
  only when it has a closed project namespace, a private manifest, and a
  project-scoped cleanup contract.
- Put a durable design or contract document in `docs/` and an in-flight plan or
  status artifact in the appropriate [active_plans/](active_plans/) subtree.
