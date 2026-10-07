# M28 supported import boundary

Status: source-boundary review only. M28 acceptance remains pending runtime proof and dependency evidence.

## Acceptance boundary

The approved M28 outcome in [docs/active_plans/active/question_spec_implementation_plan.md](../../active/question_spec_implementation_plan.md) is to prove authenticated import and Blueprint assembly through ordinary operations, with generated IDs and readback, and correct any bypassing loaders. The required checklist is Native JSON, PG/PGML, assets, metadata, ownership, exact Question references, reusable Pools, and Theme. Converter handoff remains deferred.

Record the source-to-PLE mapping and provenance at import; after launch, PLE operates independently of BiologyProblems.org and does not track or apply later source changes.

Source inspection can establish the route to ordinary operations, but it cannot establish connected authenticated behavior. SQL fixture identifiers and fixture inserts are not import workflow evidence. M28 is accepted only after a focused connected run exercises generated identities and persisted readback for the approved checklist, with all dependencies complete.

## Current canonical callers

- [crates/project-tools/src/pilot_content/publication.rs](../../../../crates/project-tools/src/pilot_content/publication.rs)`::publish_with_context` builds PostgreSQL Authoring Draft and source-binding stores, the ordinary Question Library store, the configured Question source object store, and the server Question ID issuer. `publish_plan` uses `matching_or_new_draft`, binds the source through `DraftQuestionSourceBindingStore`, then calls `NewQuestionLineagePublisher::publish` with the authenticated session, workspace, metadata/classification/authorship/license and source evidence. It returns the generated Published Question Revision Tuples. This supports the named-caller finding; it is a fixed Pilot installer path, not proof for every supported import format or asset case.
- [crates/project-tools/src/curriculum_content/publication.rs](../../../../crates/project-tools/src/curriculum_content/publication.rs)`::publish_with_context` validates the authored manifest, resolves existing Question provenance through the ordinary Question Library store, and delegates missing Question publication to `curriculum_content::parameterized_publication::publish_with_context`. It creates or loads the Blueprint through `PostgresBlueprintCourseStore`, checks loaded content and exact Question pins, and returns a receipt. `create_blueprint` loads the generated Blueprint ID back from the store and validates the content. The named finding is accurate, with the ordinary Question publisher implemented in the delegated module.
- [crates/project-tools/src/installation_data_blueprint.rs](../../../../crates/project-tools/src/installation_data_blueprint.rs)`::create_live_demo_blueprint` resolves Question Tuples from the publication mapping and creates the Blueprint through `PostgresBlueprintCourseStore::create_blueprint_course`. It reads the generated Blueprint back, validates its content and exact Question Tuple order, publishes when private, then reads it back again. It returns the store-generated Blueprint and Assessment IDs for dependent installation SQL. The input currently uses fixed Question entries and `Theme::default()`; it does not exercise Pool references or itself demonstrate Theme adoption into a Course.

Together these callers do use ordinary Question and Blueprint services rather than directly inserting the canonical objects through fixture SQL. They do not yet prove the entire M28 checklist. The installation Blueprint's explicit Theme default is a source fact; M27's separate adoption behavior still needs integrated evidence.

The content scope remains the approved pilot scope in [docs/FALL_2026_PILOT.md](../../../FALL_2026_PILOT.md): complete Genetics and Biochemistry first, retain Biotechnology, and defer other incomplete content. The checked-in Pilot Question plan is specifically a fixed Chapter 1 assessment inventory; the separate curriculum publisher handles a bundled Genetics manifest, while the Live Demo Blueprint is Biochemistry. The Chapter 1 plan has four PGML and four Native JSON items; the bundled Genetics manifest has 41 PGML items. These manifests do not declare a plain PG source, image asset, Pool, or Theme test input, so they do not alone supply the full M28 matrix. These source paths are evidence about current callers, not a reason to add import catalog rules or expand the requested content scope.

## Focused evidence route

Existing source-level test modules include [crates/project-tools/src/pilot_content/tests.rs](../../../../crates/project-tools/src/pilot_content/tests.rs), [crates/project-tools/src/curriculum_content/publication/tests.rs](../../../../crates/project-tools/src/curriculum_content/publication/tests.rs), and unit tests in [crates/project-tools/src/installation_data_blueprint.rs](../../../../crates/project-tools/src/installation_data_blueprint.rs). These can check mapping/input validation, but do not establish authenticated PostgreSQL/object-store execution by the named callers.

The concrete connected insertion point is [tests/e2e/e2e_installation_data.sh](../../../../tests/e2e/e2e_installation_data.sh), registered in [local_stack_control/acceptance_lanes.py](../../../../local_stack_control/acceptance_lanes.py) as the "ordinary installation-data provision and replay" lane. The narrow lane command is:

```sh
source ./source_me.sh && bash tests/e2e/e2e_installation_data.sh
```

The script owns its disposable Live Demo lifecycle: it starts [launchers/run_live_demo.sh](../../../../launchers/run_live_demo.sh) `--headless`, checks the pre-replay seed state with [tests/e2e/e2e_live_demo_course_seed.sh](../../../../tests/e2e/e2e_live_demo_course_seed.sh) `--state`, runs `python3 -m local_stack_control.disposable_stack_command replay-installation-data --manifest local_stack_state/live_demo_browser/workspace/disposable.manifest`, checks the state again, and stops the stack. The full existing runtime command is:

```sh
source source_me.sh && python3 local_stack.py acceptance
```

Do not run either command as part of this boundary review. The current seed-state checks use the Instructor persona against ordinary Course and Course Assessment list APIs. They require one matching Biochemistry Course and Assessment, validate canonical generated Course Instance and Assessment ID shapes, and verify the Assessment is a released `practice_question_assignment` with a current numeric edit number. This is a connected harness, not M28 proof: neither the replay command nor its current assertions read imported Question details or prove source binding, assets, Question metadata/ownership, generated Question mapping, exact Question Revision Tuples, reusable Pool references, or Theme adoption.

### Route-to-assertion map for the future lane

These existing authenticated routes identify useful readback assertions; their existence does not mean the current lane exercises them. Reuse the cookie/request helper patterns in [tests/e2e/e2e_live_demo_blueprint_course.sh](../../../../tests/e2e/e2e_live_demo_blueprint_course.sh) and [tests/e2e/e2e_live_demo_question_library.sh](../../../../tests/e2e/e2e_live_demo_question_library.sh) when extending the existing lane.

| Assertion | Existing route or connected read | Evidence and limits |
| --- | --- | --- |
| Published Question content, format, and metadata | Instructor/Sysadmin: `GET /api/questions/by-id/{question_id}/revisions/{revision_number}` ([crates/server/src/question_library.rs](../../../../crates/server/src/question_library.rs)) | Answer-free prompt/summary, backend, Question format, metadata/authorship, and image asset ID/checksum references. Does not return raw source bytes, source object checksum/media type, owner Account ID, or binary asset bytes. Authorship metadata does not prove ownership. |
| Delivered image asset | Instructor/Student: `GET /api/questions/{question_id}/revisions/{revision_number}/images/{question_image_asset_id}` ([crates/server/src/question_image_delivery.rs](../../../../crates/server/src/question_image_delivery.rs)) | Unavailable/unready assets are concealed as 404; ready assets redirect to an immutable rendition with ETag checksum. To assert binary bytes, follow the redirect and fetch/checksum the actual local rendition. The current lane has no image E2E. |
| Exact PG/PGML or Native JSON source binding and bytes | Connected application read using `PostgresQuestionLibraryStore` for the exact source object ID/checksum/media type, configured existing ObjectStore, and `objects::ResolvedQuestionSource::resolve(tuple, object_id, checksum)` | This Store/ObjectStore path can verify media type and bytes for the generated Question Revision Tuple. It is application readback, not direct SQL fixture insertion. No cited HTTP route exposes raw PG/PGML source. |
| Question ownership | Instructor: `GET /api/library-objects/search` ([crates/server/src/question_library.rs](../../../../crates/server/src/question_library.rs)) | Query `text` with the newly generated canonical PublishedQuestionId; the exact-question result includes `ownerAccountId`. Future assertion: using an authenticated Instructor, compare that value with the known importing Instructor account ID from test setup. The Question revision-detail endpoint itself omits owner. This is an existing readback seam; no connected test is claimed here. |
| Blueprint structure and references | Instructor: `GET /api/course-blueprints/{blueprint_course_id}/revisions/{revision_number}` | Reads Theme, ordered modules/assessment entries, exact fixed Question Revision Tuples, and Pool ID/Edit Number/selection settings. M07/M11 contract fields remain provisional until their handoffs. |
| Reusable Pool members | Instructor: `GET /api/course-blueprints/{id}/assessments/{assessment_id}/pools/{pool_id}/members` | Reads actual Pool members separately from the Blueprint's Pool reference and selection settings. |
| Adopted Course Theme | Active Course member: `GET /api/course-instances/{course_instance_id}/appearance` | Reads Theme on the adopted Course appearance; membership is required. The persisted Theme is one of the existing Theme values; Course color preferences and proposed Theme mappings remain documented in the Course specifications. This readback proves persisted adoption only; rendered visual proof remains pending. |

The API rows identify authenticated HTTP evidence that a future lane would obtain when it exercises those routes; they do not show that the routes have already run. The source row requires existing connected application Store/ObjectStore access. These assertions explain a future extension of the current lane and do not establish current M28 acceptance.

To make this the M28 route, extend the same disposable connected lane only after M07, M08, M11, M16, and M27 readiness is established. Exercise the authenticated ordinary import/publication and Blueprint assembly operations with a synthetic Instructor session and configured object storage; then read back through ordinary authenticated APIs/stores and assert generated IDs and persisted values for Native JSON, PG/PGML, assets, metadata, ownership, exact Question references, reusable Pools, and Theme. Include assertions that distinguish each source binding and each exact Question Tuple from fixture IDs. The Live Demo API readback currently uses the Instructor persona; a direct helper invocation alone does not establish HTTP route authorization. Correct any loader that bypasses these ordinary operations. Keep converter handoff deferred. This describes required future extension, not current behavior or acceptance.

Supporting connected selectors are partial evidence only, and all are ignored disposable-PostgreSQL tests:

- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test authoring_draft_source_postgres webwork_draft_creation_keeps_the_initial_source_binding_on_confirmation -- --ignored --exact` checks the initial Draft source binding, including its PG/PGML context; it does not invoke import publication or object readback.
- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test blueprint_course_postgres blueprint_course_postgres_lifecycle::revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe -- --ignored --exact` checks the Blueprint lifecycle selector; it does not combine imported Questions, assets, Pools, and Theme into an authenticated M28 workflow.
- The M08 lineage selector is recorded in [docs/active_plans/reports/question_spec_m08_blueprint_parents.md](../question_spec_m08_blueprint_parents.md): `cargo test -p learning-data-access --features postgres --test blueprint_course_postgres blueprint_course_postgres_lineage_fork::blueprint_forks_are_ordinary_lineages_with_immediate_parent_and_normal_revisions -- --ignored --exact`. It checks Blueprint lineage behavior only.

`source ./source_me.sh && cargo test -p project-tools` covers project-tools unit tests, not a connected target. None of these selectors establishes the full M28 route, and fixed-ID SQL fixtures are not importer proof.

## Dependencies and readiness

Approved dependencies are M07, M08, M11, M16, and M27.

- M07 is pending in the implementation ledger; `M07_DRAFT_SQL_READY` is absent.
- M08 has implementation and focused test source ready and independent specification/quality reviews passed, but its fresh-database and focused live test remain pending in [docs/active_plans/reports/question_spec_m08_blueprint_parents.md](../question_spec_m08_blueprint_parents.md).
- M11 is pending in the ledger; `M11_POOL_REFERENCE_READY` is absent.
- M16 is pending in the ledger. [M15_M16_BOUNDARY.md](M15_M16_BOUNDARY.md) records the Draft source/preview boundaries and M07 overlap, not completion of M16.
- M27 has implementation activity and focused source checks recorded in [docs/active_plans/reports/QUESTION_SPEC_M27_THEME.md](../QUESTION_SPEC_M27_THEME.md); connected adoption and integrated evidence remain pending, and its ledger row still lists M11 dependency/acceptance evidence as pending.

Therefore the boundary is useful for implementation preparation, but M28 acceptance work cannot be declared ready and M28 remains unaccepted. Recheck dependency markers and ledger rows when beginning runtime work.

## Evidence limits and handoff

This review verifies the three named files and their direct service calls. It does not establish connected database behavior, authenticated role enforcement under the acceptance runtime, object-storage asset publication/readback, all supported Native JSON and PG/PGML import paths, reusable Pool preservation, Theme inheritance, or complete metadata/ownership fidelity across those paths. The inspected source validates exact Question references in the Blueprint paths, but no connected M28 workflow result was found. Converter handoff is outside M28 scope.
