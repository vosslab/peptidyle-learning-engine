# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 18 entries are archived in [CHANGELOG-2026-09n.md](CHANGELOG-2026-09n.md).

## 2026-09-20

### Features

- Local Stack start and `python3 local_stack.py doctor` refuse a second
  running copy of postgres, MinIO, the WeBWorK renderer, api, worker,
  public-asset-publisher, or gateway across the default `containers`
  project and `ple-live-demo-browser`. Gate:
  `python3 -m pytest tests/test_local_stack_service_singletons.py`.

- Warm screenshot loop: `./devel/capture_screenshots.sh` on a running Live Demo
  rebuilds only the stale client, wasm+client, or application image group, then
  publishes the full corpus and reports PNG changes (or `no visual change`).
  `python3 local_stack.py rebuild-application` recreates api, worker, and
  public-asset-publisher without touching PostgreSQL, MinIO, or the renderer.
  Capture membership lives in scenario `captureCheckpoint` declarations; the
  committed manifest, coverage ledgers, and atlas order are generated.
  `coverage_exceptions.json` is the only hand-kept coverage list. M0 kept the
  `wasm_client` row. Gate: `python3 -m pytest tests/test_change_scope.py`,
  `node --import tsx --test tests/test_screenshot_corpus_definition.mjs`,
  `bash tests/e2e/e2e_screenshot_warm_loop.sh`.

### Fixes and Maintenance

- Semantic naming: exact identities use `Id`, exact Question/Blueprint
  revisions use named Tuples, and clocks use qualified Edit/Revision
  Numbers. Known forks, Assessment Blueprint Update, Student Work Recovery,
  Question Asset Tuple, Object Address members, and Assessment workspace
  JSON now ship those shapes. HTTP `ETag`/`If-Match` remain header
  encoding only. A compact contract registry replaces the broad domain-ETag
  spelling pytest. Gate: `node --import tsx --test
  tests/test_nested_identity_contracts.mjs`, `python3 -m pytest
  tests/test_semantic_contract_registry.py`,
  `cargo test -p question_model -p browser-api-contract -p objects
  --offline --lib`.

- Rotated 2026-09-18 changelog entries into
  [CHANGELOG-2026-09n.md](CHANGELOG-2026-09n.md).

- Warm screenshot `api_image_created_at` parses podman `image inspect`
  `{{.Created}}` Go `time.String()` stamps such as
  `2026-09-20 07:03:03.033094562 +0000 UTC`, so a TypeScript-only warm
  run stays `client` / `none` instead of treating crates as stale.
  Gate: `python3 -m pytest tests/test_change_scope.py`.

- Instructor, Student, and Sysadmin screenshot scenarios declare `role`
  and one `captures` row per checkpoint, then call
  `runtime.open(checkpoint)` and
  `runtime.captureCheckpoint(session, checkpoint)`. Gate:
  `npx eslint --max-warnings 0
  tests/playwright/screenshot_corpus/scenarios_*.ts`,
  `node --import tsx --test tests/test_screenshot_corpus_definition.mjs`
  (registry uniqueness).

- Nested Course Instance identities use `course_instance_id` /
  `courseInstanceId` across Rust store/domain structs, Object Address
  JSON, SQL object-address keys, and browser contracts. Nested
  Assessment and Assessment Attempt identities use `assessment_id` /
  `assessmentId` and `assessment_attempt_id` /
  `assessmentAttemptId`. Leftover JSON `courseId` is rejected.
  `ple_api.lock_archived_course_for_recovery` returns one
  `course_instance_id` column. Gate:
  `node --import tsx --test tests/test_nested_identity_contracts.mjs
  tests/test_live_gradebook_decoder.mjs
  tests/test_assignments_due_soon_client.mjs`,
  `source source_me.sh && ./launchers/run_fast_checks.sh`.

## 2026-09-19

### Fixes and Maintenance

- Course Instance adoption and provenance use named Blueprint Revision
  Tuples (`blueprintRevisionTuple`, `adoptedBlueprintRevisionTuple`,
  `currentBlueprintRevisionTuple`). Nested client identities use
  `courseInstanceId` / `assessmentId` / `assessmentEntryId` /
  `targetBlueprintCourseId`. Pool fork clocks use
  `QuestionPoolEditNumber`. SQL stale-concurrency messages name Edit
  Number, not ETag. Gate: `node --import tsx --test
  tests/test_course_instance_summary.mjs
  tests/test_nested_identity_contracts.mjs`,
  `source source_me.sh && python3 -m pytest
  tests/test_semantic_boundary_names.py`.

- SQL keeps `p_` parameter and `v_` local prefixes. Ambiguous domain values
  after those prefixes now name the owning clock:
  `p_expected_blueprint_revision_number`,
  `p_expected_assessment_edit_number`,
  `p_expected_draft_question_edit_number`,
  `p_expected_assessment_template_edit_number`,
  `p_expected_accommodation_edit_number`,
  `p_expected_question_availability_edit_number`,
  `v_next_blueprint_edit_number`,
  `v_result_blueprint_revision_number`. Gate:
  `source source_me.sh && python3 devel/generate_schema_tables_doc.py &&
  schema_style/check_schema_style.py`.

- Nested public JSON identities use `assessmentId` / `courseInstanceId` /
  `assessmentAttemptId`. The leftover `listCourses` `/api/courses` client is
  gone. Live Course summary, appearance, roster, and banner routes use
  `/api/course-instances/{course_instance_id}` and
  `{course_banner_id}`. Gate: `npx tsc --noEmit -p tsconfig.json`,
  `node --import tsx --test tests/test_blueprint_course_client.mjs
  tests/test_question_availability_client.mjs
  tests/test_course_instance_summary.mjs
  tests/test_blueprint_stewardship_client.mjs
  tests/test_assignment_client.mjs
  tests/test_live_assignment_release_validation.mjs
  tests/test_assessment_template_client.mjs
  tests/test_ple_question_json_authoring.mjs
  tests/test_assessment_attempt_navigation.mjs
  tests/test_assessment_attempt_history_decoder.mjs`.

- Course Instance load selects `adopted_blueprint_revision_number` /
  `current_blueprint_revision_number`. Genetics receipt JSON is
  `blueprintRevisionNumber`. Assessment decode helper takes
  `revision_number`. Blueprint exact-Revision route in API_CONTRACTS is
  `{revision_number}`. Gate: `cargo test -p learning-data-access
  --features postgres --lib load_course_instance_selects_revision_number_columns
  question_revision_tuple_takes_a_revision_number`, `cargo test -p
  project-tools --bin project-tools
  genetics_receipt_serializes_blueprint_revision_number`.

- Remaining Tuple JSON fields use Tuple names: Change Proposal
  `sourceRevisionTuple` / `targetRevisionTuple` /
  `expectedTargetRevisionTuple`, fork apply
  `expectedSourceRevisionTuple` / `expectedForkRevisionTuple`,
  stewardship `questionRevisionTuple` / `forkRevisionTuple`, and
  capability `questionRevisionTuple`. Library Question detail uses
  `?revisionNumber=`. PLE Question JSON draft concurrency uses
  `draftQuestionEditNumber`. Attempt response edits use
  `editGeneration`, not Revision. Gate: `npx tsc --noEmit -p
  tsconfig.json`, `node --import tsx --test
  tests/test_ple_question_json_authoring.mjs
  tests/test_assessment_attempt_response_state.mjs`.

- Student-view and Blueprint Course routes use `{revision_number}` for a lone
  Revision Number. Helpers that return or take a Question Revision Tuple are
  named Tuple (`row_question_revision_tuple`,
  `content_question_revision_tuples`,
  `requested_question_revision_tuples_*`, `decode_fork_asset` /
  `prepare_hotspot_asset` parameters). Gate:
  `cargo check -p learning-data-access -p server_core`.

- SQL append parameters `p_prior_revision_number` /
  `p_saved_revision_number` are Revision Numbers. LDA `parse_revision_number`
  parses a SQL bigint into `BlueprintRevisionNumber`. Student View
  `verified_question_revision_tuple` takes a Tuple. Gate:
  `cargo check -p learning-data-access -p server_core`.

- Live-demo Course Instance origin JSON is `adoptedRevisionNumber` /
  `currentRevisionNumber`. Question Library and Assessment live-demo JSON
  use `questionRevisionTuple`. SQL `p_expected_revision_number` /
  `v_result_revision_number` hold Revision Numbers. Locals that hold a
  Question Revision Tuple are `expected_revision_tuple`. Gate:
  `cargo check -p learning-data-access -p project-tools`.

- SQL parameters that hold a Revision Number use a Number suffix
  (`p_revision_number`, `p_blueprint_revision_number`). Helpers that parse or
  return a Question Revision Tuple are named Tuple:
  `verified_question_revision_tuple`, `existing_parent_question_revision_tuple`,
  `permitted_answer_revision_tuple`, `successor_question_revision_tuple`. Gate:
  `cargo check -p server_core`.

- Live-demo Blueprint E2E JSON expects `current_revision_tuple` /
  `fork_source_tuple` / `blueprintRevisionTuple` with `revisionNumber`
  members, and Course creation uses `blueprintRevisionNumber`. The Library
  discussion helper is `library_object_current_revision_number`. Keep
  `current_revision_tuple` and `current_revision_matches`. Gate:
  `cargo check -p learning-data-access`.

- The scalar Blueprint Revision Number type is `BlueprintRevisionNumber`,
  matching `QuestionRevisionNumber` and fields such as `revision_number`.
  `BlueprintRevisionTuple` and `BlueprintRevisionView` remain Revision
  objects. Gate: `cargo tsgen`, `npx tsc --noEmit -p tsconfig.json`.

- Internal store names follow the same Revision Number rule as JSON: LDA
  `current_revision_number` / `source_revision_number`,
  `BlueprintRevisionTuple.revision_number`, and SQL API columns such as
  `source_revision_number` and `adopted_blueprint_revision_number`. Store-only
  names are not a carve-out. Gate:
  `cargo check -p learning-data-access -p server_core -p question_model`.

- Lone Blueprint Revision Numbers on remaining browser contracts use a Number
  suffix: history `revisionNumber`, Course Blueprint update
  `adoptedRevisionNumber` / `sourceRevisionNumber`, Assessment Blueprint update
  `sourceRevisionNumber` / `expectedSourceRevisionNumber`. Gate:
  `cargo tsgen`, `npx tsc --noEmit -p tsconfig.json`,
  `node --import tsx --test tests/test_blueprint_course_client.mjs`.

- Course Instance adopted source JSON is `blueprintRevisionNumber`, and Course
  origin `adoptedRevisionNumber` matches `currentRevisionNumber`. LDA postgres
  seeds insert `published_question` and `question_revision`, not a Tuple-named
  table. Tuple decoder and serde tests reject a lone Revision Number under a
  Tuple field. Bloom preparation target kind is SQL `question_revision`. Gate:
  `cargo test -p question_model --lib ordered_content_validation
  revision_tuple`,
  `cargo test -p learning-data-access --lib creation_source_accepts_only`,
  `node --import tsx --test tests/test_question_revision_tuple_decoder.mjs
  tests/test_blueprint_course_client.mjs tests/test_course_instance_summary.mjs`.

- Final remaining-disagreement review of live SQL, Rust, TypeScript,
  generated/api, Terminology Contract, and Human Guidance found no leftover
  dual-meaning identity fields: Tuples stay under Tuple names, lone Revision
  Numbers stay under Number names, Pools use Edit Number, and Assignment
  appears only in Assessment Type names. Gate:
  `source source_me.sh && ./launchers/run_fast_checks.sh`.

- Name Tuple-valued Blueprint fields as Tuples: fixed-entry JSON is
  `question_revision_tuple`, current Blueprint views use
  `current_revision_tuple` / `fork_source_tuple`, comparison sides use
  `currentRevisionTuple`, and Change Proposal sides use
  `blueprintRevisionTuple`. Lone Revision Numbers use
  `currentRevisionNumber` / `sourceRevisionNumber`. Leftover
  `published_question` Tuple JSON is rejected. Gate:
  `cargo test -p question_model --lib ordered_content_validation`,
  `node --import tsx --test tests/test_question_revision_tuple_decoder.mjs
  tests/test_blueprint_course_client.mjs`.

- Rotate 2026-09-17 changelog history into [CHANGELOG-2026-09m.md](CHANGELOG-2026-09m.md) so the active changelog stays under the 1000-line source limit.

- Add mandatory pytest disk budgets for the Rust `target/` build directory and
  active Podman connection so routine testing cannot silently fill the
  developer volume. The normal pytest lane fails when `target/` physical usage
  exceeds 10 GiB or the sum of Podman images, containers, and local volumes
  exceeds 20.00 GB. A clean checkout without `target/` passes. The independent
  checks can be propagated separately. Focused gate:
  `source source_me.sh && python3 -m pytest tests/test_target_disk_budget.py
  tests/test_podman_disk_budget.py`.

- Finish remaining identity alignment for living UI copy, decoder
  errors, and route Path/store locals that still named an Id or Tuple as
  a Reference. Tuple-valued JSON fields are `questionRevisionTuple` and
  `blueprintRevisionTuple`. Course Instance fixtures use
  `COURSE_INSTANCE_ID`. QTI package locators are paths. Remaining living
  `reference` hits are leftover-rejection, SQL `REFERENCES`, English
  prose, Playwright "Reference images", or living-doc term definitions.
  Gate:
  `cargo test -p question_model --lib revision_tuple`,
  `cargo test -p objects --lib object_record_json_shape`,
  `cargo test -p browser-api-contract --lib selected_presentation`,
  `node --import tsx --test tests/test_question_revision_tuple_decoder.mjs
  tests/test_question_pool_metadata.mjs
  tests/test_question_picker_blueprint_course.mjs
  tests/test_course_appearance_view_client.mjs
  tests/test_course_summary_client.mjs
  tests/test_route_params.mjs`,
  `cargo tsgen`, `npx tsc --noEmit -p tsconfig.json`.

- Name Tuple-valued JSON fields `questionRevisionTuple` and
  `blueprintRevisionTuple`, including object-address JSON in SQL, Live Demo
  publication mapping, and generated browser types. Leftover `reference`
  locals, parameters, comments, and test keys that held an Id or Tuple now
  use Id, Tuple, or Path names, including `blueprint_course_id` store
  parameters, route Path locals, and `COURSE_INSTANCE_ID` fixtures. QTI
  package locators are paths. Gate:
  `cargo test -p question_model --lib revision_tuple`,
  `cargo test -p objects --lib object_record_json_shape`,
  `cargo test -p browser-api-contract --lib selected_presentation`,
  `node --import tsx --test tests/test_question_revision_tuple_decoder.mjs
  tests/test_question_pool_metadata.mjs
  tests/test_question_picker_blueprint_course.mjs`,
  `cargo tsgen`, `npx tsc --noEmit -p tsconfig.json`.

- Finish remaining identity contract shape. Blueprint Revision Tuple JSON
  is `{blueprintCourseId, revisionNumber}` everywhere on current contracts,
  matching Question Revision Tuple `{questionId, revisionNumber}`. Leftover
  SQL/Rust holders `read_active_student_assessment_attempt_reference`,
  `import_assessment_question_pool_fork_for_reference`,
  `question_revision_reference`, and `row_reference` are Id/Tuple names.
  Support repair locators are `resource_path` / `resourcePath`. Import
  item locators are `source_item_key`. Naming Conventions now states JSON
  identity and Tuple members are camelCase. Gate:
  `cargo test -p question_model --lib revision_tuple`,
  `node --import tsx --test tests/test_question_revision_tuple_decoder.mjs
  tests/test_blueprint_course_client.mjs
  tests/test_live_assignment_release_validation.mjs`,
  `cargo tsgen`, `npx tsc --noEmit -p tsconfig.json`,
  `python3 devel/generate_schema_tables_doc.py &&
  python3 schema_style/check_schema_style.py`.

- Align identity names with Human Guidance. One canonical object identity
  is an Id (`CourseInstanceId`, `ResponseItemId`, `CourseBannerUploadId`,
  `ImathasDeploymentId`). Multiple values that together identify one exact
  object, state, or version are a Tuple (`QuestionRevisionTuple`
  `{questionId, revisionNumber}`, `BlueprintRevisionTuple` Blueprint
  Course ID plus Revision Number, `QuestionAssetTuple` asset ID plus
  checksum). Route wrappers are `*RouteId`. Course Banner Upload storage
  uses `course_banner_upload_id`. Presentation response-item
  bindings use `presentation_response_item_id` and `response_item_id`.
  TypeScript API and UI helpers take `courseInstanceId`, `blueprintCourseId`,
  and `accountId` instead of `reference` for those identities.
  Issued Question JSON is
  `questionRevision`, not `reference`. Leftover identity JSON
  `assessmentReference` / `blueprintAssessmentReference` is `assessmentId` /
  `blueprintAssessmentId`. Support repair keeps scoped `resourcePath`.
  Install inserts still mint public
  Question IDs; `tests/fixtures/published_question/fixture_set.json` is
  offline type-loading evidence, not frozen install identity. Reference
  remains only a genuine indirect, scoped, or external locator. Gate:
  `cargo test -p question_model --lib revision_tuple`,
  `node --import tsx --test tests/test_question_revision_tuple_decoder.mjs
  tests/test_public_navigation.mjs tests/test_route_params.mjs`,
  `cargo tools fixtures --check`, `cargo tsgen`,
  `npx tsc --noEmit -p tsconfig.json`.

- Drop leftover milestone prefixes from Ribbon Playwright evidence.
  `ribbon_m8_integration`, `ribbon_m9_responsive`, `ribbon_m9b_density`,
  and `ribbon_m10_shell` files are now `ribbon_integration`,
  `ribbon_responsive`, `ribbon_density`, and `ribbon_shell`, with the
  matching harness helpers renamed. Gate:
  `git mv tests/playwright/ribbon_m*.mjs`.

- Align remaining identity names with Human Guidance: HTTP path
  parameters use `course_instance_id` / `assessment_id` /
  `blueprint_course_id` / `account_id` instead of `{reference}`;
  Question Summary JSON is `questionRevision`; browser route params
  are `:courseInstanceId` / `:assessmentId` / `:questionId` /
  `:blueprintCourseId`. Living API and identity contracts no longer
  describe leftover `/assignments` routes as current. Gate:
  `cargo tools tsgen`, `cargo check -p question_model -p server_core
  --offline --tests`, `npx tsc --noEmit -p tsconfig.json`,
  `node --import tsx --test tests/test_route_params.mjs
  tests/test_ribbon_contract.mjs
  tests/test_question_summary_latest_revision_decoder.mjs`.

- Split source files that were at or above the 900-line warning: shared
  responsive composition in `src/style_responsive.css`, Ribbon phone density
  in `src/ribbon/app_ribbon_density.css`, Assessment Blueprint update
  decoding in `src/api/decoders/assessment_blueprint_update.ts`, Question
  Library virtual rows in `src/pages/library_browse_rows.tsx`, and the PLE
  Question JSON editor workspace in
  `src/features/ple_question_json_authoring/question_json_editor_workspace.tsx`.
  Gate: `source source_me.sh && python3 -m pytest
  tests/test_source_file_line_limit.py -q --tb=no -W default`.

- Split four over-900-line Python local-stack modules under 850 lines:
  renderer wait/attestation and installation-data provision from
  `lifecycle.py`, disposable cleanup/outage/redaction helpers from
  `disposable_stack_adapter.py`, and the matching unit-test files.
  `lifecycle.py` remains the public facade. Gate:
  `source source_me.sh && python3 -m pytest
  tests/test_source_file_line_limit.py tests/test_local_stack_control.py
  tests/test_local_stack_lifecycle.py
  tests/test_local_stack_control_cleanup.py
  tests/test_local_stack_lifecycle_restart.py -q`.

- Split four over-900-line `server_core` files under 850 lines: Authoring
  HTTP helpers, Blueprint Course view builders, Live Demo plus env
  composition, and Question Library summaries. Routers and route handlers
  stay in the original modules. Gate: `cargo fmt -p server_core`,
  `cargo check -p server_core --offline --tests`,
  `source source_me.sh && python3 -m pytest
  tests/test_source_file_line_limit.py -q`.

- Split four over-900-line Rust files under 850 lines: PLE Question JSON
  source compile/validate helpers, PLE Question JSON grading validate
  helpers, presentation builder item assembly, and Live Demo activity
  converge. Public compile, shape, build, and provision entry points stay
  in the original modules. Gate: `cargo check -p adapter_ple --offline
  --tests`, `cargo check -p grading --offline --tests`, `cargo check -p
  question_model --offline --lib`, `cargo check -p project-tools
  --offline --tests`, `source source_me.sh && python3 -m pytest
  tests/test_source_file_line_limit.py -q`.

- Split four over-900-line files under 850 lines: Assessment Attempt
  start SQL, changelog day-block parsing, Ribbon M10 shell evidence
  helpers, and remaining PLE Question JSON type codec tests. Start
  functions load after lock/assert helpers. Gate:
  `source source_me.sh && python3 -m pytest
  tests/test_source_file_line_limit.py -q` and
  `node --import tsx --test tests/test_ple_question_json_authoring.mjs
  tests/test_ple_question_json_authoring_types.mjs`.

- Split four over-900-line `learning-data-access` files under 850 lines:
  Assessment delivery renditions, Assessment release decode helpers,
  Blueprint Course decode helpers, and the Blueprint lifecycle connected
  oracle. Owner-only Private discovery assertions live in
  `lifecycle_privacy.rs`. Gate:
  `cargo check -p learning-data-access --features postgres --tests`,
  `python3 -m pytest tests/test_source_file_line_limit.py -q`.

- Drop `course_membership_event_current_lookup_idx`; the UNIQUE btree on
  `(course_membership_id, occurred_at, course_membership_event_id)` already
  serves current-event lookup, including reverse `ORDER BY`. Drop stored
  `question_pool_selection.selected_question_count`; item rows are the
  authoritative Selection, archived recovery JSON no longer copies a
  count, and the deferred trigger requires selected Items rather than a
  copied cardinality. Gate:
  `cargo test -p question_model --lib question_pool_selection_retains`,
  `source source_me.sh && python3 devel/generate_schema_tables_doc.py &&
  python3 schema_style/check_schema_style.py`.

- Add referencing-side indexes in `40_indexes.sql` for every foreign key
  that was not already the leading columns of a PRIMARY KEY or UNIQUE
  constraint. `rule_14_unindexed_fk` is clean; there are no intentional
  unindexed-FK exceptions. Gate: `source source_me.sh && python3
  schema_style/check_schema_style.py`.

- Direct identity cutover: Question Pools are current-state membership under
  one Pool ID plus a sequential Pool Edit Number, stored as sibling fields
  (`question_pool_id` / `question_pool_edit_number`, JSON `questionPoolId` /
  `questionPoolEditNumber`). There is no Pool Pin wrapper and no Pool
  Revision family. JSON and docs treat those as sibling fields; "pin"
  remains the Question Revision `{questionId, revisionNumber}` pair. SQL public-ID parameters and `RETURNS TABLE` columns use
  `*_id` rather than `public_reference`. Dual `question_pool_public_id`
  columns are gone. Unrelease audit counts finalized saved responses as
  `finalized_saved_response_count`. Store parameters that carry public IDs
  use those ID names. Browser decoders accept canonical public IDs (`id`,
  nested `courseId` / `assessmentId` / `blueprintCourseId`) rather than
  UUID or leftover `*Reference` JSON for those objects. Blueprint Assessment
  lineage columns and stored JSON keys are `blueprint_assessment_id`. Pool
  forks during adoption mint only a new Pool ID. SQL locals that hold
  Blueprint Course IDs use `*_blueprint_course_id`. Live Demo e2e and
  install scripts query `account_id` / `course_instance_id` /
  `assessment_id` / `blueprint_course_id` rather than `public_reference`.
  Public-ID mint helpers live in `public_ids.sql`
  (`ple_private.assign_public_id`, `ple_private.crockford_id_suffix`).
  Browser validators are `validateCanonicalPublicId` /
  `normalizeHumanEnteredPublicId`. Human Guidance implementation
  evidence cites `public_ids.sql`, `impl_public_id`, and public-ID
  primary keys. Gate:
  `cargo test -p question_model --lib`,
  `cargo check -p learning-data-access -p server_core --offline --tests`,
  and `node --import tsx --test tests/test_question_pool_discovery_client.mjs
  tests/test_question_pool_metadata.mjs tests/test_bloom_classification_client.mjs
  tests/test_question_pool_creation_client.mjs
  tests/test_assessment_summary_policy_decoder.mjs
  tests/test_assignment_client.mjs`.

- Align Student Course landing, invitation, Assessment list, and Attempt
  context JSON public-ID fields to `id`. Live Demo activity provisioning
  reads those `id` fields. Human Guidance now states that a Pool fork
  starts at Edit Number 1, not Revision 1. Gate: `cargo test -p server
  --lib live_student_course_landing` and `cargo test -p question_model
  --lib blueprint_course`.

- Live Demo and bundled publisher Accounts insert the mint placeholder so
  `ple_private.assign_public_id` issues a random public ID. Email and
  authoring workspace remain the stable lookup. After provision, the local
  stack records the minted persona IDs into the env file and recreates the
  API before Morgan TOTP. Gate: `tests/test_live_demo_target.py`.

- `launchers/run_fast_checks.sh` regenerates schema tables docs and runs
  `schema_style/check_schema_style.py` first. The checker now exits 1 on
  any finding, including `rule_14_unindexed_fk`.
  `launchers/all_test.sh` uses the same schema gate. Docs-only plus launcher
  and checker exit policy.

- Refresh living identity and architecture docs against Human Guidance: Question
  Pools are current membership with an Edit Number, public IDs are the only
  stored identity and JSON `id`, and saved responses finalize in place.
  Touched [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md),
  [FILE_STRUCTURE.md](FILE_STRUCTURE.md),
  [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md),
  [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md),
  [QUESTION_MODEL.md](QUESTION_MODEL.md),
  [CONTRACTS.md](CONTRACTS.md),
  [API_CONTRACTS.md](API_CONTRACTS.md),
  [DATA_CONTRACTS.md](DATA_CONTRACTS.md),
  [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md),
  [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
  [BLOOM_TAXONOMY_GUIDE.md](BLOOM_TAXONOMY_GUIDE.md),
  [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md),
  [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md), and related lifecycle
  docs. Historical changelog and archive files are unchanged. Docs-only.
- Synchronized shared style guides, tests, and repository support files from the starter template.
- Synchronized shared style guides, tests, and repository support files from the starter template.
- Synchronized shared style guides, tests, and repository support files from the starter template.

