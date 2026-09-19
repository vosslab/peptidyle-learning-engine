# File structure

[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) owns product meaning. Assignment appears
only in the three Assessment Type names. Generic teaching objects are
Assessments (`ple_data.assessment`).

This map identifies the owning location for current PLE behavior. The design
boundaries are in [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md); product meaning
is in [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md).

## Top-level layout

```text
.
+- crates/                  Rust workspace: model, services, stores, tools, and adapters
+- src/                     SolidJS browser application
+- schemas/                 Canonical PostgreSQL structure and installation data
+- content/                 Reviewed teaching content, including Pilot Question sources
+- containers/              Podman Compose definitions and service images
+- deploy/opentofu/         Future infrastructure design artifacts, not runtime evidence
+- local_stack_control/     Disposable-stack lifecycle and acceptance helpers
+- tests/                   Deterministic, connected, and browser evidence lanes
+- docs/                    Durable references, changelog, and active work records
+- generated/               Ignored Rust-derived declarations and fixtures
+- devel/                   Maintainer commands
+- launchers/               Contributor and Live Demo entry points
+- tools/                   Standalone repository utilities
+- schema_style/            PostgreSQL schema-style checker
+- Cargo.toml               Rust workspace manifest
+- package.json             TypeScript and browser tooling manifest
+- build.sh                 Product build entry point
+- check_rust.sh            Rust verification entry point
+- check_codebase.sh        Browser type, lint, format, and test entry point
`- source_me.sh             Python environment activation helper
```

`OTHER_REPOS/` contains reference snapshots. It is not a product source, runtime, or import path.
The reviewed local WeBWorK renderer is the sibling
`../webwork-pg-renderer/` checkout, built by Podman through
[renderer.py](../local_stack_control/renderer.py); it is deliberately outside PLE's product
tree.

## Schema and data

```text
schemas/
+- base_schema/
|  +- install.sql                    Ordered, DDL-only PostgreSQL 17 manifest
|  +- 00_roles.sql                   Bootstrap roles and schemas
|  +- 10_types.sql                   Enums and public-ID domains
|  +- 15_table_check_functions.sql   Domain CHECK helpers
|  +- 20_tables/                     CREATE TABLE and COMMENT ON only
|  +- 22_reference_grants.sql        REFERENCES grants
|  +- 30_constraints.sql             Cross-file foreign keys
|  +- 40_indexes.sql                 Secondary indexes
|  +- 50_functions/                  SQL functions and triggers
|  +- 60_policies.sql                RLS enablement; includes 60_policies/
|  +- 60_policies/                   Row-level security policies
|  +- 70_grants.sql                  Privileges; includes 70_grants/
|  `- 70_grants/                     EXECUTE and table privileges
+- installation_data/
|  +- prepublication_context.sql      Temporary publication context
|  +- bundled_curriculum_context.sql  Bundled publisher Account and workspace
|  +- content_vocabulary.sql          Shared Discipline and Subject vocabulary
|  +- install.sql                     Data-only Live Demo manifest
|  +- live_demo.sql                   Ordinary teaching graph
|  +- live_demo_oracle.sql            Convergence and completeness checks
|  `- revoke_example_content_session.sql  Publisher session teardown
`- migrations/
   `- .gitkeep                        Reserved SQLx forward-migration directory
```

[20_tables/](../schemas/base_schema/20_tables/) holds every `CREATE TABLE`,
including [assessment.sql](../schemas/base_schema/20_tables/assessment.sql),
[assessment_attempt.sql](../schemas/base_schema/20_tables/assessment_attempt.sql)
(saved responses finalize in place on `assessment_attempt_saved_response`),
[question_pool.sql](../schemas/base_schema/20_tables/question_pool.sql)
(current Pool plus Edit Number; no Pool Revision family),
[published_question.sql](../schemas/base_schema/20_tables/published_question.sql),
[account.sql](../schemas/base_schema/20_tables/account.sql),
[course_instance.sql](../schemas/base_schema/20_tables/course_instance.sql),
and [blueprint_course.sql](../schemas/base_schema/20_tables/blueprint_course.sql).
Account, Course Instance, Assessment, Blueprint Course, Published Question, and
Question Pool use the public ID as the only stored identity.

[schemas/base_schema/README.md](../schemas/base_schema/README.md) defines the
editable pre-production and frozen-production boundary. Base layers own final
structural state directly.
[schemas/installation_data/README.md](../schemas/installation_data/README.md)
defines the separate data phase. Its `provision` command creates complete
ordinary Live Demo product data; `apply` is the narrower database-owned graph.
Before the production freeze, structural corrections belong in the owning base
layer; afterward, forward-only SQLx migrations belong in
[schemas/migrations/](../schemas/migrations/). SQLx configuration belongs to
[sqlx.toml](../crates/learning-data-access/sqlx.toml).

## Rust workspace

| Path | Purpose |
| --- | --- |
| [crates/question_model/](../crates/question_model/) | Shared product concepts: Question IDs and Revisions, current Question Pools with Edit Number, Blueprint Revisions, current Assessments, and retained Student Work evidence. |
| [crates/domain/](../crates/domain/) | Pure validation, timing, policy, scoring, disclosure, and generation behavior. |
| [crates/grading/](../crates/grading/) | Server-only answer-bearing checkers. |
| [crates/learning-data-access/](../crates/learning-data-access/) | Store traits, PostgreSQL implementations, SQLx forward-migration ledger support, and schema verification. |
| [crates/server/](../crates/server/) | Axum HTTP routes, authentication, authorization, and service composition. |
| [crates/browser-api-contract/](../crates/browser-api-contract/) | Browser-safe Rust contract roots for TypeScript generation. |
| [crates/adapters/](../crates/adapters/) | PLE, WeBWorK, iMathAS, and QTI backend or import adapters. |
| [crates/objects/](../crates/objects/) | Object-address, checksum, image, and object-store ownership. |
| [crates/wasm/](../crates/wasm/) | Answer-free Rust-to-browser WebAssembly facade. |
| [crates/export/](../crates/export/) | PDF and DOCX export models and writers. |
| [crates/project-tools/](../crates/project-tools/) | TypeScript generation, database lifecycle commands, Pilot publication, and installation-data tooling. |
| [crates/acceptance-runtime/](../crates/acceptance-runtime/) | Disposable acceptance database connection handoff. |

The Course Blueprint update review is split deliberately: its public Store types live in
[assessment_release.rs](../crates/learning-data-access/src/assessment_release.rs), its PostgreSQL
transaction lives in
[assessment_blueprint_update.rs](../crates/learning-data-access/src/postgres/assessment_blueprint_update.rs),
and the matching base-schema boundary is
[assessment_blueprint_updates.sql](../schemas/base_schema/50_functions/assessment_blueprint_updates.sql). Its lazy
Course summary lives in [course_blueprint_update_review.tsx](../src/pages/course_blueprint_update_review.tsx)
and lists adopted Assessment correspondences only. It introduces no persisted offer, receipt,
comparison baseline, new update table, or whole-Course lifecycle module.

WeBWorK's opaque adapter boundary is owned by
[crates/adapters/webwork/](../crates/adapters/webwork/). Its renderer contract,
HTTP client, source resolution, issuance, and stateless grading remain there;
it contains no HTML projection or PG-control matching module. Generic
backend-owned response and presentation types belong in
[crates/question_model/](../crates/question_model/), and
[crates/learning-data-access/](../crates/learning-data-access/) owns immutable
`backend_document` persistence and the authorized document read seam.

The database command front door is [database.rs](../crates/project-tools/src/database.rs):
`cargo tools database initialize`, `migrate`, and `verify`. The implementation coordinator is
[database_coordinator.rs](../crates/project-tools/src/database_coordinator.rs).
Installation data is owned by [installation_data.rs](../crates/project-tools/src/installation_data.rs).

## Browser application

```text
src/
+- api/                         Typed client contracts, HTTP client, and strict decoders
+- auth/                        Browser session and sign-in support
+- components/                  Shared answer-free UI and Question presentation
|  `- question_response_controls/backend_owned_document.tsx
|                               Generic iframe host and opaque form-pair capture
+- features/
|  +- blueprint_course/         Blueprint Revision editing and explicit Save UI
|  +- blueprint_change_proposal/ Blueprint Course Change Proposal UI
|  +- blueprint_forks/          Blueprint fork review and apply UI
|  +- question_picker/          Published, non-archived Question selection UI
|  +- question_attempt/         Student Attempt interactions
|  +- course_appearance/        Authorized course appearance UI
|  +- ple_question_json_authoring/ Native PLE Question JSON editor
|  `- profile_avatar/           Provided avatar and profile-image UI
+- pages/
|  +- assessment_workspace/     Assessment edit, release, and Unrelease UI
|  `- assignment_access/        Unused leftover helper; not Student Coursework UI
+- ribbon/                      Capability-aware navigation catalog and rendering
+- styles/                      Browser-wide styles and local font declarations
+- wasm/                        Browser bridge modules
+- routes.ts                    Executable route map
`- application_shell.tsx        Shared application shell and accessibility boundary
```

The Course-level adopted-Assessment summary is
[course_blueprint_update_review.tsx](../src/pages/course_blueprint_update_review.tsx); it links to the
existing detail review presentation,
[assessment_blueprint_update_review.tsx](../src/pages/assessment_workspace/assessment_blueprint_update_review.tsx);
the existing Question Editor owns its Review, Apply, and Cancel controls. The browser uses the
existing Assessment release client rather than a separate update client.

[src/api/decoders/](../src/api/decoders/) is the runtime DTO boundary. Generated declarations in
`generated/api/` are derivative; modify their Rust source and regenerate rather
than editing them. Public-ID JSON fields are `id`. Question Pools use sibling
`questionPoolId` and `questionPoolEditNumber` (or snake_case
`question_pool_id` / `question_pool_edit_number` on Blueprint stored content).
Question Revision pins remain `{questionId, revisionNumber}`.

`src/public/ple_bridge.js` is copied into the browser build. Same-origin Student
documents transport an entire backend-owned form as ordered string pairs; opaque
WeBWorK previews emit only their bounded resize record. `src/styles/ple_embed.css`
supplies the generic iframe baseline; it contains no WeBWorK or control-type
selectors.

## Local stack and deployment

```text
containers/
+- Containerfile.api             API image
+- Containerfile.gateway         Gateway image
+- Caddyfile                     Local TLS reverse proxy
`- compose.yaml                  Local Compose services

local_stack_control/
+- cli.py                        Typed local-stack command interface
+- lifecycle.py                  Stack lifecycle coordination
+- lifecycle_database.py         Database initialization and verification path
+- renderer.py                   Reviewed sibling renderer selection and Podman build
+- live_demo_seed.py             Local Live Demo environment and selector support
+- browser_suite_developer.py    Browser-suite supervisor (lease owner, control socket)
`- browser_suite_developer_start.py  Parent-side start: progress heartbeat, stall detection

deploy/opentofu/
`- DATABASE_PROVISIONING.md      Future database-provisioning design runbook
```

The migrator image contains the base manifest, installation-data manifest, and forward migrations.
Runtime API and worker images do not carry the PostgreSQL client or DDL authority.
OpenTofu is not a current cloud deployment or acceptance path.

`devel/webwork_render_probe.py` is a one-time connected renderer-evidence tool.
It accepts explicit PG/PGML input or temporary input, records opaque document,
asset, and one-grade-request observations, and creates no tracked question
corpus or fixture.

## Tests and generated output

```text
tests/
+- test_*.py                     Fast Python repository and lifecycle checks
+- test_*.mjs                    Fast Node contract and browser-model checks
+- e2e/                          Disposable PostgreSQL and service acceptance
+- playwright/                   Browser and screenshot evidence
`- fixtures/                     Small durable fixtures

generated/
+- api/                          Ignored generated TypeScript declarations
`- fixtures/                     Ignored generated test data
```

Build output in `dist/`, `dist_wasm/`, `target/`, and `test-results/` is generated and ignored.
Use [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) to select an appropriate verification lane;
connected and browser evidence are not substitutes for deterministic contract tests.

## Documentation map

- [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md): human product priorities and operating guidance.
- [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md): product vocabulary and semantic contract.
- [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md): database catalog and lifecycle details.
- [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md): PostgreSQL role and authorization model.
- [CONTRACTS.md](CONTRACTS.md): durable module and API contract index.
- [DEVELOPMENT.md](DEVELOPMENT.md): contributor workflow and local development constraints.
- [GMAIL_EMAIL_DELIVERY_BACKEND.md](GMAIL_EMAIL_DELIVERY_BACKEND.md): planned provider-neutral
  email delivery contract, Gmail API adapter, operator setup, and secret-storage boundary.
- [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md): permanent-test and acceptance-evidence policy.

## Where to add work

- Put a schema correction in its owning [schemas/base_schema/](../schemas/base_schema/) layer
  while the base remains editable; later structural evolution belongs in
  [schemas/migrations/](../schemas/migrations/).
- Put database-owned installation records in `schemas/installation_data/`.
- Put new durable concepts in [crates/question_model/](../crates/question_model/) or
  [crates/domain/](../crates/domain/), then storage in [crates/learning-data-access/](../crates/learning-data-access/).
- Put HTTP composition in [crates/server/](../crates/server/) and pair browser clients with strict
  decoders in [src/api/](../src/api/).
- Put deterministic regression tests beside the contract they protect; use [tests/e2e/](../tests/e2e/)
  only for connected database or service behavior.
