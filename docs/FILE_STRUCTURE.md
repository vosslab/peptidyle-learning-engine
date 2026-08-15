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
   +- local_identity.py  Local credential and hash-only identity projection
   +- renderer.py        Selected renderer OCI provenance and probe boundary
   +- _consumer_cli.py   Private disposable-consumer adapter
`- run_playwright_tests.sh Browser test entry point
```

`OTHER_REPOS/` contains reference snapshots. It is not a source import path,
container build context, or runtime dependency.

## Rust workspace

| Path                                                            | Owns                                                                                                                  |
| --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| [crates/question_model/](../crates/question_model/)             | Question types, capabilities, identifiers, and public presentation schemas.                                           |
| [crates/domain/](../crates/domain/)                             | Attempt state, policies, seeded generation, timing inputs, and answer-free validation.                                |
| [crates/grading/](../crates/grading/)                           | Answer keys, checkers, and correctness decisions.                                                                     |
| [crates/objects/](../crates/objects/)                           | Typed object-store interface, four bucket domains, checksums, image validation, and MinIO/S3 backends.                |
| [crates/learning-data-access/](../crates/learning-data-access/) | Store contracts, in-memory and PostgreSQL implementations, RLS, capability roles, migrations, and conformance tests.  |
| [crates/adapters/native/](../crates/adapters/native/)           | First-party generated questions and flat-question source compilation.                                                 |
| [crates/adapters/webwork/](../crates/adapters/webwork/)         | Private renderer protocol, safe projection, cache, and grading delegation.                                            |
| [crates/adapters/qti/](../crates/adapters/qti/)                 | Bounded QTI parsing, profile mapping, and private grading handoff.                                                    |
| [crates/adapters/h5p/](../crates/adapters/h5p/)                 | H5P practice import and capability declaration.                                                                       |
| [crates/adapters/imathas/](../crates/adapters/imathas/)         | iMathAS provider and broker boundary.                                                                                 |
| [crates/export/](../crates/export/)                             | Print model plus PDF and DOCX writers.                                                                                |
| [crates/wasm/](../crates/wasm/)                                 | `wasm-bindgen` facade over answer-free domain behavior.                                                               |
| [crates/server/](../crates/server/)                             | Axum API, auth, authorization, broker, ordinary worker, dedicated public-asset publisher, and dependency composition. |
| [crates/project-tools/](../crates/project-tools/)               | Repository-only code generation, fixture, pilot-content validation, migration, and E2E seed commands.                 |

Cargo package directories use hyphens; Rust imports use underscores. For
example, `learning-data-access` is imported as `learning_data_access`.

## Learning data access

```text
crates/learning-data-access/
+- src/
|  +- contracts/       Store and capability contracts
|  |  `- catalog.rs    Catalog query contract and HMAC continuation codec
|  +- in_memory/       Database-free capability implementations
|  |  +- catalog.rs    Catalog state projection, pagination, and snapshot assembly
|  |  `- catalog_search.rs Portable ranked-search admission and fixed-point scoring helpers
|  +- postgres/        PostgreSQL implementations and connection attestation
|  |  `- catalog/search.rs Canonical ranked full-text and word-similarity search
|  +- activity.rs      Actor-scoped learner reads and activity ownership
|  +- external_tool.rs External broker leases, dispatch, and finalization contracts
|  +- jobs.rs          Durable job and publication-outbox contracts
|  +- publication_validation/ Published content and asset registry validation
|  +- lib.rs           Public facade and stable re-exports
|  +- in_memory.rs     In-memory composition facade
|  `- postgres.rs      PostgreSQL composition facade
`- tests/
   +- conformance/     Backend-neutral behavior cases
   +- fixtures/        Small safe fixture evidence
   `- postgres_*_live.rs Disposable PostgreSQL acceptance gates, including ignored catalog Store, disclosure, and plan suites
```

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
+- course/                            Course, roster, invitation, and assignment routes
+- run/                               Attempt issue, prefetch, submission, feedback, and external-tool routes
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
`- migrations/        Ordered forward SQL migrations, including auth, RLS, external fences, publication outbox, and 2026081401 ranked catalog discovery

containers/
+- compose.yaml       Normal local services, private networks, hardening, and one-shot setup
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
+- playwright/        Built-browser tests and the private live-validation lane runner
+- e2e/               Generic disposable whole-system runners
+- walkthrough/       Teaching-loop entry points and fixed child processes
|  `- walklib/        Importable runner configuration, contracts, and lifecycle
`- fixtures/          Small checked-in fixture evidence

generated/
+- api/               Generated TypeScript contracts
`- fixtures/          Generated fixture projections
```

`dist/`, `dist_wasm/`, `target/`, `test-results/`, and Playwright report
directories are reproducible ignored output. Checked-in fixtures under
`tests/fixtures/` are source evidence and should change deliberately.

`python3 local_stack.py acceptance` is the explicit live aggregate entry point. It
hands lifecycle conflict detection and environment sanitization to the controller, then
`local_stack_control/acceptance_lanes.py` runs the maintained browser and real-stack lanes in a
fixed fail-fast order. `tests/playwright/run_validation_lanes.sh` is only a compatibility `exec`
facade back to that public Python command. Those opt-in commands are live acceptance evidence, not
part of the fast offline `pytest tests/` suite.

`tests/e2e/e2e_database_baseline.sh` selects the ignored PostgreSQL catalog
Store, disclosure, and qualitative plan suites by exact test name. It creates
the disposable database baseline and is live acceptance evidence, not a fast
offline test.

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
