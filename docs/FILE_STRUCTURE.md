# File structure

This map names each area by its responsibility first and gives the current code
path second. The system architecture and security boundaries are documented in
[CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md). Package and release status comes
from the active [release_completion_plan.md](active_plans/active/release_completion_plan.md)
and the dated [project_status_report_2026-08-10.md](active_plans/reports/project_status_report_2026-08-10.md):
WP-RC1, WP-RC2, WP-RC3, and WP-ARCH1 are accepted. WP-RC3 is a bounded local
WeBWorK integration, not broad OPL compatibility. WP-RC4's PLE flat JSON v2
implementation is present for all eight families and awaits independent
closeout; later release packages describe remaining work, not files already
present in this tree.

## Top-level layout

```text
.
+- crates/             Rust product crates and repository project tools
+- src/                SolidJS and TypeScript browser application
+- schemas/            PostgreSQL baseline migrations
+- containers/         Local PostgreSQL, MinIO, API, gateway, and private PG-renderer stack
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
public/private content, and performs server-only evaluation. Its
`flat_question/v2.rs` child owns the closed MC, MA, FIB, MULTI-FIB, NUM,
MATCH, ORDER, and HOTSPOT source shapes; version 1 single-choice remains in the
facade-compatible owner so its historical bytes do not change.

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
`crates/adapters/webwork/src/renderer_contract.rs`; its HTTP facade is
`http_renderer.rs`, while `http_renderer/{client,protocol,response_shape,html_projection,grade}.rs`
own the focused implementation. Fixed upstream details are in
`standalone_render_api.rs`, and safe cached markup passes through `sanitizer.rs`.
The HTTP implementation uses
`html5ever` to project the one supported single-radio group and refuses the
other upstream control shapes. `crates/server/src/webwork_backend.rs` resolves
the immutable catalog source and joins the adapter to the run backend without
exposing source, upstream field names, or credentials to the browser.

The similarly named reference trees have distinct roles:

- `OTHER_REPOS/pg/` documents the PG/PGML execution engine;
- `OTHER_REPOS/webwork-pg-renderer/` mirrors the maintained standalone HTTP
  renderer at `../webwork-pg-renderer`; and
- `OTHER_REPOS/webwork2/` documents the full homework/course application used
  by the current RC3 compatibility path.

Every `OTHER_REPOS/` path is reference-only. Contributor code must use a pinned
maintained upstream source or image, never an `OTHER_REPOS/` build context,
import, mount, or runtime path.

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
|  +- contracts/             Store, catalog, course, run, and worker contracts
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
|  +- in_memory.rs           In-memory data-access composition facade
|  +- in_memory/             In-memory capability implementations
|  |  `- runs/attempt_issuance.rs Attempt, presentation, timing, prefetch, and replay issuance
|  +- postgres.rs            PostgreSQL data-access composition
|  `- postgres/              PostgreSQL capability implementations
|     `- runs/attempt_issuance.rs Transactional attempt issuance and replay persistence
`- tests/
   +- conformance.rs         Shared conformance facade and broad activity cases
   +- conformance/           Capability-focused conformance cases
   +- postgres_flat_import_provenance_live/  Split live provenance cases
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

- `auth.rs` plus `catalog/{routes,query,response,publication,lifecycle,capabilities}.rs` for
  catalog HTTP and publication behavior behind stable facades.
- `course/{routing,queries,assignments,policy,projection}.rs` and
  `workspace/{crud,publication,state,support}.rs` for focused course and authoring behavior.
- `run/{routes,queries,prefetch,submission,contracts,manual_grading,external_tool}.rs` for
  focused run-domain routes and state behavior.
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
The implemented, acceptance-open course-level roster, PLE-owned passwordless identity,
invitation, bulk import, manual grade export, and atomic membership-to-assignment
reconciliation contract is in [docs/ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md).

## Tests

- [tests/](../tests/) contains fast repository hygiene checks in `test_*.py`.
- `tests/test_*.mjs` contains deterministic browser-contract and component
  behavior tests without a live browser.
- [tests/playwright/](../tests/playwright/) tests built browser behavior over
  HTTP.
- [tests/e2e/](../tests/e2e/) owns slower whole-system, replica, WebAssembly,
  and disposable PostgreSQL gates.
- `tests/e2e/e2e_webwork_render_rpc.sh` owns the live private standalone-renderer
  semantic path through PLE; `tests/playwright/webwork_run.spec.ts`
  owns its browser-only network and keyboard boundary.
- `tests/test_webwork_renderer_container.py` inspects the normal Compose
  renderer boundary, absence of SQL/volumes/host ports, and network isolation
  without claiming live behavior.
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

Start with [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) for the reasoning behind a boundary, then
use this three-tier guide to find its authority and implementation detail.

1. **Source authorities.** [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) records durable owner decisions;
   [implementation_plan.md](active_plans/implementation_plan.md) and the active release plan own
   scope and acceptance; [CONTRACTS.md](CONTRACTS.md), schemas, and the named code owner define
   what a change must preserve.
2. **Decision and contract maps.**
   - Learning activity: [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md),
     [MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md),
     [QUESTION_MODEL.md](QUESTION_MODEL.md), and [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md).
   - Browser and backend traffic: [API_CONTRACTS.md](API_CONTRACTS.md),
     [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md),
     [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md), and
     [CACHING_AND_PREFETCH.md](CACHING_AND_PREFETCH.md).
   - Trust and data: [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
     [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md), [DATA_CONTRACTS.md](DATA_CONTRACTS.md), and
     [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md).
   - Durable operations: [CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md),
     [FAILURE_RECOVERY.md](FAILURE_RECOVERY.md), [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md),
     and [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).
3. **Operating and reference documents.** [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md),
   [SECURITY_MODEL.md](SECURITY_MODEL.md), [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md),
   [DATABASE_TENANCY.md](DATABASE_TENANCY.md), [OBJECT_STORAGE.md](OBJECT_STORAGE.md),
   [MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md),
   [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md),
   [LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md),
   [FRONTEND_ARCHITECTURE.md](FRONTEND_ARCHITECTURE.md),
   [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md), and
   [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md) document implementation,
   local operation, accessibility, and external integrations.

Use the dated [project_status_report_2026-08-10.md](active_plans/reports/project_status_report_2026-08-10.md)
for accepted current state. The Aug. 9 report and partial-commit status are historical records, not
competing authorities.

## Where to add new work

- Add domain rules to the owning focused file under `crates/domain/src/`.
- Add persistence as a contract plus in-memory, PostgreSQL, and conformance
  modules under the learning data-access subtree.
- Add HTTP behavior to the owning server route module and keep composition thin.
- Add browser behavior to the owning feature, component, page, or API module.
- Add question-format behavior behind an adapter; keep grading server-only.
- Add an upstream WeBWorK response family only through the private renderer
  contract and exact parser tests. Preserve immutable PGML source ownership,
  use the standalone `webwork-pg-renderer` target, and do not add WeBWorK2
  course or assignment state.
- Add repository automation under the project-tools subtree and expose a
  `cargo tools` command.
- Keep every new source file below 1000 lines and split by capability before a
  parent becomes an implementation warehouse.

The principal WP-ARCH1 contributor paths are:

- persistence: `crates/learning-data-access/src/contracts/`, `in_memory/`,
  `postgres/`, and `tests/conformance/`;
- server: `catalog/`, `course/`, `workspace/`, and `run/` route, query, policy,
  projection, CRUD, prefetch, and submission owners behind stable facades;
- server support: `retention/{access,parsing,projection,routes}.rs` owns retention access and HTTP
  behavior; `composition/{backend,local_identity,router,settings,worker}.rs` owns dependency
  assembly; and `imathas_backend/{launch_state,projection,provider_dispatch,submission}.rs` owns
  the brokered provider boundary behind their stable facades;
- adapters and tools: child modules beside the existing adapter facades,
  `crates/project-tools/src/e2e_seed/`, and `devel/bump_version/`;
- browser: `src/api/decoders/`, `src/api/http_client/`,
  `src/api/mock/handlers/{auth,catalog,courses,runs,authoring,assets}.ts`,
  `src/components/responses/` family controllers, and
  `src/components/response_widget/` keyboard/external-tool extensions.

These paths describe capability ownership. The stable parent modules remain
the consumer-facing imports, and the permanent test protects the size boundary
rather than freezing this exact module list.

## Naming convention

Cargo packages and crate directories use descriptive hyphenated names, such as
`learning-data-access` and `project-tools`. Rust imports and modules use the
language-native underscore forms, such as `learning_data_access` and
`in_memory`. The naming migration updated manifests, imports, scripts, tests,
and documentation as one atomic package.
