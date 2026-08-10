# File structure

This map names each area by its responsibility first and gives the current code
path second. The system architecture and security boundaries are documented in
[CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md). Package and release status comes
from the active [release_completion_plan.md](active_plans/active/release_completion_plan.md)
and the dated [project_status_report_2026-08-09.md](active_plans/project_status_report_2026-08-09.md):
WP-RC1 and WP-RC2 are accepted, while WP-RC3 remains implemented but not
accepted until its live gates pass. Later release packages describe planned
work, not files already present in this tree.

## Top-level layout

```text
.
+- crates/             Rust product crates and repository project tools
+- src/                SolidJS and TypeScript browser application
+- schemas/            PostgreSQL baseline migrations
+- containers/         Local PostgreSQL, MinIO, API, gateway, and optional private WeBWorK stack
+- pipeline/           Maintained browser and WebAssembly build steps
+- tests/              Fast hygiene, Node behavior, Playwright, and E2E gates
+- docs/               Architecture, contracts, operations, and active plans
+- devel/              Small repository maintenance scripts
+- tools/              Focused developer utilities
+- generated/          Ignored reproducible TypeScript and fixture projections
+- Cargo.toml          Rust workspace and exhaustive internal dependencies
+- package.json        Browser dependencies and repository front-door commands
+- build.sh            Complete build entry point
+- launch_local_stack.sh  Build, start, health-check, and open the local Podman stack
`- check_codebase.sh   Complete static and behavior gate
```

## Rust product crates

| Plain component name | Current path                                                    | Responsibility                                                                                          |
| -------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Question model       | [crates/question_model/](../crates/question_model/)             | Question-agnostic definitions, identities, policies, capabilities, and browser-safe projections         |
| Domain rules         | [crates/domain/](../crates/domain/)                             | Attempt state, timing, scoring, completion, generation, validation, and analysis math                   |
| Server-only grading  | [crates/grading/](../crates/grading/)                           | Answer keys, checkers, and correctness decisions                                                        |
| Object storage       | [crates/objects/](../crates/objects/)                           | Object contract, S3/MinIO backend, keys, checksums, and bucket rules                                    |
| Learning data access | [crates/learning-data-access/](../crates/learning-data-access/) | Persistence contracts, in-memory and PostgreSQL implementations, migrations, RLS, jobs, and conformance |
| Question engines     | [crates/adapters/](../crates/adapters/)                         | Native, WeBWorK, QTI, H5P, and iMathAS adapters                                                         |
| Exam export          | [crates/export/](../crates/export/)                             | Deterministic DOCX and PDF models and writers                                                           |
| WebAssembly bridge   | [crates/wasm/](../crates/wasm/)                                 | Browser-safe delegation to domain rules without grading dependencies                                    |
| API and workers      | [crates/server/](../crates/server/)                             | Axum routes, authentication, composition, adapters, and worker handlers                                 |

## Question engines

Question engines keep format-specific parsing and rendering behind stable
adapter facades. The native engine owns generated families in `generator.rs`
and the strict static PLE JSON source compiler in
[flat_question.rs](../crates/adapters/native/src/flat_question.rs). The latter
has no data-access or HTTP concerns: it only validates, canonicalizes, splits
public/private content, and performs server-only evaluation.

The adapter-level QTI parser is split into focused owners:

- [crates/adapters/qti/src/lib.rs](../crates/adapters/qti/src/lib.rs) for stable
  public re-exports;
- `crates/adapters/qti/src/model.rs` for archive limits, safe projections,
  warnings, and per-item outcomes;
- `crates/adapters/qti/src/parser.rs`
  for bounded ZIP/XML parsing, asset extraction, normalized duplicate checks,
  and server-only grading handoff; and
- `crates/adapters/qti/src/parser/tests.rs` for hostile archives, resource
  limits, partial success, and duplicate warnings.

The H5P practice importer is `crates/adapters/h5p/src/import.rs`. The
server-only WeBWorK renderer boundary is
`crates/adapters/webwork/src/renderer_contract.rs`; its HTTP implementation is
`http_renderer.rs`, fixed upstream details are in `shipped_render_rpc.rs`, and
safe cached markup passes through `sanitizer.rs`. The HTTP implementation uses
`html5ever` to project the one supported single-radio group and refuses the
other upstream control shapes. `crates/server/src/webwork_backend.rs` resolves
the immutable catalog source and joins the adapter to the run backend without
exposing source, upstream field names, or credentials to the browser.

The accepted Canvas/Blackboard profile-to-native path adds these contributor entry points:

- [crates/adapters/qti/src/profiles/](../crates/adapters/qti/src/profiles/) owns the reviewed
  profile matrix, mapped-item model, canonical digests, and vendor-specific readers.
- [crates/server/src/qti_profile_import.rs](../crates/server/src/qti_profile_import.rs) owns the
  author-only opaque upload plus answer-free status/report route.
- [crates/server/src/qti_import.rs](../crates/server/src/qti_import.rs) owns worker parsing and safe
  registry persistence.
- [crates/server/src/qti_profile_conversion.rs](../crates/server/src/qti_profile_conversion.rs)
  owns exact archive/report revalidation and the atomic conversion command.
- [crates/server/src/qti_profile_flat_bridge.rs](../crates/server/src/qti_profile_flat_bridge.rs)
  is the only profile-mapped-item to native-flat compilation bridge.
- [src/features/qti_profile_import/](../src/features/qti_profile_import/) owns the same-origin upload,
  review, acknowledgement, convert, recovery, and editor-handoff UI.
- [crates/server/src/qti_profile_postgres_live.rs](../crates/server/src/qti_profile_postgres_live.rs)
  and [tests/e2e/e2e_database_baseline.sh](../tests/e2e/e2e_database_baseline.sh) own the disposable
  PostgreSQL 17 profile-to-native acceptance oracle with real RLS roles, grading, provenance,
  retention, and exact cleanup.

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
|  +- course_appearance.rs   Revisioned appearance, banner promotion, and cleanup contract
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

Course appearance persistence and current-banner delivery use five focused owners:

- `crates/learning-data-access/src/course_appearance.rs`
- `crates/learning-data-access/src/in_memory/course_appearance.rs`
- `crates/learning-data-access/src/postgres/course_appearance.rs`
- `crates/learning-data-access/tests/conformance/course_appearance.rs`
- `crates/learning-data-access/tests/postgres_course_appearance_live.rs`

Course appearance HTTP behavior uses three server owners:

- `crates/server/src/course_appearance.rs` for authenticated GET, PUT, candidate orchestration, and
  bounded request-triggered cleanup
- `crates/server/src/course_appearance/image.rs` for bounded orientation, crop, and WebP output
- `crates/server/src/course_appearance/tests.rs` for recovery, refusal, ETag, current delivery, and
  combined PostgreSQL/MinIO cleanup

Course-scoped browser theming uses these focused owners:

- `src/features/course_appearance/theme_catalog.ts` for all 15 anchors and accessible projections
- `src/features/course_appearance/course_theme_route.ts` for pure course-route classification
- `src/features/course_appearance/course_theme_scope.tsx` for the pre-render loader and subtree
- `src/features/course_appearance/course_theme_context.ts` for route-data reuse without router
  coupling
- `src/features/course_appearance/course_theme_scope_styles.ts` for scoped and forced-color rules
- `src/features/course_appearance/course_appearance_model.ts` for the pure editable draft,
  validation, and exact atomic update
- `src/features/course_appearance/course_appearance_repository.ts` for the narrow API client seam
- `src/features/course_appearance/course_appearance_page.tsx` and
  `course_appearance_styles.ts` for the complete instructor workflow and responsive presentation
- `src/features/course_appearance/course_entry_identity.tsx` for the text title and optional
  entry-only learner banner
- `tests/test_course_appearance_settings.mjs` for draft, upload, CAS, and transport behavior
- `tests/test_course_theme_scope.mjs` for fail-closed/token/contrast behavior
- `tests/playwright/course_appearance.spec.ts` for keyboard, recovery, responsive, forced-color, and
  axe acceptance
- `tests/playwright/course_appearance_visual.spec.ts` for the reproducible contact sheet,
  screenshots, rendered metrics, and OKLab dedup table
- `tests/playwright/course_theme_scope.spec.ts` for built scope, cleanup, entry-only banner, shell,
  and rendered contrast
- `tests/e2e/e2e_course_appearance.sh` and `compose.course-appearance.yaml` for isolated PostgreSQL,
  MinIO, and their combined claim/delete/complete acceptance

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
  `imathas_backend.rs`; `webwork_backend.rs` resolves PLE-owned immutable PG
  source and its render cache under tenant authorization before private
  rendering or grading. Private grading material remains server-side.

## Project tools

[crates/project-tools/](../crates/project-tools/) contains repository-only project tools. The
Cargo package name is `project-tools`; the contributor command is `cargo tools`.

| Command                        | Purpose                                                            |
| ------------------------------ | ------------------------------------------------------------------ |
| `cargo tools tsgen`            | Generate browser TypeScript contracts from Rust models             |
| `cargo tools fixtures --check` | Verify tracked fixture evidence and refresh its ignored projection |
| `cargo tools bindgen ...`      | Generate version-matched browser and Node WebAssembly glue         |
| `cargo tools database ...`     | Report, apply, or verify the ordered SQLx migration set explicitly |
| `cargo tools e2e-seed ...`     | Seed the standard E2E contract or the immutable WeBWorK PGML pilot |

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

[schemas/migrations/](../schemas/migrations/) contains the accepted six-file pre-data baseline,
ordered by domain:

1. Principals and authentication.
2. Catalog and authoring.
3. Courses and assignments.
4. Activity and feedback.
5. Operations and analytics.
6. Retention.

The first forward file, `schemas/migrations/2026080907_course_appearance.sql`, adds revisioned course
appearance, candidate cleanup state, current-banner delivery constraints, and forced RLS. Future
schema changes continue as forward migrations; a migration-owned trigger enforces the current
delivery's exact banner kind, tenant, and course, and the six baseline files are no longer edited in
place.
The implemented table relationships, proposed production identity tables, fall-pilot row estimates,
FERPA isolation layers, and measured growth thresholds are documented in
[docs/DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).

## Tests

- [tests/](../tests/) contains fast repository hygiene checks in `test_*.py`.
- `tests/test_*.mjs` contains deterministic browser-contract and component
  behavior tests without a live browser.
- [tests/playwright/](../tests/playwright/) tests built browser behavior over
  HTTP.
- [tests/e2e/](../tests/e2e/) owns slower whole-system, replica, WebAssembly,
  and disposable PostgreSQL gates.
- `tests/e2e/e2e_webwork_render_rpc.sh` owns the opt-in source-pinned private
  WeBWorK semantic path through PLE; `tests/playwright/webwork_run.spec.ts`
  owns its browser-only network and keyboard boundary.
- `tests/test_webwork_renderer_container.py` inspects the optional private
  compose profile, source pins, secrets, and network isolation without
  claiming a live image build.
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
  implementation states.
- [docs/DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) maps the PostgreSQL
  tables and pilot-to-scale growth path.
- [docs/SECURITY_MODEL.md](SECURITY_MODEL.md) records trust, authentication,
  grading, tenant, and object-delivery boundaries.
- [docs/CONTAINER.md](CONTAINER.md) documents the local container lifecycle;
  [docs/MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md) owns replica scaling,
  shared-state configuration, failure behavior, and planned production shape.
- [docs/active_plans/decisions/secure_question_grading_payload_plan.md](active_plans/decisions/secure_question_grading_payload_plan.md)
  owns the accepted pre-WP-RC5 render/response payload cutover.
- `docs/active_plans/active/source_module_decomposition_plan.md` owns the post-WP-RC3 capability
  extraction and permanent source-size gate.
- [docs/ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md](ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md)
  records the student no-mouse task model, response-family key contracts,
  corrections, limitations, and executable acceptance evidence.
- [docs/HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) records durable owner
  decisions.
- [docs/active_plans/implementation_plan.md](active_plans/implementation_plan.md)
  is the architecture and milestone source of truth.
- [docs/active_plans/project_status_report_2026-08-09.md](active_plans/project_status_report_2026-08-09.md)
  is the current dated status snapshot; `docs/active_plans/partial_commit_status.md`
  is a historical handoff record, not a competing status authority.
- `docs/active_plans/workstreams/webwork_shipped_integration.md` owns the
  explicit WP-RC3 source, private-stack, and acceptance contract.

## Where to add new work

- Add domain rules to the owning focused file under `crates/domain/src/`.
- Add persistence as a contract plus in-memory, PostgreSQL, and conformance
  modules under the learning data-access subtree.
- Add HTTP behavior to the owning server route module and keep composition thin.
- Add browser behavior to the owning feature, component, page, or API module.
- Add question-format behavior behind an adapter; keep grading server-only.
- Add an upstream WeBWorK response family only through the shipped renderer
  contract and exact parser tests; retain the private `render_rpc` service and
  immutable PGML source boundary.
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
