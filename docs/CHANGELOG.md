# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 17 entries are archived in [CHANGELOG-2026-09m.md](CHANGELOG-2026-09m.md).

## 2026-09-19

### Fixes and Maintenance

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
  `?revisionNumber=`. Assessment editor and PLE Question JSON draft
  concurrency are `etag`. Attempt response edits use
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
## 2026-09-18

### Additions and New Features

- Pre-production identity cutover: public-ID JSON fields are `id`, not a
  parallel `reference` property. Dual `id`+`reference` is gone from
  Course, Assessment, Attempt, and Blueprint views. Gate:
  `cargo tsgen` and `./launchers/run_fast_checks.sh`.

- Live Demo and bundled publisher Account IDs are public `UXXXXXXXZ`
  values (`U0000035E` and kin). The public-ID mint trigger keeps a
  supplied canonical ID and only replaces the mint placeholders
  `U00000009`, `CI0000000Y`, `A0000000A`, and `BP0000000C`. Gate:
  `tests/test_live_demo_target.py`.

- Student Work API grants start as `ple_private_owner` so REVOKE/GRANT on
  `ple_private` SECURITY DEFINER functions apply. Replica durability counts
  `assessment_attempt_saved_response` instead of dropped `question_response`
  tables. The replica count validator accepts one integer per query, not a
  fixed five-field row. The stored Question fixture set uses public Course
  Instance and Assessment IDs for `cargo tools fixtures`. Live Demo composition tests
  use public Account IDs. Markdown schema links follow the layered
  `schemas/base_schema/` layout; gitignored `catalog_snapshot.json` is not
  a browsable link. Course Banner HTTP lives in
  `crates/server/src/course_appearance/banner.rs`. The Assessment Question
  Editor view lives in
  `assessment_workspace_questions_view.tsx`. Clock type and Library
  statistic increment rules moved from Human Guidance into
  [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md). Gate:
  `tests/test_disposable_stack_replica_adapter.py` and
  `./launchers/run_fast_checks.sh`.

- Close-out gates: disposable `postgres:17` install plus Live Demo teaching
  graph (publisher psql variables supplied; eight Pilot Question pins and a
  public Blueprint so `live_demo.sql` loads). Connected Student Work oracles
  `attempt_expiry_connected_oracle.sql` and `unrelease_connected_oracle.sql`
  pass. M4 Unrelease purge `EXPLAIN` uses
  `assessment_attempt_assessment_id_idx`. Domain CHECK helpers
  `is_canonical_prefixed_public_id` are `EXECUTE` to `PUBLIC`. Gate:
  `tests/_temp/run_m4_install_and_explain.py` and
  `./schema_style/check_schema_style.py` twice.

- WP-3.8 Library projection JSON: `QuestionStatistics` keeps `{state:"unavailable"}`
  and adds Instructor `available` with snake_case counts and rates
  (`issued_count`, `blank_count`, `answered_count`, outcome counts,
  `credit_sum`, `credit_sum_sq`, omitted rates when the denominator is 0).
  Bulk search uses the all-Revision rollup; Question detail adds `revisions`.
  Pool detail `evidence` uses current members' all-Revision rollup (mean
  credit weighted by `answered_count`) plus `pool_issued_count` from
  `question_pool_statistics`. SQL:
  `ple_api.read_question_library_usage_statistics`,
  `read_question_library_revision_usage_statistics`,
  `read_question_pool_library_usage_statistics`. Students never receive
  these aggregates; Sysadmin Library reads stay unavailable. Gate:
  `cargo test -p question_model --offline --lib question_library_statistics`
  and `node --test tests/test_question_statistics_decoder.mjs`.

- M4 WP-4.1, WP-4.2: Recorded `EXPLAIN (ANALYZE, BUFFERS)` for Unrelease
  purge, FERPA purge, Student landing, gradebook, Watch feed, and Pool
  difficulty (member join to `question_revision_statistics`) in
  `{SCRATCH}/explain_baseline.log`. Live Demo teaching graph was not
  loaded (publisher psql variables absent), so plans ran on an empty
  Student Work catalog; execution times were sub-millisecond. FERPA
  purge already uses `assessment_attempt_student_assessment_lookup_idx`;
  gradebook uses the course-leading unique key. Unrelease
  `DELETE ... WHERE assessment_id = ...` seq-scans `assessment_attempt`
  because the PK leads with `course_instance_id`; added
  `assessment_attempt_assessment_id_idx`. No cached Pool difficulty
  column (empty-catalog join does not miss a latency budget). Remaining
  advisory `rule_14` unindexed FKs deferred. WP-4.3 generic `to_jsonb`
  immutability guards deferred: column-list `IS DISTINCT FROM OLD`
  triggers remain on Student Work and Course lifecycle. Gate:
  `tests/_temp/run_m4_install_and_explain.py` (install.sql, M2 probes
  pass, M3 revision counters pass).

- M3 WP-3.1, WP-3.2, WP-3.8 (WS-derived): Dropped `question_attempt_state`,
  `finalization_kind`, `question_response`, `question_response_grading`,
  submission `receipt`, and `ple_private.imathas_question_backend_session`.
  Saved responses finalize in place (`finalized_at`,
  `assessment_submission_id`). `grading_result` FKs
  `(course_instance_id, question_attempt_id)` only; receipts FK that PK.
  `ple_private.delivery_toolchain` holds the seven toolchain values;
  `question_attempt.delivery_toolchain_id` is the NOT NULL FK. Library statistics
  store `issued_count`, `blank_count`, `answered_count`, outcome buckets,
  and credit sums; dropped choice statistics; added pool issued and member
  selected counts. JSON field names stay the same except Library stats
  (snake_case). Gate: `tests/_temp/m3_library_stats_probe.sql` and
  `source source_me.sh && python3 devel/generate_schema_tables_doc.py &&
  python3 schema_style/check_schema_style.py`. GRANT
  `question_source_binding_fields_are_valid` now matches the three-argument
  CREATE so disposable `postgres:17` `install.sql` succeeds.

- M3 WP-3.4 to WP-3.7 (WS-fanout): Watch inbox JOINs
  `library_watch_event_recipient` to `library_watch_event` (same JSON
  columns; no `read_at`). Dropped `ple_private.library_watch_notification`,
  `ple_private.imathas_render_cache_entry`, iMathAS source-binding columns
  and enum labels (`question_backend`, `question_format`, `import_format`),
  delivery SQL wrappers, and `ple_audit.object_delivery_access_event` plus
  unused `ple_data.access_decision`. Postgres iMathAS session store returns
  Unavailable. Shared work lease is `lease_token` plus `lease_expires_at`
  with `ple_private.work_lease_pair_is_valid` on `job`,
  `course_retention_notification`, `course_banner_work`, and
  `profile_image_work` (banner/profile keep `lease_state` payload). Claim
  stays per-worker. Workers still needed: public-asset job, FERPA notifier,
  banner work, profile image work. Retention worker sweeps expired/revoked
  `authenticated_session`, expired/consumed `email_authentication_challenge`
  and `passkey_ceremony`, and old `authentication_rate_limit` windows after
  a hardcoded 7-day grace (`ple_api.sweep_expired_authentication_growth`).
  Partial sweep indexes sit on those predicates. `question_backend` is
  `ple` and `webwork` only. Gate:
  `source source_me.sh && python3 devel/generate_schema_tables_doc.py &&
  python3 schema_style/check_schema_style.py`.

- M2 WP-2.5 and WP-2.3: Question Pools are current state. Dropped
  `question_pool_revision`, `question_pool_revision_member`, and
  `question_pool_revision_bloom`. Members live in `question_pool_member`
  keyed by `(question_pool_id, member_position)`. Attestation and Edit
  Number live on `question_pool`. `save_question_pool_members` CAS-es the
  Edit Number, no-ops an identical ordered list, and re-attests on change.
  Forks copy current members once. Assessment pool entries and snapshots
  pin `question_pool_id` only. `question_pool_selection` stores
  `question_pool_id` plus `question_pool_edit_number`; selected items keep
  exact Question pins. Watch emits `members_changed`. Student Work tables
  in `assessment_attempt.sql` (and ple_audit correction targets) carry
  `course_instance_id NOT NULL` leading every PRIMARY KEY and UNIQUE, bound
  in parent FKs. Accommodation stays teaching config with a uuid PK and
  `course_instance_id` in Attempt FK tuples. JSON field names are unchanged.
  Gate: `source source_me.sh && python3 devel/generate_schema_tables_doc.py &&
  python3 schema_style/check_schema_style.py` twice exits 0 (advisory
  `rule_14` 175). `tests/_temp/m2_snapshot_probes.sql` on disposable
  `postgres:17` shows equal policies share one snapshot, distinct policies
  do not, and an Attempt keeps the original snapshot after the Assessment
  policy changes (`student_work_oracles.log`).

- M2 WP-2.2: `assessment_entry` is a parent of common columns; kind-specific
  facts live in `assessment_entry_question` and `assessment_entry_pool`.
  Frozen issue facts live in `ple_private.assessment_entry_snapshot`
  (content-addressed SHA-256 PK). `issued_question` stores
  `assessment_entry_snapshot_id` and drops `point_value`, `scoring_rule`,
  `question_attempt_limit`, `question_attempt_time_limit_seconds`, and
  `question_attempt_grace_seconds`. Save and issue call
  `ple_private.ensure_assessment_entry_snapshot`; the scorer reads points
  through the snapshot. JSON field names are unchanged. Gate:
  `source source_me.sh && python3 devel/generate_schema_tables_doc.py &&
  python3 schema_style/check_schema_style.py` exits 0 (advisory `rule_11`
  15, `rule_14` 172).

- Human Guidance public-ID names: Rust/TypeScript types are `AccountId`,
  `CourseInstanceId`, `AssessmentId`, and `BlueprintCourseId` (no parallel
  `*Reference` or `CourseId` alias). `docs/TERMINOLOGY_CONTRACT.md` matches
  HUMAN_GUIDANCE: public IDs are the primary keys; Question Pools are current
  state with an Edit Number; Student Work pins Pool ID plus Pool Edit Number.
  Gate: `cargo test -p question_model --offline --lib` (146 passed);
  `cargo check -p learning-data-access -p server_core -p project-tools
  --offline`; `cargo tsgen` wrote 395 types.

- M2 WP-2.1: frozen Assessment policy lives in
  `ple_data.assessment_policy_snapshot` (content-addressed SHA-256 PK).
  `assessment`, `assessment_template`, and `assessment_attempt` store
  `assessment_policy_snapshot_id` instead of copying title, instructions,
  schedule, limits, rules, and feedback. `ple_private.ensure_assessment_policy_snapshot`
  inserts-or-reuses the row; Attempt start copies the Assessment's current
  snapshot id. Quiz/Exam attempt-limit = 1 is enforced in the helper.
  JSON field names are unchanged. Style checker allows `sha256_digest` PKs
  on `role: snapshot` tables. Gate:
  `source source_me.sh && ./schema_style/check_schema_style.py` twice exits 0
  (advisory `rule_11` 15, `rule_14` 170).



- M1 WP-1.4: Account, Course Instance, Blueprint Course, Assessment, and
  Question Pool primary keys are the public ID (`ple_data.account_id`,
  `course_instance_id`, `blueprint_course_id`, `assessment_id`,
  `question_family_id` domains in `10_types.sql`). Foreign keys target those
  columns. GRANT identities were rewritten to match CREATE FUNCTION argument
  types. Disposable `postgres:17` `install.sql` completes (146 tables, 281 FKs,
  600 routines including public-ID domain helpers; CHECK/index/policy/trigger
  counts changed with enums and clocks). Gate:
  `source source_me.sh && ./devel/generate_schema_tables_doc.py && ./schema_style/check_schema_style.py`
  exits 0 with advisory `rule_11` and `rule_14`. Catalog snapshot type display
  keeps schema-qualified enums (`ple_data.product_role`). Live Demo seed and
  Rust `AccountId`/`CourseInstanceId`/`AssessmentId` mappings still follow.

- M1 WP-1.6: the concurrency token is the Edit Number only. SQL columns are
  `course_edit_number`, `blueprint_edit_number`, and
  `question_pool_edit_number`. JSON uses those names (`courseEditNumber` /
  `blueprint_edit_number` / `blueprintEditNumber` depending on the contract's
  serde case). HTTP `ETag`/`If-Match` is the decimal integer as a strong
  validator (`"42"`), not a UUID. `cargo tsgen` wrote 395 types including
  `BlueprintEditNumber`. Gate: `cargo check -p learning-data-access -p
  server_core --offline` after the Course JSON field rename.

- M1 Live Demo: `installation_data_activity` issues temporary sessions by
  looking up `ple_private.account_authentication_email.normalized_email` as
  `ple_migrator`/`ple_private_owner` through `PLE_MIGRATION_DATABASE_URL`.
  Fictional emails stay the stable seed keys; minted Account public IDs are
  not compiled in. Gate: `cargo check -p project-tools --offline`.

- Account, Course Instance, and Assessment identities in Rust are the
  canonical public ID string (`AccountId` / `CourseInstanceId` /
  `AssessmentId`). `AccountId`, `CourseInstanceId`, and `AssessmentId` are
  aliases of those types so SQL, Rust, and TypeScript share one value. Postgres
  binds and decodes that text. Live Demo env vars parse public IDs, not UUIDs.

- WP-1.4 leftover in `50_functions`: bodies that still read
  `public_reference` / `reference_number` on Account, Course Instance,
  Blueprint Course, Assessment, and Question Pool now use the text `*_id`
  PK. Locals and parameters that held those IDs as `uuid` or `bigint`
  (including `v_actor` from `current_session_account_id()`) are `text`.
  API `RETURNS TABLE (public_reference text, ...)` aliases remain.
  `assessment_attempt.reference_number` and
  `authoring_workspace.reference_number` are unchanged. 60_policies needed
  no further public-ID column rewrites.

- After dropping `ple_private.assessment_attempt.reference_number` and
  `ple_private.authoring_workspace.reference_number`, `50_functions` readers
  and writers look up Assessment Attempts by `assessment_attempt_id uuid`.
  Parameters, `RETURNS TABLE` columns, GRANT EXECUTE identities, and JSON
  `assessmentAttempt` values use that PK text rather than `'R-' ||
  reference_number`. Archived recovery keyset pagination uses
  `p_after_assessment_attempt_id uuid`.

- Removed `ple_data.course_retention_policy`. FERPA notice, archive, recovery,
  and deletion intervals are installation GUCs
  (`ple.retention_inactive_warning_lead_time`,
  `ple.retention_archive_notice_lead_time`,
  `ple.retention_archive_after_retention_start`,
  `ple.retention_delete_after_archive`) read by `ple_data.retention_schedule()`,
  with the previous 14/70/100/265-day defaults. Callers no longer select a
  named policy row. Table grants and RLS for that object are gone.

- Identity PKs that were bigint IDENTITY, boolean, or a Blueprint
  reference-number PK are now `uuid` (or `text` for the singleton retention
  policy). The SQLx ledger in `ple_migration` stays outside `rule_08`.
  Nullable columns have `COMMENT ON COLUMN` stating that NULL means the
  optional fact is absent. Table-level `CHECK` clauses after `--` comments
  are no longer parsed as a column named CHECK. Gate:
  `source source_me.sh && ./devel/generate_schema_tables_doc.py && ./schema_style/check_schema_style.py`
  exits 0 with only advisory `rule_11` and `rule_14`.

- `schema_style/check_schema_style.py` loads `schemas/catalog_snapshot.json`
  when that file exists, so
  `source source_me.sh && ./devel/generate_schema_tables_doc.py && ./schema_style/check_schema_style.py`
  applies snapshot Tier 3 without `-j`. An explicit `-j` path still errors if
  missing.

- M1 types, clocks, and key names: closed vocabularies are PostgreSQL enums in
  `10_types.sql` (snake_case labels); `text CHECK (IN ...)` columns now use
  those types; every table has a creation clock and current-state/aggregate
  tables have `updated_at`/`updated_on`; single-column FKs name their parent
  table (`course_instance_id`, `published_question_id`, `object_record_id`)
  per DATABASE_STYLE.md. JSON emitted by SQL stays snake_case
  (NAMING_CONVENTIONS.md). Assigned-Instructor Course columns are gone;
  Instructor membership is the Course teaching team. `course_theme` is a
  vocabulary table. Disposable `postgres:17` `install.sql` completes.
  Style gate:
  `source source_me.sh && ./devel/generate_schema_tables_doc.py && ./schema_style/check_schema_style.py -j schemas/catalog_snapshot.json`.
  Public-ID primary keys (WP-1.4) and Rust mappings remain.

- M0 install census (disposable `postgres:17`, bootstrap through
  `migration_principal_bootstrap_sql`, then `psql --single-transaction -f
  install.sql` as `ple_migrator`): 146 tables / 281 FKs / 497 CHECKs / 282
  indexes / 290 policies / 599 routines / 158 triggers, matching the
  2026-09-17 audit. `content_vocabulary.sql` loaded. `live_demo.sql` still
  requires publisher `psql` variables (`pilot_question_publications` and
  Blueprint references) from `cargo tools installation-data apply`. CHECK
  helpers live in `15_table_check_functions.sql`; default REFERENCES/EXECUTE
  grants let later layers create cross-schema FKs and triggers. Gate log:
  `{SCRATCH}/install_seed.log`.

- M0 WP-0.1 to WP-0.3: moved `schemas/base_schema/` into the DATABASE_STYLE.md
  layered layout with `git mv` for every mixed module that survived as one
  destination (`00_roles.sql`, `10_types.sql`, `20_tables/<aggregate>.sql`,
  `30_constraints.sql`, `40_indexes.sql`, `50_functions/<domain>.sql`,
  `60_policies/` plus a short include manifest, `70_grants/` plus a short
  include manifest). Every table has a `COMMENT ON TABLE` that begins with
  `role:`. Policies and privileges are split by domain so no layer file
  crosses the 999-line source budget. `rule_layout` now matches GRANT/REVOKE
  as statement keywords, not substrings of `revoked_at`. Gate:
  `source source_me.sh && ./schema_style/check_schema_style.py` exits 0 with
  zero blocking findings (M1/M2 rules remain advisory). `provided_avatar_catalog.sql`
  was inlined into `20_tables/profile_media.sql` and removed as a path.

- Added `devel/generate_schema_tables_doc.py` (WP-0.4): reads the schema catalog
  through `schema_style.schema_catalog_lib` from `schemas/base_schema/` or
  `-d`/`--database`, and writes [SCHEMA_TABLES.md](SCHEMA_TABLES.md) plus
  `catalog_snapshot.json`. One
  Markdown section per `20_tables/*.sql` file when that directory exists,
  otherwise one section per source file that contains `CREATE TABLE`. Each table
  lists qualified name, role tag, columns, constraints, foreign keys, indexes,
  and catalog comments. Command:
  `source source_me.sh && python3 devel/generate_schema_tables_doc.py`. Documented
  in [USAGE.md](USAGE.md).

- Added the SQL base schema restructure plan
  ([sql_schema_restructure_plan.md](archive/sql_schema_restructure_plan.md)) in the
  `blueprint-plan-drafter` multi-workstream form: five milestones (M0 layered layout and catalog
  comments, M1 types and identity, M2 snapshots, M3 derived data and fan-out, M4 measured indexes
  and generic immutability guards), eight workstreams with parallel-readiness stated per
  milestone, 28 one-owner work packages numbered in dependency order, six resolved decisions so execution needs no further
  human input (feedback rules frozen in the Attempt snapshot, the public ID as primary key
  wherever one exists, Edit Numbers replace `<retired-term-replace-me>`, responses finalized in place,
  iMathAS removed, course themes stay a reference table), gates, risk register, and patch plan. The audit now holds
  evidence only and points to the plan.

- Renumbered the `DATABASE_STYLE.md` checklist to a straight 1-18 (the inserted `6b` and `9b`
  became 7 and 11) and the restructure plan's M2 work packages to dependency order (partition
  keys are WP-2.3, the probe WP-2.4); every cross-reference in the style doc, both plans, the
  audit, and this changelog uses the new numbers.
- Added `devel/markdown_section_sizes.py`: lists a Markdown file's headings with line numbers
  sorted by the count of `- ` bullets directly under each (`-n` top N, `-m` minimum), so
  oversized Human Guidance sections are easy to find. First run on `HUMAN_GUIDANCE.md`: 116
  headings, 1,118 bullets; largest are Content classification (36), Search Question Library
  interface (29), and Agent working principles (23). Documented in `USAGE.md`.
- Split three oversized Human Guidance sections with every bullet preserved: "Human-facing
  reference IDs" (48) into the parent plus alphabet and canonical form, checksum, formats by
  object, and database keys and clocks; "Content classification" (36) into the hierarchy plus
  vocabulary management, selection and discovery, and tags and names; "Search Question Library
  interface" (29) into interface, filters, and syntax siblings. Human Guidance now states the
  under-25-bullets rule per section with the tool that checks it; no section exceeds it. The
  pre-production bullets were reduced to one "no legacy" and one "fix the foundation" bullet.
- Added a separate step-list plan for the schema style checker
  ([schema_style_checker_plan.md](active_plans/active/schema_style_checker_plan.md)):
  `devel/schema_catalog_lib.py` (one parser and model shared with the doc generator) and
  `devel/check_schema_style.py` (one `rule_<id>` function per mechanical checklist item, findings
  with locations, summary per rule, exit code, `--snapshot` / `--database` / `--report`). Tier 1
  rules run on the current source today and must reproduce the audit's counts (72 / ~110 / 17 /
  43 / 166); Tier 2 activates with role tags, Tier 3 with the catalog. The restructure plan's
  WP-0.5 now references this plan instead of restating it.
- Shipped the schema style checker as repo-root package `schema_style/` (`check_schema_style.py`
  orchestrator, `schema_style_rules.py`, `schema_catalog_lib.py` plus parse/scan/database
  modules). `devel/` is a support location, not an import package. Command:
  `source source_me.sh && python3 schema_style/check_schema_style.py`. Each finding line is
  `rule_<id>`, location, message, and source `file:line`. `-q`/`--quiet` prints per-rule
  counts only. CLI flags are `-s`/`--source-dir`, `-j`/`--snapshot`, `-d`/`--database`,
  `-r`/`--report`, and `-q`/`--quiet`; the findings file path is hardcoded to
  `output/schema_style_findings.txt`. First source run exits 1 with 612 findings in 7 rules.
  Versus the 2026-09-17 audit (72 / ~110 / 17 / 43 / 166): `rule_7_key_names` 74
  single-column FK suffix misses (audit 72 of 180; parser sees 177); `rule_4_types` 104
  (audit ~110); `rule_2_constant_columns` 17; `rule_16_clock_present` 44 (audit 43; extra
  `ple_private.object_storage_check`); `rule_14_unindexed_fk` 165 advisory (audit 166);
  `rule_5_duplicate_literal_sets` 59; `rule_layout` 149 advisory until `20_tables/` exists.
  A CREATE TABLE parse/count mismatch is blocking (`rule_table_count`). Tier 2 skipped
  (no role tags); Tier 3 skipped (source-only). `--snapshot` on a missing path raises
  FileNotFoundError.

### Behavior or Interface Changes

- WP-0.5: `schema_style/check_schema_style.py` keeps every rule implementation and
  treats M1 rules as advisory until M1 (`rule_7_key_names`, `rule_4_types`,
  `rule_2_constant_columns`, `rule_5_duplicate_literal_sets`,
  `rule_16_clock_present`, `rule_16_updated_clock`, `rule_16_clock_type`), M2
  `rule_11_student_work_keys` as advisory until M2, and `rule_14_unindexed_fk` as
  advisory. `rule_layout` stays advisory until `20_tables/` exists, then blocks.
  `rule_17_role_tag` blocks once any table carries a role tag. `table count`
  stays blocking. Gate:
  `source source_me.sh && ./schema_style/check_schema_style.py`.
- `schema_style/check_schema_style.py` default stdout is per-rule counts only. `-v`/`--verbose`
  prints one `rule_<id>` finding line per violation. The full list is always written to
  `output/schema_style_findings.txt`. Flags are `-s`/`--source-dir`, `-j`/`--snapshot`,
  `-d`/`--database`, `-r`/`--report`, and `-v`/`--verbose`.
- Schema style default summaries are `count`, tab, `rule_##_title` with two-digit
  numbers so they align and sort (` 17\trule_02_constant_columns` before
  `166\trule_14_unindexed_fk`). Verbose finding lines use the same padded ids.

- Question Pools are now current state with an Edit Number, not a Revision family. Student Work
  already pins the exact Published Question Revision served from a Pool, so a Pool Revision added
  nothing that grading, history, or Unrelease needs; Assessments own forked Pools as current
  state; Blueprint Revisions embed their Pool member lists; statistics and Bloom key by Pool.
  For every Pool-served Question, Student Work pins four values: Published Question ID, its
  Revision Number, Question Pool ID, and the Pool's Edit Number at selection (evidence, not an
  FK). Human Guidance (revision specifications, Question Pool specifications, usage statistics,
  Bloom metadata, Student Work), the FERPA policy, `DATABASE_STYLE.md` role and identity tables,
  and plan decision 7 with WP-2.5 (drop `question_pool_revision`, one `save_question_pool_members`
  with CAS and canonical no-op) record it. Patches renumbered 1-10+.
- Human Guidance "Human-facing reference IDs" now states one identity rule instead of "may":
  an object with a public ID uses that public ID as its primary key and sole foreign-key target;
  objects without one use a native UUID or a composite natural key; no table carries an
  auto-increment integer key; every table has one creation clock (`timestamptz` for enforced,
  ordered, or audited rows, `date` for authored content); table shape follows
  `DATABASE_STYLE.md`. The pre-production rule now also says to use that state to improve
  foundational schemas, contracts, abstractions, and ownership boundaries. `DATABASE_STYLE.md`
  "Identity" and checklist item 7, plan decision 2 and WP-1.4, and audit finding 2.1 record the
  same decision: Question Pools, Course Instances, Assessments, Accounts, and Blueprint Courses
  re-key to their public IDs and every `reference_number` column is dropped.

- `DATABASE_STYLE.md` gained "Partition readiness": at 50 Courses x 100 Students x 20
  Assessments x 25 Questions x 5 Attempts, Student Work tables grow ~12.5M rows per semester,
  held to a 25-40M steady state by the 365-day FERPA purge, so partitioning is unlikely; every
  Student Work table nonetheless carries `course_instance_id`, binds it in its parent FK, and
  leads every PK and UNIQUE with it, because the per-Course purge benefits now and a future
  `PARTITION BY LIST (course_instance_id)` then attaches without a re-key. Nothing is partitioned before a
  measured trigger (100M rows, per-Course purge in seconds, or autovacuum lag). Checklist item 11
  and plan WP-2.3 carry it; row estimates use five Attempts per Student.

### Fixes and Maintenance

- Connected PostgreSQL fixture SQL in
  `crates/learning-data-access/tests` now mints public IDs (`U`/`CI`/`A`/`BP`
  via `RETURNING`) and uses the restructured columns:
  `account_id` text, `course_instance_id`, `blueprint_course_id`,
  `assessment_policy_snapshot_id`, `assessment_entry_question`,
  `published_question_id`, `content_discipline_id`,
  `course_membership_id`. Assessment Access keeps
  `student_record_id` uuid `0xeb02` and `assessment_entry_id` uuid
  `0xed02`. Gate: `cargo test -p learning-data-access --features postgres
  --no-run --offline --locked`.

- Public IDs (`AccountId`, `CourseInstanceId`, `AssessmentId`) wrap `String`
  and are no longer `Copy`. Tests clone reused values at Course Banner saga,
  Course Appearance, and iMathAS Question Backend call sites. Gate:
  `source source_me.sh && cargo check --workspace --all-targets --locked --offline`.

- Browser TypeScript uses Blueprint/Course Edit Number JSON fields
  (`blueprint_edit_number`, `blueprintEditNumber`, `courseEditNumber`) and
  quoted `"42"` ETags instead of retired `metadataEtag` UUID validators.

- Assessment Attempt identity in Rust, HTTP paths, and JSON is
  `AssessmentAttemptId` (UUID text). Store traits, Postgres binds, path
  parsing, and TypeScript route/JSON parsers no longer use the `R-`
  numeric `AssessmentAttemptReference`. JSON field names stay
  `assessmentAttempt`. Gate:
  `source source_me.sh && cargo test -p question_model student_work::identifiers --offline`
  and `cargo check -p learning-data-access -p server_core --offline`.

- Rewrote `schemas/installation_data/live_demo.sql` and `live_demo_oracle.sql`
  onto public-ID primary keys. Accounts resolve by seed email, the Course by
  short_name `BCHM 301`, the Assessment by title `Chapter 1 Pilot Practice`, and
  `ple.installation_live_demo_blueprint_public_reference` is the Blueprint
  Course ID. Course inserts set `created_at`, `active_until_at` (UTC plus six
  months), and `retention_starts_at`. Instructor teaching-team membership
  replaces assigned-instructor columns. Internal membership, invitation,
  student-record, workspace, and event keys stay uuid. Domain placeholders
  `CI0000000Y` and `A0000000A` satisfy the public-ID CHECK before
  `assign_human_reference` mints the row.

- Restored `schema_style` M1 rules as blocking (`rule_7_key_names`, `rule_4_types`,
  `rule_2_constant_columns`, `rule_5_duplicate_literal_sets`, `rule_16_clock_present`).
  Only `rule_14_unindexed_fk` and `rule_layout` (until `20_tables/` exists) are advisory.
  Default CLI prints `rule_<id>` finding lines and exits 1. Current source: 529 findings
  in 8 rules, `rule_7_key_names` 75 (audit 72 of 180).

- Settled the Question usage statistic in one dedicated Human Guidance section ("Question"
  Library object usage statistics"), mirrored in `FERPA_DATA_POLICY.md` and `DATABASE_STYLE.md`: per Published
  Question Revision, counters for issued, blank, answered, correct, partial, and incorrect plus
  credit sum and sum of squares (mean and standard deviation for bulk sorting); every Attempt
  counts, including practice; blank is tracked separately from incorrect and never reaches a
  backend; the row holds no Course, Student, Account, Attempt, timestamp, choice, or response
  linkage; the per-observation receipt is purged Student Work; Pool statistics are the sum of
  member Revisions at read time, while each Pool Revision keeps its own issued count and
  per-member selected counts; storage is per Revision, every bulk view displays the
  all-Revision rollup, and the Question detail page alone adds the per-Revision breakdown; the Library shows each rate beside its observation count
  with no exposure threshold. Answer-choice counts and "privacy threshold" wording were removed from
  HG and the FERPA policy. Plan WP-3.8 reshapes `question_revision_statistics` accordingly and
  drops the two choice tables.
- Rewrote `DATABASE_STYLE.md` in positive phrasing throughout: every rule states the action to
  take ("declare every column NOT NULL", "enforce immutability with privileges", "reach a
  parent's value through the foreign key") and lists of things to avoid were removed or folded
  into a "Replaces" column in the type table, following the prompt-positively principle in
  `REPO_STYLE.md`. Checklist questions now read as positive checks ("Is immutability enforced by
  privilege?"). The enum declaration file is `10_types.sql` throughout. No rule changed meaning.
- The clock rule now assigns the type by table role: full-precision `timestamptz` where the
  server enforces, orders, or audits (Student Work, sessions, leases, events, Courses,
  Accounts) and `date` for authored content (Published Questions, Pools, Blueprints, their
  Revisions, Draft Questions, usage statistics), because every `timestamptz` precision stores
  the same 8 bytes and compares at the same speed while `date` is 4 bytes and reads as the day.
  Rows mutated in place carry a second bookkeeping clock (`updated_at`/`updated_on`) of the same
  type; every further clock is a domain fact named for its event and typed by its meaning. The
  statistics aggregate carries `created_on` and `updated_on` (day of last increment), since day
  granularity on a global counter identifies no one. Integer epochs and
  reduced-precision `timestamptz(n)` are outside the rule. Recorded in
  `DATABASE_STYLE.md`, `HUMAN_GUIDANCE.md`, the FERPA policy (statistics carry the publication
  date), and plan WP-1.7.
- `DATABASE_STYLE.md` records the aggregate exception to the clock rule: anonymous statistics
  tables carry `created_on` and `updated_on` as `date`, because an increment `timestamptz` on a
  small-count aggregate is a re-identification path when joined to a roster while a calendar
  date on a global counter is not; FERPA attaches to identifiability, not timestamps. Plan
  WP-1.7 converts `question_revision_statistics.updated_at`.
- Style enforcement is designed as a Python maintainer tool rather than an external linter
  (SQLFluff and Squawk cover formatting and migration safety, schemalint and pgTAP need Node or a
  live database): the M0 generator also emits `schemas/catalog_snapshot.json`, every table
  comment begins with a role tag, and the maintainer tool `devel/check_schema_style.py`
  checks the layout and the mechanical checklist items (key names, types, clocks, Student
  Work keys, comments) against the snapshot or a live database in one command, with
  unindexed purge-path FKs under `--report`; a pytest wrapper is optional and later. schemalint (seven built-ins: casing,
  singular names, text, timestamptz, jsonb, identity, primary key) runs in parallel against the
  disposable Podman database whenever the snapshot is regenerated, as independent confirmation
  rather than a gate; the schema already satisfies all seven. Recorded in `DATABASE_STYLE.md`
  "Organization of the SQL source" and plan WP-0.4 / WP-0.5.
- The schema audit gained finding 2.11: 72 of 180 single-column FK columns are named so the
  parent table is invisible (`course_id` -> `course_instance`, `object_id` -> `object_record`,
  `thread_id`, and so on). `DATABASE_STYLE.md` "Naming" now requires PK `<table>_id` and FK
  `[<role>_]<parent_table>_id` with the full table name, checklist item 7 checks it, and plan
  WP-1.8 renames the columns while API field names stay mapped at the boundary.
- The schema audit gained finding 2.10 on growth outside the FERPA purge: grading and statistics
  receipts cascade with the Work (fine); `authenticated_session` and the other authentication
  tables are revoked but never deleted (unbounded, identifying); `ple_audit.object_delivery_access_event`
  has no writer anywhere and would log every asset load if wired (placeholder, remove); Published
  Questions are authored-scale and stay a global Library, with discovery search, not partitioning,
  as the Question-side concern. Plan WP-3.7 adds the expiry sweeps and drops the placeholder.
- The schema audit gained finding 2.9: 43 of 146 tables have no timestamp column, including the
  current-state `assessment_template` and the Student Work row `issued_question`; plan work
  package WP-1.7 adds the creation instant required by `DATABASE_STYLE.md`.
- Synchronized shared style guides, tests, and repository support files from the starter template.

### Developer Tests and Notes

- M3 WP-3.3: `tests/e2e/attempt_expiry_connected_oracle.sql` and
  `tests/e2e/unrelease_connected_oracle.sql` follow the current Student Work
  schema (minted public IDs, policy snapshot, `published_question_id`,
  `delivery_toolchain_id`, in-place saved-response finalization, observation
  capture). Same intent as before: stale snapshot fence, 0.67 credit replay
  1.34 then 2.01/3, attempt limit, landing, expired unanswered 0/3, Unrelease
  reject/accept/repeat, and anonymous stats `issued_count=2`. Not rerun on
  PostgreSQL here. Gate: review of the two oracles against `20_tables/` and
  `50_functions/`.
- WP-0.4/WP-0.5: `source source_me.sh && python3 devel/generate_schema_tables_doc.py`
  exits 0 and writes [SCHEMA_TABLES.md](SCHEMA_TABLES.md) (24 `20_tables/` sections,
  146 tables) and `catalog_snapshot.json`
  (146 tables, 282 indexes, 3 enums, all 146 tables tagged).
  `python3 -m pyflakes devel/generate_schema_tables_doc.py schema_style/*.py` exits 0.
  `source source_me.sh && ./schema_style/check_schema_style.py` exits 0 with
  `529 findings in 8 rules`, all advisory: `rule_11_student_work_keys 15`,
  `rule_14_unindexed_fk 166`, `rule_16_clock_present 44`, `rule_16_updated_clock 49`,
  `rule_2_constant_columns 17`, `rule_4_types 104`,
  `rule_5_duplicate_literal_sets 59`, `rule_7_key_names 75`. `rule_layout` is
  blocking and clean. `--snapshot` on a missing path raises `FileNotFoundError`.
