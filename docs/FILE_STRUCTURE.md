# File structure

This map names each area by its responsibility first and gives the current code
path second. The system architecture and security boundaries are documented in
[CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md).

## Top-level layout

```text
.
+- crates/             Rust product crates and repository project tools
+- src/                SolidJS and TypeScript browser application
+- schemas/            PostgreSQL baseline migrations
+- containers/         Local PostgreSQL, MinIO, API, renderer, and gateway stack
+- pipeline/           Maintained browser and WebAssembly build steps
+- tests/              Fast hygiene, Node behavior, Playwright, and E2E gates
+- docs/               Architecture, contracts, operations, and active plans
+- devel/              Small repository maintenance scripts
+- tools/              Focused developer utilities
+- generated/          Ignored reproducible TypeScript and fixture projections
+- Cargo.toml          Rust workspace and exhaustive internal dependencies
+- package.json        Browser dependencies and repository front-door commands
+- build.sh            Complete build entry point
`- check_codebase.sh   Complete static and behavior gate
```

## Rust product crates

| Plain component name | Current path                                        | Responsibility                                                                                          |
| -------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Question model       | [crates/question_model/](../crates/question_model/) | Question-agnostic definitions, identities, policies, capabilities, and browser-safe projections         |
| Domain rules         | [crates/domain/](../crates/domain/)                 | Attempt state, timing, scoring, completion, generation, validation, and analysis math                   |
| Server-only grading  | [crates/grading/](../crates/grading/)               | Answer keys, checkers, and correctness decisions                                                        |
| Object storage       | [crates/objects/](../crates/objects/)               | Object contract, S3/MinIO backend, keys, checksums, and bucket rules                                    |
| Learning data access | [crates/learning-data-access/](../crates/learning-data-access/)                   | Persistence contracts, in-memory and PostgreSQL implementations, migrations, RLS, jobs, and conformance |
| Question engines     | [crates/adapters/](../crates/adapters/)             | Native, WeBWorK, QTI, H5P, and iMathAS adapters                                                         |
| Exam export          | [crates/export/](../crates/export/)                 | Deterministic DOCX and PDF models and writers                                                           |
| WebAssembly bridge   | [crates/wasm/](../crates/wasm/)                     | Browser-safe delegation to domain rules without grading dependencies                                    |
| API and workers      | [crates/server/](../crates/server/)                 | Axum routes, authentication, composition, adapters, and worker handlers                                 |

## Question engines

Question engines keep format-specific parsing and rendering behind stable
adapter facades. The native engine owns generated families in `generator.rs`
and the strict static PLE JSON source compiler in
[flat_question.rs](../crates/adapters/native/src/flat_question.rs). The latter
has no data-access or HTTP concerns: it only validates, canonicalizes, splits
public/private content, and performs server-only evaluation.

QTI import is split into four focused owners:

- [crates/adapters/qti/src/lib.rs](../crates/adapters/qti/src/lib.rs) for stable
  public re-exports;
- `crates/adapters/qti/src/model.rs` for archive limits, safe projections,
  warnings, and per-item outcomes;
- [crates/adapters/qti/src/parser_stub.rs](../crates/adapters/qti/src/parser_stub.rs)
  for bounded ZIP/XML parsing, asset extraction, normalized duplicate checks,
  and server-only grading handoff; and
- `crates/adapters/qti/src/parser_stub/tests.rs` for hostile archives, resource
  limits, partial success, and duplicate warnings.

## Learning data access

The current `store` path is the learning data-access component. New work should
follow capability ownership rather than adding more behavior to its parent
files.

```text
crates/learning-data-access/
+- src/
|  +- lib.rs                 Contract facade and stable re-exports
|  +- activity_policy.rs     Activity, timing, score, and completion policy
|  +- asset_delivery.rs      Immutable asset registration and protected delivery
|  +- external_tool.rs       External-tool persistence contract
|  +- feedback.rs            Feedback and summary contract
|  +- item_analysis.rs       Course item-analysis contract
|  +- jobs.rs                Durable job types and queue contract
|  +- manual_grading.rs      Manual-grading contract
|  +- policy.rs              Groups and assignment exceptions
|  +- publication_validation.rs
|  +- qti.rs                 Private QTI registry and isolated grader contract
|  +- retention.rs           Retention policy and contract types
|  +- session.rs             Authentication-session persistence contract
|  +- memory.rs              In-memory data-access composition
|  +- memory/                In-memory capability implementations
|  +- postgres.rs            PostgreSQL data-access composition
|  `- postgres/              PostgreSQL capability implementations
`- tests/
   +- conformance.rs         Shared conformance facade and broad activity cases
   +- conformance/           Capability-focused conformance cases
   `- postgres_*_live.rs     Opt-in disposable PostgreSQL behavior gates
```

A contributor changing one capability should normally read its contract, the
chosen backend implementation, and its conformance test. For example,
external-tool persistence is owned by four files:

- [crates/learning-data-access/src/external_tool.rs](../crates/learning-data-access/src/external_tool.rs)
- [crates/learning-data-access/src/in_memory/external_tool.rs](../crates/learning-data-access/src/in_memory/external_tool.rs)
- [crates/learning-data-access/src/postgres/external_tool.rs](../crates/learning-data-access/src/postgres/external_tool.rs)
- [crates/learning-data-access/tests/conformance/external_tool.rs](../crates/learning-data-access/tests/conformance/external_tool.rs)

Protected asset delivery uses the same complete ownership shape:

- [crates/learning-data-access/src/asset_delivery.rs](../crates/learning-data-access/src/asset_delivery.rs)
- [crates/learning-data-access/src/in_memory/assets.rs](../crates/learning-data-access/src/in_memory/assets.rs)
- [crates/learning-data-access/src/postgres/assets.rs](../crates/learning-data-access/src/postgres/assets.rs)
- [crates/learning-data-access/tests/conformance/assets.rs](../crates/learning-data-access/tests/conformance/assets.rs)

Authentication sessions use four focused owners:

- [crates/learning-data-access/src/session.rs](../crates/learning-data-access/src/session.rs)
- [crates/learning-data-access/src/in_memory/sessions.rs](../crates/learning-data-access/src/in_memory/sessions.rs)
- [crates/learning-data-access/src/postgres/sessions.rs](../crates/learning-data-access/src/postgres/sessions.rs)
- [crates/learning-data-access/tests/conformance/sessions.rs](../crates/learning-data-access/tests/conformance/sessions.rs)

QTI import and its separately injected grader capability use:

- `crates/learning-data-access/src/qti.rs`
- [crates/learning-data-access/src/in_memory/qti.rs](../crates/learning-data-access/src/in_memory/qti.rs)
- [crates/learning-data-access/src/postgres/qti.rs](../crates/learning-data-access/src/postgres/qti.rs)
- [crates/learning-data-access/tests/conformance/qti.rs](../crates/learning-data-access/tests/conformance/qti.rs)
- `crates/learning-data-access/tests/postgres_qti_import_live.rs` for the disposable
  production-schema/RLS oracle

## API server and workers

[crates/server/src/lib.rs](../crates/server/src/lib.rs) is the library facade.
[crates/server/src/composition.rs](../crates/server/src/composition.rs) wires
concrete dependencies; it should not own route behavior.

Key route and worker owners include:

- `auth.rs`, `catalog.rs`, `course.rs`, `workspace.rs`, and `run.rs` for HTTP
  capability groups.
- `run/external_tool.rs` and `run/manual_grading.rs` for focused run-domain
  routes.
- [qti_publication.rs](../crates/server/src/qti_publication.rs) for the QTI
  publication route and `qti_publication/tests.rs` for its bytes-first,
  authorization, revision, and immutable-promotion contract.
- [qti_backend.rs](../crates/server/src/qti_backend.rs) for QTI issue,
  reproduction, and private grading, with shared fixtures and focused direct
  grading/run-lifecycle cases under `qti_backend/tests/`.
- `worker.rs` for the closed handler/committer registry, `worker/runtime.rs`
  for bounded polling and shutdown, and `composition/worker.rs` for the six
  complete production families. Scoring, timing, export, retention,
  item-analysis, and QTI-import behavior remains in its owning module.
- Backend adapter files such as `webwork_backend.rs`, `qti_backend.rs`, and
  `imathas_backend.rs`; private grading material remains server-side.

## Project tools

[crates/project-tools/](../crates/project-tools/) contains repository-only project tools. The
Cargo package name is `project-tools`; the contributor command is `cargo tools`.

| Command                        | Purpose                                                            |
| ------------------------------ | ------------------------------------------------------------------ |
| `cargo tools tsgen`            | Generate browser TypeScript contracts from Rust models             |
| `cargo tools fixtures --check` | Verify tracked fixture evidence and refresh its ignored projection |
| `cargo tools bindgen ...`      | Generate version-matched browser and Node WebAssembly glue         |
| `cargo tools database ...`     | Report, apply, or verify the six SQLx migrations explicitly        |
| `cargo tools e2e-seed ...`     | Seed and exercise the production PostgreSQL contract for E2E runs  |

`cargo tools` is the stable contributor-facing command. Do not add product
runtime behavior to this crate.

## Browser application

```text
src/
+- api/            Generated-contract decoding, HTTP client, repositories, mocks
+- auth/           Session context and authenticated browser state
+- components/     Reusable question, response, and feedback components
+- features/       Capability-owned browser logic such as the attempt loop
+- pages/          Route pages and page-specific models/repositories
+- wasm/           One shared WebAssembly facade and Solid context
+- app.tsx         Application shell
+- routes.ts       Route definitions
`- main.tsx        Browser entry point
```

The browser receives no answer keys or grading implementation. Add API shapes
through the Rust-owned generated contracts rather than handwritten duplicate
interfaces.

## Database schema

[schemas/migrations/](../schemas/migrations/) contains exactly six pre-data
baseline files, ordered by domain:

1. Principals and authentication.
2. Catalog and authoring.
3. Courses and assignments.
4. Activity and feedback.
5. Operations and analytics.
6. Retention.

Once durable deployment data exists, schema changes become forward migrations;
the six baseline files are no longer edited in place.

## Tests

- [tests/](../tests/) contains fast repository hygiene checks in `test_*.py`.
- `tests/test_*.mjs` contains deterministic browser-contract and component
  behavior tests without a live browser.
- [tests/playwright/](../tests/playwright/) tests built browser behavior over
  HTTP.
- [tests/e2e/](../tests/e2e/) owns slower whole-system, replica, WebAssembly,
  and disposable PostgreSQL gates.
- Rust unit and integration tests remain beside their owning crates.

## Generated artifacts

These directories are reproducible and ignored:

- `generated/api/` - TypeScript contracts generated from Rust models.
- `generated/fixtures/` - TypeScript projection of tracked fixture evidence.
- `dist/` - browser bundle.
- `dist_wasm/` - browser and Node WebAssembly glue.
- `target/` - Rust build artifacts.
- `test-results/` and `playwright-report/` - browser-test evidence.

Tracked fixture evidence under
[tests/fixtures/published_problem/](../tests/fixtures/published_problem/) is
intentional source material, not disposable generated output.

## Documentation map

- [docs/CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) explains system and
  security boundaries.
- [docs/CONTRACTS.md](CONTRACTS.md) records module owners, consumers, and
  stubs.
- [docs/HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) records durable owner
  decisions.
- [docs/active_plans/implementation_plan.md](active_plans/implementation_plan.md)
  is the architecture and milestone source of truth.
- [docs/active_plans/partial_commit_status.md](active_plans/partial_commit_status.md)
  records the current verified checkpoint and remaining order.

## Where to add new work

- Add domain rules to the owning focused file under `crates/domain/src/`.
- Add persistence as a contract plus in-memory, PostgreSQL, and conformance
  modules under the learning data-access subtree.
- Add HTTP behavior to the owning server route module and keep composition thin.
- Add browser behavior to the owning feature, component, page, or API module.
- Add question-format behavior behind an adapter; keep grading server-only.
- Add repository automation under the project-tools subtree and expose a
  `cargo tools` command.
- Keep every new source file below 1000 lines and split by capability before a
  parent becomes an implementation warehouse.

## Naming convention

Cargo packages and crate directories use descriptive hyphenated names, such as
`learning-data-access` and `project-tools`. Rust imports and modules use the
language-native underscore forms, such as `learning_data_access` and
`in_memory`. The naming migration updated manifests, imports, scripts, tests,
and documentation as one atomic package.
