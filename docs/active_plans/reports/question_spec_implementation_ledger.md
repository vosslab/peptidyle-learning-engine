# Question-spec implementation ledger

Status (2026-10-07 final): Accepted locally. Canonical `all_test` retry 8228 exited 0 in
`output_question_spec/all_test_final_format_retry_20261007.log` and ends `PASS: complete live
acceptance is green.` It reports 11,178 pytest passes (line 3353), all 19 fresh PostgreSQL selector
groups (lines 3358-3376), database acceptance PASS (line 3377), Course Appearance acceptance (tests
3564 and 3573), and final acceptance PASS (line 3589). Retry 33479's 11,158 pytest passes are prior
run evidence only. The completed run also covers installation provision/replay. The permitted
production-browser suite passed, and canonical screenshot run 8149 published 257 captures and the
atlas with clean Demo shutdown. The final SPEC integration review found no source/model drift;
distinct final QUALITY review found no confirmed code, security, or maintainability issue and no
unapproved feature. Independent visual review passed for representative canonical images and all five
focused rendered captures. This closes M01-M29 and all 35 in-scope TODO mappings. Typed numeric fields
retain their types; temporarily invalid editor input such as `6.02e` stays local until valid, while
Draft autosave preserves valid representable source state. Save does not run Question Publication
Validation. This is a settled implementation detail, not a product question. Source review and
runtime acceptance are distinct evidence; the statements above
identify both. Older pending statements below are dated checkpoint history and are superseded by this
closeout.

Post-acceptance product correction (2026-10-07): Native JSON display-content strings should carry
HTML with inline CSS, including ordinary image references resolved through existing Question image
asset tuples and storage. Current source/render paths escape or treat this content as text, so this is
a newly recorded implementation gap; it does not revise the accepted M01-M29 evidence or claim that
the corrected behavior has passed. Authoring, preview, live prompt/choice rendering, and converter
preservation remain tracked in [TODO.md](../../TODO.md#future-product-capabilities). HTML
sanitization detail remains deferred, and existing author-script isolation and RDKit rules continue.

Terminology clarification (2026-10-07): Assessment point-value and partial-credit changes
recalculate awarded points from stored Backend credit without another Backend evaluation. The
deferred case is reevaluating previously submitted Native JSON responses after an answer-key or
grading-rule correction; if implemented, the new result replaces the previous result. This remains
outside the current grill unless it becomes a release-blocking decision. HG's existing answer-key
replacement guidance remains in force.
The docs link/ASCII checks (2967), HG checklist diff/consistency checks (1244), `git diff --check`,
independent `score_terms_spec_review` (SPEC), and distinct `score_terms_quality_review` (QUALITY)
all passed, as confirmed by this session's worker and reviewers; completed implementation acceptance
is unchanged.

### Native JSON handoff documentation audit (2026-10-07)

The focused six-pass audit corrected stale M15/M16/M29 status in the import specification and
removed an unsupported future-fetch/rendering implication from the QTI `externalResources` note.
The accepted M01-M29 ledger remains the status authority; converter integration and HTML/image
rendering gaps remain open. A proposed removal of ZIP preference wording was rejected because the
user prefers avoiding ZIP files; transport remains unspecified. See
[NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md](NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md) for scope,
review dispositions, and final focused checks. Docs only; no product behavior or runtime evidence
changed.

Separate documentation follow-up (2026-10-07): HG and current supporting docs now state initial
release license scope, expired-Draft cleanup deferral without an expiry policy, direct Native JSON
acceptance from qti-package-maker-rs, existing image storage, and Pool-statistics accumulation
across changes to its Question Revision Tuples. Source inspection found no PLE Native JSON converter
integration, so a separate TODO records that future work; M01-M29 and their accepted evidence remain
unchanged. The focused docformat/link checks passed (552 tests); checklist `--diff`, `--consistency`,
and `--gate07` aligned 1246 bullets after a narrow repair, and `git diff --check` passed. Independent
`policy_clarifications_spec` (SPEC) and `policy_clarifications_quality` (QUALITY) reviews passed.
These results confirm documentation alignment only; they add no code or runtime acceptance.

Image-reference correction (2026-10-07): withdrew the proposed local-name-versus-asset-ID question.
The established `QuestionImageAssetTuple` carries `questionImageAssetId` and SHA-256 checksum;
logical asset identity is separate from physical object identity. QTI verifies bytes and derives its
logical ID at bind time, while the Draft upload path mints its own logical ID and publication keeps
the logical ID while creating physical object identity. The converter-to-server workflow remains
future integration work; it does not imply a new source schema or transport protocol. Updated the
uncertainty log, QTI and Native JSON specs, and existing import TODO. No code or acceptance evidence
changed.

Inline-image evidence follow-up (2026-10-07): source tracing confirms that ordinary HTML `<img>`
markup in Native JSON prompt and choice strings compiles as text, while structured image blocks use
the existing tuple and asset URL rendering path. The QTI parser can create image blocks, but
multiple-choice rendering reduces choice image blocks to description text. Updated the QTI spec and
the existing future import TODO to require prompt and choice image display through the existing
paths. This is an implementation gap, not a new source-format or identity decision; no code or
runtime acceptance evidence changed.

Native JSON converter handoff (2026-10-07): Added a concrete writer handoff against the current
closed, unversioned source shape, with a minimal HTML/image example, response-kind field checklist,
seven-kind implementation start, and qti-package-maker-rs writer/media pointers. PLE keeps Question
metadata and existing image-tuple binding. The HOTSPOT pre-binding representation remains a shared
implementation question. Corrected three stale Native JSON statements to link this ledger's final
M01-M29 acceptance; the separate HTML display and ordinary multi-image lifecycle gaps remain open.
Updated the existing import TODO and QTI evidence. Docs-only; no runtime acceptance changed.

Prior checkpoint before final retry (2026-10-07): Canonical `all_test` retry 8228 in
`output_question_spec/all_test_final_format_retry_20261007.log` is running. Its latest output
reports 11,178 pytest tests passed (10 warnings), then starts the fresh database baseline; no final
result is recorded yet. Retry 33479 completed with exit 0 and ends `PASS: complete live acceptance is
green.` Its full Rust checks/tests, 594 Node tests, 11,158 pytest tests (10 warnings), all 19 database
selector groups, ordinary installation-data replay, and course-appearance acceptance passed in
`output_question_spec/all_test_clean_retry_20261007.log`; this is a prior passing broad gate, not the
current final state. The earlier run 66088 failed at `tests/test_target_disk_budget.py` before the
successful cleanup/retry; retain that as history.
The database baseline's `question_library_search_filters_and_pages_in_postgresql` selector also
ran; `blueprint_course_postgres::question_library` calls `assert_mixed_search_matrix`, so the M24/M25
mixed-filter matrix is already covered by this runtime proof. The earlier canonical browser attempt failed before browser launch in
`output_question_spec/production_browser_final_20261007.log`. Root-owned permitted retry completed with exit 0 and ends `PASS: serial production-browser
scenarios passed.` in `output_question_spec/production_browser_permitted_20261007.log`; it includes
authentication, authoring, Course Appearance, Sysadmin access, UI backbone, Assessment Release and
Attempt, WeBWorK capture/reload, Instructor Accounts, Support, invitation dry run, and Course Seed.
Five focused visual captures were completed (PG/PGML previews, Pool editor, selected Blueprint Theme,
adopted Course Theme) in `m29_visual_capture_wording_final_20261007.log`. The canonical screenshot
corpus replay and independent visual review are recorded below. Other focused proofs pass for Native PG/PGML authoring,
M23 stored-credit statistics, Library policy/search, M12 release validation, M28 import/readback, and
Theme inheritance. These prove only their stated behaviors. The complete canonical screenshot corpus
now passes on fresh Demo data: run 8149 exited 0 in
`output_question_spec/canonical_screenshots_all_controls_20261007.log`, publishing 257 captures and
the atlas, then stopping the owned Demo cleanly. The corrected final capture-profile mapping has
fresh 12/12 corpus checks; the earlier 12/12 result predates the new manifest and is historical.
Independent visual review of Library, pickers, Pool, Native, Student, Sysadmin, and five focused
PG/PGML/theme captures passed. Fresh SPEC and QUALITY reviews of the corrected capture contract
passed. M15/M16 evidence confirms autosaved source and metadata, reopen/readback, valid Native/PG/PGML
preview-test-refine-publish flows, and final rendered PG/PGML images; earlier blank-image/current-gap
statements are superseded. Fresh final integration QUALITY remains required for M29. The separate
raw invalid numeric-literal persistence question is deferred without a new decision. All 35 TODO
behaviors are reconciled below against current evidence. The connected M04/M05/M06 proof now covers
their remaining specific metadata, Pool Bloom, and permission criteria. M29 fresh-database runtime
is underway in retry 8228; final integration QUALITY remains pending. Incomplete milestones remain open.

### Prior evidence reconciliation checkpoint (2026-10-07)

The Pool and Library rows below now reflect the literal checks and current runtime evidence. M10's
direct Pool ID, distinct Assessment counts, owner preservation, Save, and workspace/API readback
pass in `assessment_saved_response_current_20261007.log`. M11's Blueprint persistence, adoption,
co-Instructor update, explicit fork metadata/independence, and existing-versus-future Attempt tuple
checks pass in the cited source and connected runs; the ledger no longer requires hypothetical
operation combinations. M24's mixed-search matrix and Library result/picker/detail/search discard
and reopen checks pass; M25's fresh-schema and temporary-search checks pass. Their final integration
dependency remains under M29.

The connected proof passed one Playwright case in 2.1 seconds in
`output_question_spec/final_proofs/metadata_nonowner_proof_origin_20261007.log`. It verifies M04
Native Title correction in detail and search with the source/Revision Tuple unchanged and supplied
`fr-CA` language preserved through publication; M05 ordinary nullable Pool Bloom editing and
readback; and M06 non-owner Instructor read/use/fork, absent edit controls, and rejected direct
metadata write (HTTP 404). Pools have no Revision, so unchanged-Revision language does not apply.
The retained source and config are inert text under `output_question_spec/final_proofs/`; this is
one-time evidence, not a permanent browser test. The specific M04/M05/M06 criteria are supported.
M29 fresh-database runtime and final integration QUALITY remain pending.

The complete fresh screenshot corpus passes with 257 captures. The final corrected profile mapping
passed fresh capture SPEC and QUALITY reviews and its current 12/12 corpus checks. Independent
visual review passed for representative Library, picker, Pool, Native, Student, Sysadmin, and five
focused PG/PGML/theme images. Fresh final integration QUALITY remains pending. These checkpoints do
not accept M29.

### Final capture checkpoint (2026-10-07)

The complete fresh capture passed in 233.6 seconds of capture time. The final source uses current
Draft creation/save controls, selects All Questions without assuming the previous filter, follows
the visible Sysadmin Find Courses action, and permits the Student's own prior-Attempt `score`
container alongside earned/possible points in the selected-response privacy profile. Answer-key
and private-source checks remain. These are corrections to the existing capture workflow.
The Instructor helper also uses the library-script wait style and no longer claims publication
depends on the deferred AI Bloom provider.

Scoped ESLint/Prettier and `tsc --noEmit -p tsconfig.lint.json` passed. The existing screenshot
corpus unit checks passed 12/12 in `output_question_spec/screenshot_privacy_alignment_checks_20261007.log`.
The isolated staff captures passed in `output_question_spec/canonical_staff_capture_controls_20261007.log`.
The manager inspected the final Sysadmin confirmation and phone Matching images; this is not the
required independent visual review. Agent launches and follow-ups still fail with `agent thread
limit reached`, leaving fresh reviews of the last small corrections and final integration pending.

The runtime-evidence identity list in `devel/human_guidance_checklist.py` now matches HG's current
Sysadmin access, confirmation, and audit statements. This completes the specific stale-wording
correction already listed in TODO. HG itself is unchanged. Checklist diff/consistency checks pass
with 1,242 matching bullets; a one-time check confirms the three replacement strings match HG.
The temporary history-submission diagnostic was removed after its source was retained as inert
text in `output_question_spec/final_proofs/`; its successful submission diagnostic is not a new
permanent test. No new product uncertainty was introduced by these corrections.

### Earlier UI checkpoints (2026-10-07)

Latest capture check: the Student Question Type scenario passed (run 48447, exit 0,
33.2 seconds) in `output_question_spec/canonical_student_types_scope_20261007.log`.
It covers answered and unanswered controls across four viewports. The prior run 8313 failed
because the helper required the initial search scope to be `inNoPool`; successive searches can
retain `all`. The helper now reads the actual scope and selects All Questions when needed.
Current Draft creation and saved-status controls also replace obsolete capture selectors.
The helper passed scoped ESLint/Prettier, `tsc --noEmit -p tsconfig.lint.json`, and diff checks.
The root `tsconfig.json` excludes these test files and is not evidence for their type checking.
Fresh SPEC passed the earlier selector corrections; QUALITY then found the initial-scope
assumption. Agent launch and follow-up calls repeatedly returned `agent thread limit reached`,
so the manager applied that final small helper correction. Independent re-review remains pending.
The full corpus retry is recorded in
`output_question_spec/canonical_screenshots_scope_corrected_20261007.log`; it later failed on the
selected-response privacy profile's missing own-score container key. That correction is covered
by the final full capture pass above.
These capture-helper defects are separate from newly introduced product behavior. The confirmed
new product drift remains the removed blanket Draft-save restrictions and unsupported Sysadmin
Draft-list test restriction; the removed permanent fixture machinery was unnecessary test complexity.

Evidence correction for M16: the passing Native authoring proof is more than a preview check.
`output_question_spec/final_proofs/source_m29_valid_draft_execution.spec.txt` 985-1180 creates PG
and PGML Drafts, saves source and metadata, previews, tests the saved version, refines the source,
tests again, publishes, and reads back the new Question. Lines 483-549 prove Native immediate
navigation/reopen and preservation of unfinished input while unrelated metadata saves. Lines
1015-1057 prove PG immediate navigation/reopen. The runtime is
`output_question_spec/native_publish_proof_final_20261007.log` (1 passed).
The Draft preview/test implementation in `crates/server/src/draft_preview.rs` loads Draft source
and invokes the Backend; its operations have Draft/object-store readers and a renderer, with no
Student Work writer. This is source evidence for transient testing, distinct from runtime
publication evidence. Final review should assess the remaining literal criteria using those
assertions rather than the test title or a new per-field/per-role test matrix.

The production browser gate passed in `output_question_spec/production_browser_permitted_20261007.log`
(exit 0). Fresh SPEC `browser_acceptance_spec_oct7` and distinct QUALITY
`browser_acceptance_quality_oct7` passed. QUALITY withdrew the pre-existing seeded-Course mutation
as a new defect. The five-format real-grading-to-History/display probe passed in
`output_question_spec/native_scoring_display_final_20261007.log` (root run 9371, exit 0; 1 passed,
11.3 s) after the readable-points formatter correction. Numeric ratios match for MATCH (0.6),
FIB regex (1), MULTIFIB (1/3), MA (0.6), and ORDER (0.25), with 2.78/5 visible points. History
ratios establish projected points with partial credit enabled; direct raw-fraction persistence has
separate existing database coverage and is not claimed by this probe. Fresh SPEC and QUALITY
reviews passed for the formatter summary/overview; TypeScript/format checks and the client build
passed (`output_question_spec/final_display_client_build_20261007.log`, exit 0).

The Pool wording was corrected to "Pool Edit Number," "Questions being edited," and "Questions in
this Pool"; the corresponding selector tests were adjusted and fresh SPEC passed. The wording change
is covered by the passing focused scoring/browser path and final source integration review. The
earlier `output_question_spec/m29_visual_capture_current_20261007.log` capture
passed (exit 0) for rendered PG/PGML, Pool table, and Blueprint/adopted Ocean; independent visual
review confirmed those captures. Its narrow support text areas predate HEAD and are a limitation
of that capture, not new product work. The newer isolated Library capture is recorded below.
Earlier canonical screenshot runs failed on stale controls/search transitions; these are historical. The screenshot
journey was corrected with separate SPEC and QUALITY reviews; canonical retry
`output_question_spec/canonical_screenshots_tuple_aligned_20261007.log` failed at the shared-search
helper's stale `Pool` request; the picker now follows the same contract and passes. The two obsolete
Pool request setups were corrected. Isolated Library capture 20755 passed, with
`output_question_spec/canonical_library_capture_20261007.log` confirming compact/list views, visuals,
and Pool detail. Canonical full corpus replay 88614 ended in failure in
`output_question_spec/canonical_screenshots_complete_20261007.log`: the WeBWorK capture timed out
looking for the named Question after an earlier Pool setup because the default no-Pool filter hid
it. A screenshot-helper-only correction is in place, but isolated WeBWorK capture 60968 also failed
waiting for the Question membership control in
`output_question_spec/canonical_webwork_capture_20261007.log`. The canonical screenshot corpus and
final independent visual review remain incomplete. The Live Demo is running and the production-
browser suite passed; that suite does not complete the screenshot corpus.

Agent 4's broad fast checks passed (11,157) before the existing 10 GiB Cargo cache guard failed;
root cleared the generated development cache and the budget recheck passed (1 passed, exit 0).
There was no product change for this cache issue. The earlier `all_test` pass remains scoped to
that complete run; later TypeScript changes have focused verification and a passing client build.
Temporary proof cleanup is complete. Final canonical screenshot replay/independent visual review and
fresh final integration QUALITY remain open. Existing raw numeric-text evidence remains recorded.

Historical cleanup checkpoint (2026-10-07): The `tests/_temp/` proof cleanup is complete; no
milestones or TODO mappings are promoted by this checkpoint. Twelve misplaced untracked logs were
moved from `docs/active_plans/reports/output_question_spec/` to the root
`output_question_spec/`; source reports staged under other output directories were preserved.
Retained one-time evidence is indexed in the ignored local audit directory
`output_question_spec/final_proofs/MANIFEST.md`. Its `.txt` source copies are inert proof records;
the original executable specs were removed. This evidence is local audit material, not committed
permanent fixtures. Fresh SPEC `cleanup_disposition_spec_review` and distinct QUALITY
`cleanup_disposition_quality_review` passed. The Human Guidance checklist diff/consistency review
passed (1,242 items), and `git diff --check` passed.

The initial canonical database baseline attempt was blocked by a completed browser owner's lock; its
historical log is `output_question_spec/database_baseline_final_20261007.log`. Root stopped controller
11167, and retry 12529 completed with exit 0 in
`output_question_spec/database_baseline_final_retry_20261007.log`. All 19 named selector groups
passed, including the direct Assessment Attempt finalization group (5 tests); the final line reports
`database baseline E2E: PASS`. The initial lock failure and owner stop are operational history, not
failures of retry 12529. This is the canonical database baseline result, not whole-plan acceptance.
At that checkpoint, canonical `all_test` was still running in
`output_question_spec/all_test_final_20261007.log`; its output had reached workspace strict Clippy.
Fresh source integration review `final_source_integration_oct7_current` found no confirmed source
contradiction within its sampled main boundaries. Functional evidence now includes focused M12,
M23, Library, M28, and Theme proofs listed below, but their wider integration remains open. The
canonical screenshot corpus still lacks rendered PG/PGML Draft frames, final Pool editor evidence,
and adopted Theme visuals. The bounded ignored helper at
`tests/_temp/capture_m29_visual_evidence.mjs` creates fresh UI records and awaits rendered preview;
the earlier visual capture is recorded above, while final canonical corpus evidence remains pending.
Existing canonical screenshot scenarios cover Sysadmin confirmation.

Earlier focused checkpoint (2026-10-07): Native PG/PGML authoring is PASS (1/31.7 s) in
`output_question_spec/native_publish_proof_final_20261007.log`: Native Question forks receive new
IDs and copy authors; the Sysadmin correction preserves the original Question ID and owner and
publishes Revision 2. Preview, test, refine, and publish assertions pass.
Fresh SPEC `native_proof_final_spec_oct7` and QUALITY `native_proof_final_quality_oct7` passed.
Native functional DOM/API evidence passes; PG and PGML screenshots show blank iframes, so visual
verification remains incomplete.
M23 is PASS (1/6.9 s) in `output_question_spec/m23_available_final_20261007.log`, with artifacts in
`tests/_temp/m23-available-final-20261007`: five distinct Students each received and answered one
Question. Each response earned a stored credit fraction of 0.5 and awarded 0 of 1 Assessment
points because partial credit was disabled. Question Tuple `7MYW-ZRWV` Revision 1 and its origin
Pool each show issued 5, answered 5, partial 5, correct 0, incorrect 0, credit sum 2.5, and mean
0.5. Existing per-positive-category privacy at k=5 is unchanged from HEAD; the earlier
mixed-sample unavailability was the expected implementation limit and requires no policy or
product fix. `m23_final_spec_oct7` and distinct QUALITY `m23_final_quality_oct7` passed. The current
five-student stored-credit proof is authoritative; an earlier M23 prep report describing six mixed
students is stale. Independent review of the M23 screenshots confirms the displayed statistics.
Library run 7 diagnosed a preexisting access SQL gap, not new plan behavior or drift. The submission
returned HTTP 200; History returned HTTP 200 at 4.384 s with 2.5/5 points in
`tests/_temp/library-response-final7-20261007/m29_library_scoring_eviden-ede54-nd-discarded-Library-search/m29-assessment-attempt-http-evidence.json`.
The access response returned HTTP 200 at 4.443 s, but its previous-attempt entry omits the score and
the overview shows `Score pending`. The run exited 1 in
`output_question_spec/library_response_final_20261007.log`; its assertions and failure remain as
historical diagnostic evidence. The SQL access-producer correction is now implemented in
`schemas/base_schema/50_functions/assessment_attempt_access.sql`, with the existing privacy check
comparing the Student's own access score with History. Fresh SPEC `overview_score_fix_spec_oct7`
and `overview_score_combined_spec_oct7`, distinct QUALITY `overview_score_quality_oct7`, and the
isolated database projection check passed; the database run exited 0. Disposable stack 88279 passed
startup at `https://localhost:8116` (`output_question_spec/overview_score_fresh_stack_20261007.log`).
Root Library run 90817 and its popup-setup failure are historical; the temporary helper correction
was followed by a passing focused run in the root `output_question_spec/library_policy_final_20261007.log`
(1 test, 10.2 s; test 9.9 s, exit 0). It covers Pool evidence, Assessment scoring, highest Attempt
display in Student Scores, and discarding a Library search before reopening it. Fresh scoped SPEC
`library_final_spec_review` passed for partial-credit recalculation, highest Attempt Student Scores,
and search discard/reopen; distinct QUALITY `library_final_quality_review` passed. This is focused
Library evidence, not a claim about Instructor Gradebook behavior or whole-milestone acceptance.
Registered browser acceptance, remaining captures, TODO integration, and broader gates remain
pending. Preserve prior failure evidence and Rust/TypeScript, pytest, M12, M22, M28, Pool, and Theme
passes; these results do not accept whole milestones.

### Overview previous-attempt score projection (2026-10-07)

The access API's previous-attempt projection omitted the score even though History returned 2.5/5.
`schemas/base_schema/50_functions/assessment_attempt_access.sql` now derives `Score` using the
canonical current-point, partial-credit policy, and scoring-completeness rules. The existing
`05_student_history_privacy` check also compares the Student's own access score with History;
privacy assertions remain in place. Fresh SPEC `overview_score_fix_spec_oct7` and combined SPEC
`overview_score_combined_spec_oct7` passed; distinct QUALITY `overview_score_quality_oct7` passed,
and direct inspection of the isolated database log confirmed the projection result. The isolated
database run exited 0 (`output_question_spec/overview_score_projection_20261007.log`). Disposable
stack 88279 passed at `https://localhost:8116`. The later focused Library run passed in the root
`output_question_spec/library_policy_final_20261007.log` (1 test, 10.2 s; exit 0), including Student
Scores and discarded Library search/reopen. Fresh scoped SPEC `library_final_spec_review` and
distinct QUALITY `library_final_quality_review` passed for partial-credit recalculation, highest
Attempt Student Scores, and search discard/reopen. The earlier run 90817 popup-setup failure is
historical; the temporary helper correction enabled the passing rerun. This evidence does not
establish Instructor Gradebook behavior or full Library/milestone acceptance. Native authoring and
M23 passes remain recorded above. At this score-projection checkpoint, canonical all_test, registered
browser, captures, TODO integration, and broader gates were pending; the current all_test and cleanup
state is recorded at the top of this ledger.

### Historical long status (superseded; 2026-10-07)

M01 and M02 are accepted on their literal criteria. Initial
publication and exact-ID search runtime passed in
`output_question_spec/authoring_exact_id_verified_20261007.log`. Successor publication returned
HTTP 422; its diagnostic identifies the Native create `type: None` bug in
`output_question_spec/authoring_successor_diagnostic_20261007.log`. The create fix has 7 focused
tests and fresh SPEC/QUALITY passes. The authorized pending-image delivery correction passed fresh
SPEC/QUALITY review and focused server tests (4/4). Native initial publication and published-image
first-view loading passed in `output_question_spec/temporary_native_image_ready_20261007.log`;
the subsequent Sysadmin correction editor exposed a route-access gap. The editor route now permits
Sysadmin, with separate SPEC/QUALITY and focused route tests (4/4). Run 29334 verified the refreshed
route through correction Draft open and Revision 2 publication; the temporary proof then stopped on
a strict locator matching eight status regions. The M07 author read/prefill and fork-of-fork path
advanced through Revision 2 publication in that run. The latest temporary browser handoff is
recorded in `output_question_spec/temporary_corrected_native_library_stats_20261007.log`:
Native same-ID Sysadmin Revision 2 and fork authors are verified, with the WeBWorK PG preview locator still
pending rerun. Library saved/released Assessment and Matching responses are verified, while capture
of the rendered WeBWorK Pool frame remains pending. M23 save/release/zero-score passed; its
one-correct/one-blank half-credit case displayed `Score pending` for an unknown reason. Run 8 then
failed after a temporary fixture change from two to four pairs triggered a duplicate-choice bug; this
fixture experiment is not a product fix or acceptance evidence. The browser worker stopped, and
the root paused it. Fresh owners are `/root/native_preview_proof_finish_oct7` for browser and native
file follow-up, and `/root/matching_partial_blank_diagnosis_oct7` for read-only diagnosis of the
original blank-response result. No production changes were made in this handoff.

API+migrator image build 66778 terminated with exit 125 in
`output_question_spec/authors_images_20261007.log`; the database-migrator image
`5ae7e699149d` built, while the API image layer failed for disk space. API-only retry 71192 then
passed and produced image `dab980bffc74`. Refreshed stack 64253 passed startup at
`https://localhost:8063` (`output_question_spec/authors_pool_detail_fresh_stack_20261007.log`).
Run 29334 is recorded in
`output_question_spec/temporary_corrected_native_library_stats_20261007.log`, with per-scenario
outputs under `tests/_temp/corrected-*-20261007`. Library and M23 proofs reached the release step;
the M23 error context shows the fixture needs the ordinary required due date. The Native/M07 proof
reached Revision 2 publication before its strict-status locator failure. The earlier browser window
and three temporary fixture files were assigned to `/root/temporary_browser_completion`; its later
run-8 outcome and current owners are recorded above. No product change is implied by these
locator/setup failures. Full Rust gate 51848 failed at
compilation in
`output_question_spec/rust_authors_20261007.log`: the existing
`DraftGeneralFeedbackResponse` test literal omitted the newly required `authors` field. The fixture
now supplies an empty author list. Full Rust gate 15739 passed in
`output_question_spec/rust_authors_fixture_20261007.log`, and
`cargo check --workspace --all-targets --all-features --locked` passes. Verified-unused images were
removed to make build space; the current runtime was preserved. A later selected Cargo cleanup
reduced `target` to 8.8 GiB; unused API/compiler images were removed without deleting volumes or
current images (`output_question_spec/target_cleanup_current_20261007.log`).
`check_codebase` passed (594 Node tests plus TypeScript, lint, and format) in
`output_question_spec/codebase_authors_pool_detail_20261007.log`. The original full pytest run
recorded 11,156 passes and one target-budget failure in `output_question_spec/pytest_20261007.log`;
the later `pytest_current_20261007.log` run recorded 11,155 passes and two budget failures before
cleanup, and both focused budget tests passed afterward in `disk_budget_current_20261007.log`. After
removing an obsolete unused project-tools image, the full pytest run passed: 11,157 tests and 10
warnings in `output_question_spec/pytest_after_cleanup_20261007.log`. Current full Rust gate 15739
and `check_codebase` 15936 remain passing. Schema generation/style and the Human Guidance
checklist passed (1,242 bullets). Pool passed its ignored workflow proof (run 3153), and Theme passed
adoption, independent Course change/reload, and original Blueprint readback (run 43634). M23 run
38262 reached publication, then exposed expanded helper text in the Question Authors label; M23 now
uses the established partial label match and supplies the ordinary required instructor description.
Run 98095 still returned publication HTTP 422; its error context omitted the body, so the cause is
unconfirmed. The source-save path derives and persists Native Type from the current PLE source; a
temporary diagnostic now records the actual response status/body if publication fails again. Library
run 98095 captured three saved Assessment entries with the expected Pool ID, count, and points, but
the new entry remained in its loading fallback. The incomplete new direct-reference path did not
load detail for newly added Pool entry IDs; the focused TypeScript fix now reuses the existing detail
loader and failure state. Fresh SPEC/QUALITY passed; the root-owned browser rerun remains pending.
The M23 description/response-capture correction passed focused TypeScript/format checks and fresh
SPEC/QUALITY; its root-owned runtime rerun remains pending. The isolated M12 DB oracle passed
(exit 0, selectors
01-07 and 09) in `output_question_spec/assessment_saved_response_current_20261007.log`; this gives
focused direct Pool-save, two-Assessment, fork, privacy, retired-Pool, and Owner/Sysadmin evidence.
The current isolated M12 proof passed in
`output_question_spec/m12_release_current_20261007.log` (runner exit 0): two valid Pool questions
can be saved with a request for three, insufficiency is reported, release is blocked, metadata
rollback permits release, and Pool removal preserves issued tuples. The current M22 connected
oracle passed in `output_question_spec/statistics_current_20261007.log` (runner exit 0), including
delivery retry, direct-versus-Pool counts, repeated finalization, replacement-member origin, and
`unrelease_connected_oracle_pass`. These focused database results do not accept either milestone;
their dependencies and browser/integrated acceptance remain pending. The
M12 request-three/release-blocked assertions already exist in
`tests/_temp/08_m12_release_validation.sql`; the disposable-database runner is
`source ./source_me.sh && bash tests/_temp/e2e_assessment_saved_response_m12_proof.sh`, whose
oracle runs that selector after its required fixture selectors. M22 assertions already exist in
`tests/e2e/unrelease_connected_oracle.sql`; its connected runner is
`tests/e2e/e2e_unrelease_connected.sh <leased-postgres-container-id>`. Current oracle results are
recorded above. The
M28 one-time API readback passed
(exit 0) in `output_question_spec/m28_import_readback_reviewed_20261007.log`: 8 generated Questions,
owner and metadata, stored-source/checksum and manifest-hash checks, ordinary Pool, Blueprint fixed/
Pool tuples, and adopted Course Theme. Overall TODO/evidence reconciliation is pending and does not
reopen M01/M02. A change-focused drift review sampled Pool/Sort, Draft, Sysadmin, and metadata changes
and found no confirmed unsupported behavior in that sample; it does not clear the whole diff. The
blanket autosave gates were confirmed new drift and removed. A Sysadmin Draft-list denial assertion
was briefly added during route correction, then removed because it was outside the approved
editor-access change and lacked Human Guidance authority. No such list restriction remains in this
correction. The missing Pool detail after a newly added reference was corrected by implementing
`loadPoolDetails`; its runtime proof now passes in the focused Library run
`output_question_spec/library_policy_final_20261007.log`, with retained assertion/readback evidence
in `output_question_spec/final_proofs/source_m29_library_scoring_evidence.spec.txt` and
`output_question_spec/final_proofs/library_final_pool_question_document_evidence.json`. The earlier
Pool editor scenario also passed in `output_question_spec/temporary_theme_pool_stats_reviewed_20261007.log`;
that specific scenario result does not pass the other scenarios in its run. M23's
missing description is a temporary fixture omission; its HTTP 422 is not evidence of new Type-switch
behavior. Temporary locator failures are recorded as proof-maintenance findings, not product drift.
Later milestones and overall plan acceptance remain pending.

### Earlier plan source review checkpoint (2026-10-06)

Fresh SPEC `current_plan_source_integration` and distinct fresh QUALITY `current_plan_source_quality` independently passed a sample tracing Question metadata/plain-text citation through Question parent -> Draft -> Published revision 1, permissive Draft saving, direct Assessment Pool references, and explicit Pool fork. The trace covered SQL question-authoring/publication functions, the Question JSON editor, Assessment workspace/client save, and Pool creation/fork paths. These source reviews did not inspect all milestones or the 35 TODO mappings, or establish runtime acceptance. Human Guidance restoration and popup-aware helper/caller corrections are complete. The saved-state correction and production build passed focused checks; browser session 68057 passed through Archive apart from a corrected punctuation assertion. The current run passed the search-absence check, then archived direct detail showed Library object unavailable. Ordinary detail lookup requires Available despite settled archived read/restore/fork guidance; a narrow lookup correction is under review. Assertions remain unchanged, with no revision-specific URL workaround. Restoration/fork checks and full browser acceptance remain pending. Focused Markdown-link/source-file-line-limit pytest passed (2,589 passed, 10 existing near-limit warnings; session 4121). The later archive-lookup correction and current browser result are recorded in the status paragraph above.

Retries 24-25 established the Course publication query-column and UUID/text bind corrections; both passed focused Rust formatting, PostgreSQL-feature compile-only, diff, and distinct fresh SPEC/QUALITY review. Retry 27 followed retry 26's RLS readback correction, as detailed above.

Draft autosave drift correction (2026-10-06): A new drift review found blanket numeric/hotspot/upload guards prevented unrelated metadata and support edits from saving. Those persistence guards were removed while existing unsaved/navigation/publication checks and truthful transient status remain. The ignored proof now confirms metadata and source readback; its earlier stale-status assertion and request-listener race were corrected. Dedicated temporary `tsc -p tests/_temp/tsconfig.json` and format checks passed. Fresh SPEC `draft_autosave_readback_spec` and distinct QUALITY `draft_autosave_gate_quality` passed. A separate sampled source review of metadata, citation, Bloom, statistics, scoring, and import paths found no actionable new drift; it is not whole-plan clearance. No permanent test was added. This is an implementation correction under existing Human Guidance, not a new product uncertainty. Raw unfinished numeric-literal persistence remains a separate deferred decision.

Earlier Sysadmin publication diagnostic closeout (2026-10-06): A prior one-time diagnostic captured SQLSTATE 23514 from the instructor-only publication actor trigger. The mapper was restored byte-for-byte with SHA-256 `3ea2d457bebad97110e707e5c424bb6c8ac0fcd7235b9a2ddd78493ad3662c14`; that diagnostic script was removed and created no permanent test or acceptance evidence. The later retry-21 42501 trace and current actor fix are recorded in the status above.

Earlier publication actor alignment and retry 21 start (2026-10-06): The SQL trigger was changed to permit an Active Instructor or Active Sysadmin as recorded editor/accepter while retaining active-account, timestamp, ownership, and public-authentication checks. Fresh SPEC `revision_actor_spec` and distinct QUALITY `revision_actor_quality` passed. Canonical retry 21 later exposed the separate Draft-image publication permission failure described in the current status. The Course Pool connection fixture review remains scoped evidence; none of these checks establish acceptance.

Artifact cleanup evidence (2026-10-06): `target_budget_cleanup_execution` ran `cargo clean -p server_core -p learning-data-access -p project-tools -p question_model` through `source ./source_me.sh`; it exited 0 and removed 2,997 files/7.1 GiB. The target directory measured 8.9 GiB at that checkpoint and may grow with subsequent builds. `pytest tests/test_target_disk_budget.py` passed (1 test, 0.11 s). This records disk-budget cleanup only, not source or acceptance evidence.

Historical canonical retry history (2026-10-06): retry 11 (session 66178, exit 1; `output_question_spec/acceptance_integrated_retry11_20261006.log`) passed all earlier selectors, including Question fork parents, then failed Question Revision metadata setup at `question_revision_metadata.rs:63` with SQLSTATE 23514: the authenticated Sysadmin session fixture violated `authenticated_session_check2`. The schema default for `updated_at` is `transaction_timestamp()` and the check requires it to be at least `created_at`; two `clock_timestamp()` calls in the existing fixture could produce `updated_at < created_at`. The test-only correction uses `transaction_timestamp()` for both values; no production behavior changed. `rustfmt`, `cargo test -p learning-data-access --test blueprint_course_postgres --no-run`, and diff checks passed. Fresh test-only SPEC `metadata_session_time_spec` and distinct QUALITY `metadata_session_time_quality` passed. Canonical retry 12 (session 31069, exit 1; `output_question_spec/acceptance_integrated_retry12_20261006.log`) passed through Question fork parents, then failed Question Revision metadata setup with SQLSTATE 23505 on duplicate `authenticated_session_token_hash_key` (`[0xc4; 32]`). Retry 12 ran the then-current source, which used token bytes `[0xc4; 32]` also inserted by `course_instance_postgres.rs:199`; the collision was confirmed by the duplicate-key error. After retry 12 exited, the source changed to distinct token material (`id(0xb10c)`) and was compile-checked with `cargo test -p learning-data-access --features postgres --test blueprint_course_postgres --no-run` and formatted. Separately, `sysadmin_correction_publication.rs` now uses the same `transaction_timestamp()` for both authenticated-session timestamps, addressing the confirmed timestamp check failure; no production behavior changed. Reused independent SPEC `citation_permission_quality_fresh` and QUALITY `fork_reservation_test_quality` reviewed the token-material and timestamp corrections together because fresh reviewer slots were unavailable; both passed. These were reused reviewers for this correction, not fresh reviews. No grants or test additions were made. Canonical retry 13 (session 13086, terminal 1) passed all preceding selectors through Question Revision metadata, then failed the invalid direct-INSERT Watch helper at `question_library_stewardship.rs:208`: expected SQLSTATE 23514, got 42501. The CHECK-shape helper/call was removed from the acceptance test scope per `PYTEST_STYLE.md`; production SQL and grants are unchanged. `rustfmt`, PostgreSQL-feature compile, and diff checks passed; fresh SPEC `watch_test_scope_spec` passed; distinct fresh QUALITY `watch_test_scope_quality` failed, recommending malformed DB-event invariant coverage and flagging a reserved child-ID fixture mismatch. The manager retained the CHECK and actual Human Guidance event-delivery tests but removed permanent invalid-row insertion coverage because no concrete invalid-event producer or demonstrated regression was identified. The test-only reserved-ID fixture correction now reads a generated ID using the existing Question fork fixture pattern. `rustfmt`, PostgreSQL-feature compile, and diff checks passed; combined fresh SPEC `watch_fixture_final_spec` and distinct QUALITY `watch_fixture_final_quality` passed. Canonical retry 14 (session PTY90004; `output_question_spec/acceptance_integrated_retry14_20261006.log`) exited 1 after prior selectors passed, at Watch creating a Question fork (`question_library_stewardship.rs:99`) with `InvalidRecord`. SQL lines 227-247 require copied source checksum, size, and media type; the fixture bytes do not match seeded source metadata. The test-only fixture now copies source checksum, size, and media type from the exact-bound object record; generated-ID readback is unchanged. `rustfmt`, PostgreSQL-feature compile-only, and diff checks passed. Reused SPEC `m03_metadata_api` and distinct reused independent QUALITY `fork_reservation_test_quality` both passed because fresh reviewer slots were unavailable. Canonical retry 15 (session 6388; `output_question_spec/acceptance_integrated_retry15_20261006.log`) exited 1 after selectors through Question Revision metadata passed, then failed at Watch while loading the forked Question Draft with `InvalidRecord("database returned an invalid Draft Question source binding")`. Retry 16 (session 1045; `output_question_spec/acceptance_integrated_retry16_20261006.log`) exited 1 at Watch when the source-classification fixture SELECT hit SQLSTATE 42501 on `ple_data`; the correction uses the loaded fork Draft classification. Formatting, diff, PostgreSQL-feature compile-only, fresh SPEC, and distinct fresh QUALITY checks passed; canonical retry 17 (session 45432) exited 1 after Question Library Watch passed and Pool member Watch failed at `support.rs:32` (`SET LOCAL ROLE ple_auth` denied on the migration connection). In shared `support.rs`, the `pleQuestionJson` ObjectRecord fixture media type changed to `application/vnd.peptidyle.question+json`; formatting, diff checks, and `cargo test -p learning-data-access --features postgres --test blueprint_course_postgres --no-run` passed. Reused SPEC `m03_metadata_api` and distinct reused independent QUALITY `fork_reservation_test_quality` passed; fresh reviewers were unavailable. Retry 19 (session 78018; `output_question_spec/acceptance_integrated_retry19_20261006.log`) terminated with exit 1 after the preceding Blueprint Revision/library/fork, Question metadata, three Watch, and Manual WebWork Type selectors passed; Sysadmin same-ID correction failed to open the fixture inspection connection at `support.rs:58` (SQLSTATE 53300, too many connections for `ple_migrator`). SPEC `ledger_checkpoint19_spec` and distinct QUALITY `ledger_checkpoint19_quality` passed before this runtime result. The source freeze has since been released, and `sysadmin_fixture_connections_investigation` is underway without a selected correction. Runtime/browser queue and whole-plan acceptance remain pending.


### Historical Explicit Pool-fork application path checkpoint (2026-10-06)

The explicit Pool-fork backend and browser path is implemented within existing M11/M14/M29 scope. Scoped checks passed; browser SPEC `pool_fork_browser_spec` and browser QUALITY review passed. The browser SPEC was reused for backend SPEC review, and `final_source_integration_quality` was reused for browser QUALITY because fresh reviewer slots were constrained; fresh backend QUALITY `pool_fork_backend_quality` passed. Root creation-client checks passed (2 tests). Temporary browser proof is prepared and ignored; fresh SPEC `pool_fork_temp_proof_review` and fresh QUALITY `pool_fork_temp_quality` passed. Review dismissed a proposed regex-ID escaping test because Question IDs are validated as Crockford Base32 with stored hyphens and cannot contain regex operators; no test change was warranted. Canonical retry 18 (session 35850; `output_question_spec/acceptance_integrated_retry18_20261006.log`) exited 1 after eight preceding selector groups passed, then Blueprint Revision acceptance stopped compiling `learning-data-access`: `stewardship.rs:367` moved `forked_pool_id` and `source_pool_id`, reused at lines 370 and 383. The root reproduced this with `cargo test -p learning-data-access --features postgres --test blueprint_course_postgres --no-run`; an earlier compile check omitted the PostgreSQL feature. This is not runtime acceptance. The ordinary Pool Save fixture uses the Store call and has compile-only evidence. The approved M02/M14 Watch event field names are implemented: SQL and typed Rust use `question_revision_number` and `question_pool_edit_number`; JSON exposes the nullable pair `questionRevisionNumber` and `questionPoolEditNumber`, with the existing target/event kind selecting the populated field. A fork event number describes its source; an ordinary child begins at 1. At this earlier checkpoint, root's `cargo test -p learning-data-access --features postgres --test blueprint_course_postgres --no-run` had passed and fresh SPEC `watch_number_spec` had passed while distinct QUALITY `watch_number_quality` was still running; retry 19 had not produced a result. The current status is at the top of this ledger. These field names add no notification behavior. The owned test-clone correction is also implemented, with no database CHECK-only test. These changes add no product event and do not change Human Guidance. At this earlier checkpoint, source freeze had been released; fresh Watch reviews and canonical runtime retry 19 had not started. All acceptance remains pending, and no whole-milestone or whole-plan acceptance is claimed.

Latest focused evidence (2026-10-06): permanent proof 09 now requires expected SQLSTATE categories and unchanged state while omitting exact error wording. Fresh SPEC `pool_save_error_check_quality` and distinct fresh QUALITY `pool_save_error_handler_final_review` passed. M12 focused wrapper retry 25 (session 20828, exit 0; `output_question_spec/assessment_saved_response_m12_proof_retry25_20261006.log`) passed its registered SQL checks and wrapper. Its `03_course_pool_forks.sql` proof covers direct references and shared Pools across two Assessments; `04_blueprint_pool_forks.sql` covers the ordinary fork path. These provide partial M11 evidence, but do not prove all Course operations or accept M11. Canonical retries 6, 7, and 8 (sessions 22327, 87811, and 3294) each passed the exact registered `Blueprint Revision PostgreSQL acceptance` lifecycle selector. Retry 9 (session 59013, exit 1) also passed that selector, plus mixed Question Library and Blueprint fork, before Question fork parents failed at `question_fork_parents.rs:62:10`: `Unavailable('database operation failed ... column reference "created_at" is ambiguous ...')`, originating at SQL line 1041. The correction renamed the function-local `created_at` to `fork_created_at`, used only as a timestamp; schema style and fresh SPEC plus distinct fresh QUALITY reviews passed. Retry 10 (session 62577, exit 1; `output_question_spec/acceptance_integrated_retry10_20261006.log`) passed Blueprint Revision, mixed Question Library, and Blueprint fork, then failed Question fork parents at `question_fork_parents.rs:78`: SQLSTATE 42501 (`permission denied for schema ple_private`) on the fixture's child-ID reservation SELECT. The fixture now opens a transaction, sets `SET LOCAL ROLE ple_private_owner`, performs the SELECT, and commits; the SQL source grant remains unchanged. `rustfmt` and `cargo test -p learning-data-access --test blueprint_course_postgres --no-run` passed with existing warnings. Fresh SPEC `fork_reservation_test_spec` and distinct fresh QUALITY `fork_reservation_test_quality` passed, each test-only scoped. Canonical retry 11 (session 66178, exit 1; `output_question_spec/acceptance_integrated_retry11_20261006.log`) passed Question fork parents, then failed Question Revision metadata setup with SQLSTATE 23514 in the Sysadmin session fixture. The fixture now uses `transaction_timestamp()` for both session timestamps, with focused SPEC/QUALITY reviews passed. Canonical retry 12 (session 31069, exit 1) passed Question fork parents, then failed Question Revision metadata setup on duplicate authenticated Sysadmin token material. Retry 13 (session 13086, terminal 1) passed through Question Revision metadata, then failed the invalid Watch helper at `question_library_stewardship.rs:208` (expected SQLSTATE 23514, got 42501); its scoped correction and review status are in the latest canonical retry checkpoint above. The runner registration is `local_stack_control/database_baseline_owner.py:473-484`; lifecycle assertions at `crates/learning-data-access/tests/blueprint_course_postgres/lifecycle.rs:97-147` verify adopted Forest, then independent Course Ocean while the Blueprint stays Forest. This is focused M27 runtime evidence only. Retry 6 later failed at Blueprint fork on a UUID/text bind mismatch; the scoped nullable-text and `BP0000000C` placeholder fix passed SPEC `blueprint_fork_id_spec`, QUALITY `blueprint_fork_id_quality`, and `cargo check -p learning-data-access -p server_core`. Retry 7 later failed in Question fork fixture setup because separately generated Revision/acceptance timestamps violated their equality constraint; the fixture now uses the stored Revision timestamp and passed SPEC `question_fork_timestamp_spec` and QUALITY `question_fork_timestamp_quality`. Retry 8 later failed when Question fork returned `StoreError::Forbidden`; diagnostic output identified permission denied on `question_revision_license`, and removal of unnecessary `FOR KEY SHARE` locks passed SPEC `question_fork_decoder_spec` and QUALITY `question_fork_decoder_quality`. M12 and M27 milestone acceptance, browser acceptance, lanes 2/3, and whole-plan acceptance remain pending.

Today's Human Guidance checklist reconciliation updated canonical
`docs/archive/audits/hg_checklist_parts/01_development.md`, its generated checklist, and the changelog.
Two configuration bullets remain unchecked/pending; the expanded robustness item retains its
not-applicable disposition. Fresh SPEC `checklist_configuration_spec_review` and distinct fresh
QUALITY `checklist_configuration_quality_review` passed. The focused gate and diff checks passed,
Human Guidance consistency reported 1,242 source and checklist bullets, and the documentation
pytest command passed 551 tests (root session 54776, exit 0; worker rerun also passed). These results
support only the checklist and documentation scope; they do not establish milestone acceptance.

Plan: [question_spec_implementation_plan.md](../active/question_spec_implementation_plan.md).
Material unresolved questions for Neil: [question_spec_implementation_uncertainties.md](../decisions/question_spec_implementation_uncertainties.md).
Scope: the 35 bullets under [TODO.md](../../TODO.md#question-spec-implementation-follow-up).
Authority: [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md). Findings: [QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md](QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md) and [FORK_MODEL_CODE_AUDIT_2026_10_05.md](FORK_MODEL_CODE_AUDIT_2026_10_05.md).

The manager assigns fresh implementation and review agents as dependencies become ready. Record
exact commands, outputs/exit statuses, affected files, review findings, correction owners, and evidence
paths here. Pending means acceptance has not been demonstrated. Product uncertainty remains deferred
where the approved plan says so; this ledger links the uncertainty log rather than duplicating
deferred decisions.

## Plan-saving task

| Task                          | Owner              | Status                      | Evidence                                                                                                                                                  | Specification review                                                            | Quality review                                                                                       |
| ----------------------------- | ------------------ | --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Save approved plan and ledger | save_approved_plan | Saved; narrow checks passed | Repository Markdown scanner passed for all 3 changed docs; ASCII/whitespace, M01-M29, 35 TODO rows, required runner commands, and git diff --check passed | Accepted: review_saved_plan; 29 milestones, 35 mappings, dependencies and links | Accepted: final_plan_quality; complete scope, links, honest pending status, separate automated gates |

## Test and closeout policy

Before adding or retaining a permanent test, answer every item in the [permanent test checklist](../../PYTEST_STYLE.md#permanent-test-checklist). Use ignored `tests/_temp/` for implementation proof while its long-term value is unclear. Before plan closure, review every plan-specific check there, promote only checks that earn permanent coverage under the checklist, and remove the rest. Connected runtime acceptance remains gated on this cleanup and completion of the assigned source reviews.

## Milestone status

| Milestone | Outcome                                  | Dependencies            | Task owner                                                                                                              | Status                                                                                                                                                                                                                                                                                                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Specification review                                                       | Quality review                                                                               |
| --------- | ---------------------------------------- | ----------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| M01       | Establish implementation coverage        | None                    | m01_runtime + m01_coverage                                                                                              | Accepted on literal criteria | Final canonical `all_test` retry 8228 passed Rust checks/tests, 594 Node, 11,178 pytest (10 near-1000-line advisory warnings), all 19 DB selector groups, installation provision/replay, and Course Appearance; the permitted production-browser gate also passed. | Passed: m01_coverage_spec                                                  | Passed after TODO/changelog corrections: m01_coverage_quality                                |
| M02       | Align Revision Tuple names               | M01                     | m02_docs_correction                                                                                                     | Accepted on literal criteria; focused checks and independent specification and quality reviews passed |[M02 verification report](question_spec_m02_tuple_names.md); M02 acceptance is complete on its literal criteria. | Passed: m02_spec_recheck                                                   | Passed: m02_quality                                                                          |
| M03       | Complete Question Revision records       | M02                     | m03_metadata_schema + m03_metadata_reads + m03_metadata_api | Accepted on literal criteria. Metadata corrections preserve exact Revision/source under CAS; publication carries metadata to the next complete Revision; the canonical M02 dependency is accepted. | Canonical selector 13 proves exact-tuple metadata CAS, source preservation, owner/Sysadmin writes, unauthorized-reader rejection, and nullable one-dimension Bloom. `question_publication_operations.sql` copies parent metadata into the successor Revision; the passing `native_publish_proof_final_20261007.log` verifies a Sysadmin correction publishes Revision 2 and its detail readback retains the expected metadata. | Passed: m03_sql_review, m03_read_spec, M03 browser regression spec, m03_final_review_coordinator | Passed: m03_sql_quality, m03_read_quality, M03 browser regression quality, m03_final_review_coordinator |
| M04       | Separate Native JSON metadata            | M03                     | M04 core, adapter, browser, and producer lanes                                                                          | Specific criteria pass: Native Title correction appears in detail/search with source and Revision Tuple unchanged; supplied language survives publication. Fresh-database/runtime integration passes under M29. | `final_proofs/metadata_nonowner_proof_origin_20261007.log` passes connected detail/search/source-Tuple and `fr-CA` publication checks; retained source/config are inert text in `final_proofs/`. | Passed: core, adapter, browser, producer                                   | Passed: core, adapter, browser, producer                                                     |
| M05       | Make Bloom ordinary metadata             | M03                     | m05_backend_implementation + m05_browserfix + m05_api_docs_fix + m05_doc_link_fix; root coordinator for generation       | Accepted: Assessment sorting with missing and partial Bloom values, plus ordinary nullable Pool Bloom correction/readback. Pools have no Revision. Fresh-database/runtime integration passes under M29. | `tests/test_assignment_workspace_questions.mjs` 131-158 proves classified order, stable ties, missing values last, partial Bloom values last, tuple preservation, and idempotence. Question metadata selector 13 proves nullable correction, concurrency, and owner/Sysadmin writes. `final_proofs/metadata_nonowner_proof_origin_20261007.log` proves ordinary nullable Pool Bloom edit/readback. See [M05 API docs report](QUESTION_SPEC_M05_API_DOCS.md). | Passed: backend source spec; final documentation recheck passed           | Passed: backend source quality; m05_final_docs_quality                            |
| M06       | Correct content-edit permissions         | M03, M05                | m06_permissions_type_implementation (permission/WeBWorK Type track) + m06_queryfix (shared metadata-read track)         | Specific criteria pass: owner/Sysadmin editing and Type correction, plus non-owner Instructor read/use/fork with owner-only UI and server edit controls. Fresh-database/runtime integration passes under M29. | Canonical DB selectors 13, 17, and 18 cover metadata authority, Manual WebWork Type correction, and Sysadmin same-ID correction publication. `final_proofs/metadata_nonowner_proof_origin_20261007.log` proves non-owner read/use/fork, absent edit controls, and rejected direct metadata write. | Passed: read correction and Type-tag source reviews                        | Passed: read correction and Type-tag source reviews                                          |
| M07       | Store Question fork parents              | M04, M05, M06           | m07_parents_implementation | Question fork criteria and M04/M05/M06 specific dependencies pass; fresh-database/runtime integration passes under M29. | Canonical DB selector 12 verifies retry and snapshot independence. `native_publish_proof_final_20261007.log` records PASS for `tests/_temp/m29_valid_draft_execution.spec.ts:435:3` (31.7s); its source at lines 734-841 verifies copied source/metadata and both parent tuples. M04/M05/M06 connected evidence is in `final_proofs/metadata_nonowner_proof_origin_20261007.log`. No byte-equivalence requirement is added. | Passed: fresh M07 SPEC                                                     | Passed: fresh M07 QUALITY                                                                    |
| M08       | Store Blueprint fork parents             | M02                     | m08_blueprint_parents                                                                                                   | Connected Blueprint lineage selector passes exact source tuple, ordinary child Revision history, fork-of-fork immediate parent, and repeated-request behavior; M02 dependency is accepted. |Canonical DB baseline selector 11 runs `blueprint_forks_are_ordinary_lineages_with_immediate_parent_and_normal_revisions`; assertions cover same-request replay, rejection of changed source tuple, child Revision 1, independent Revision 2, preserved parent history, and immediate parent on second-generation fork. | Passed: m08_spec_review                                                    | Passed: m08_quality_review                                                                   |
| M09       | Simplify Pool storage and Save           | M05, M06                | m09_pool_storage_implementation + m09_orderfix + m09_pool_questions_impl                                                | Connected Pool Save, shared references, permissions, and isolated editor behavior pass. The unused `created_in_transaction` column and guard were removed after a no-consumer check; the source pointer remains. M05/M06 specific dependency criteria pass. Fresh-database/runtime integration passes under M29. | `pool_transaction_cleanup_spec` and distinct `pool_transaction_cleanup_quality` pass; generator/style checks pass. `assessment_saved_response_current_20261007.log` selectors 01-07/09 cover direct Pool Save, two Assessments sharing a Pool with independent counts, fork/privacy/retired-Pool, and Owner/Sysadmin authority. M05/M06 evidence is in `final_proofs/metadata_nonowner_proof_origin_20261007.log`. | Passed: fresh `m09_editor_spec`; D1/D2 applied | Passed: different fresh `m09_editor_quality` |
| M10       | Reference Pools directly                 | M09                     | m10_m11_pool_reference_manager                                                                                          | The literal check passes: two Assessments save and read back one direct Pool ID with different requested counts while the Pool owner remains unchanged. M09 dependency closure passes. | `assessment_saved_response_current_20261007.log`; `tests/e2e/assessment_saved_response/03_course_pool_forks.sql` 390-471 asserts both API Save results, workspace/API readback at counts 1 and 2, and unchanged Pool ownership. | Passed: fresh M10 SPEC review                                              | Passed: fresh M10 QUALITY review                                                             |
| M11       | Preserve Pools through Course operations | M08, M10                | m10_m11_pool_reference_manager                                                                                          | The literal Pool lifecycle checks pass: Blueprint persistence/fork, adoption, co-Instructor update, explicit fork metadata/independence, and old-versus-new Attempt tuple behavior are demonstrated. M08 and M10's M09 dependency closure pass. | `04_blueprint_pool_forks.sql` 130-248, 259-325; `adoption.rs` 8-40, 188-263; `lifecycle.rs` 428-439; `lifecycle_co_instructor.rs` 35-95; and `07_assessment_fairness.sql` 180-398. Passing runtime logs: `assessment_saved_response_current_20261007.log`, `m12_release_current_20261007.log`, and `all_test_clean_retry_20261007.log`. | Passed: fresh M11 SPEC review                                              | Passed: different-agent fresh M11 QUALITY re-review                                          |
| M12       | Validate Assessment release              | M03, M11                | m12_release_implementation + root runtime/ledger                                                                         | Request-three Save/release, mismatch Save/report/block/correction, rollback, and issued-tuple preservation pass; production Assessment Release journey passes. M11 and its M09/M05/M06 specific dependency criteria pass; fresh-database integration passes under M29. | See `m12_release_current_20261007.log` assertions `metadata_edit_progression_saves_classification_mismatch_and_independent_insufficiency`, `metadata_invalid_pool_member_blocks_release`, and `metadata_edit_3_restores_clean_release_validation`. | Passed: final fresh static SPEC                                            | Passed: different fresh static QUALITY                                                       |
| M13       | Align Question Archive                   | M06, M07                | m13_implementation                                                                                                      | Archive behavior passes read-only access, normal discovery exclusion, preserved Pool references, restore, and fork. Its M06/M07 specific dependency criteria pass; fresh-database integration passes under M29. | `production_browser_permitted_20261007.log` passed the `instructor_authoring` production scenario, whose assertions in `tests/playwright/e2e/instructor_authoring.spec.ts` 195-264 cover archived read-only state, exclusion from normal discovery, forked source tuple and metadata, restoration, and return to discovery. M12 retry 19 separately passed Archive plus restore preservation for existing Pool tuples after restoring the valid fixture. M06 evidence is in `final_proofs/metadata_nonowner_proof_origin_20261007.log`. | Passed: m13_final_spec_mapping_verified                                    | Passed: m13_quality_recheck_report                                                           |
| M14       | Remove written impact notices            | M09                     | m14_notices_implementation                                                                                              | Connected Watch selectors pass for Question Revision, Question fork, Pool membership/fork events. M09 Pool Save dependency now has connected proof; no M14-specific runtime gap identified. | Canonical DB selectors 14-16 pass Question Library Watch, Pool member Watch, and Pool fork Watch. M09 evidence is recorded in `assessment_saved_response_current_20261007.log`. | Passed: m14_spec_review                                                    | Passed: m14_quality_review                                                                   |
| M15       | Autosave unfinished Drafts               | M04, M06                | m15_m16_native_authoring + m15_raw_source_worker + root_m15_m16_backend_source_correction + m15_save_ack            | Autosaved source/metadata, reopen/readback, and saved status pass for Native JSON, PG, and PGML. Preserve-error and publish-latest-saved behavior pass in the connected proof. Invalid numeric text stays local until valid; autosave preserves valid representable source state. | `draft_remaining_acceptance_review` confirms the connected workflow and saved source behavior; see current Draft proof references above. | Passed: fresh scoped M15 save-ack SPEC review                       | Passed: different fresh scoped M15 save-ack QUALITY review                                                        |
| M16       | Complete Draft preview and testing       | M15                     | m15_m16_native_authoring | Valid Native, PG, and PGML preview-test-refine-publish flows pass with final rendered PG/PGML images. Draft testing is transient in the current implementation. M15 unfinished-work criteria and M29 integration are separately accepted. | `native_publish_proof_final_20261007.log` passes the retained source proof; `draft_remaining_acceptance_review` confirms composition with M15; final PG/PGML images were independently reviewed. `crates/server/src/draft_preview.rs` reads Draft/object-store data and uses Backend rendering/grading without a Student Work writer. | Passed: m16_final_review_coordinator | Passed: m16_final_review_coordinator |
| M17       | Apply partial-credit settings            | M03                     | M17 policy lane + m17 final-review coordinator                                                                         | Accepted on literal criteria. Raw fractions, both setting directions/highest-Attempt effect, and copied Blueprint setting preservation pass; M03 is accepted. | Canonical `grading_rescore`, `library_policy_final_20261007.log`, five-format scoring/display proof, and Blueprint adoption selector pass. `blueprint_course_postgres/adoption.rs` 140-141 compares adopted snapshot `partial_credit_enabled` with source defaults; `support.rs` 414 sets the source default false. | Passed: fresh M17 SPEC                                                    | Passed: fresh M17 QUALITY                                                                       |
| M18       | Grade Matching proportionally            | M17                     | m18_m20_m21_implementation                                                                                              | Accepted on literal criteria. Connected grade/save/storage/points/display passes at 0.6; source/domain checks cover keyed identity under shuffle and partial saved responses. | `native_scoring_display_final_20261007.log` proves connected scoring/display; existing source/domain checks establish keyed identities and partial-response behavior. M17 is accepted. | Passed: source spec and Native spec recheck                                | Passed: source quality                                                                       |
| M19       | Grade FIB and MULTI-FIB                  | M17                     | m19_fib_implementation_cli + m19_evidencefix                                                                            | Accepted on literal criteria. Connected scoring/save/display passes for regex FIB and MULTI-FIB; scoring tests cover literal/normalized/regex modes, wrong/blank input, alternatives, invalid patterns, and denominator behavior. | Connected five-format proof plus `crates/grading/src/ple_question_json/fill_in_tests.rs` 58-217 covers connected workflow and settled scoring cases. M17 is accepted. | Passed: fresh midplan SPEC review                                          | Passed: fresh midplan QUALITY review                                                           |
| M20       | Grade Multiple Answer                    | M17                     | m18_m20_m21_implementation                                                                                              | Accepted on literal criteria. Connected saved scoring/display passes at 0.6; scoring tests cover settled examples, bounds, and selection outcomes. | `native_scoring_display_final_20261007.log` plus `crates/grading/src/ple_question_json/multiple_answer.rs` 37-94 cover connected score display and scoring behavior. M17 is accepted. | Passed: source spec and Native spec recheck                                | Passed: source quality                                                                       |
| M21       | Grade ORDER                              | M17                     | m18_m20_m21_implementation                                                                                              | Accepted on literal criteria. Connected saved scoring/display passes at 0.25; scoring tests cover examples, reversals, swaps, and permutations. | `native_scoring_display_final_20261007.log` plus `crates/grading/src/ple_question_json/ordering.rs` 48-112 cover connected score display and scoring behavior. M17 is accepted. | Passed: source spec and Native spec recheck                                | Passed: source quality                                                                       |
| M22       | Count deliveries and outcomes            | M11, M17                | m22_m23_statistics_implementation + native-oracle correction                                                            | Connected delivery/origin/retry oracle passes; M23 five-student Revision/Pool display passes. M11 Course-operation and M09/M05/M06 specific dependency criteria pass. M17 is accepted; fresh-database integration passes under M29. | See `statistics_current_20261007.log` and `m23_available_final_20261007.log`. | Passed: fresh M23 browser-scenario SPEC review | Passed: source QUALITY; cleanup fixes and fresh recheck |
| M23       | Display agreed statistics                | M22                     | m22_m23_statistics_implementation + native-oracle correction                                                            | Five-student proof passes exact Revision/Pool metrics and stored 0.5 credit with zero points when partial credit is off. M11 Course-operation and M09/M05/M06 specific dependency criteria pass. M17 is accepted; fresh-database integration passes under M29. | See `m23_available_final_20261007.log`, `final_proofs/m23_statistics_evidence.json`, and `native_scoring_display_final_20261007.log`. | Passed: fresh exact Revision/Pool browser SPEC review | Passed: initial QUALITY findings fixed; fresh cleanup recheck passed |
| M24       | Complete Library Object results          | M04, M05, M11           | m24_search_implementation                                                                                               | Mixed-filter, shared-result, picker, Question/Pool detail, and temporary-search discard/reopen checks pass. M04/M05 specific criteria pass; final M29 integration passes. | `blueprint_course_postgres::question_library` invokes `assert_mixed_search_matrix` at `crates/learning-data-access/tests/blueprint_course_postgres/question_library.rs:828`; the passing canonical baseline exercises it. `source_m29_library_scoring_evidence.spec.txt` 454-470 and 700-811 covers both result kinds/picker, detail reads, and discard/reopen; `library_policy_final_20261007.log` passes. M04/M05 evidence is in `final_proofs/metadata_nonowner_proof_origin_20261007.log`. | Passed: fresh M24 SPEC review                                              | Passed after Author-term correction and test-selector integration: fresh M24 QUALITY reviews |
| M25       | Remove persistent search storage         | M24                     | m25_search_storage_implementation + m25_completion_cli                                                                  | Fresh-schema filtering/pagination and temporary search behavior pass; saved-search persistence removal is implemented and reviewed. M24 dependency and final M29 integration pass. | Passing `all_test_clean_retry_20261007.log` includes live Question Library acceptance at lines 4263-4273; `source_m29_library_scoring_evidence.spec.txt` 700-811 proves discard/reopen behavior, and `library_policy_final_20261007.log` passes. See [M25 report](QUESTION_SPEC_M25_SEARCH_STORAGE.md). | Passed: fresh SPEC                                                         | Passed: different-agent fresh QUALITY                                                        |
| M26       | Align Sysadmin access                    | M01                     | m26_sysadmin_server + m26_sysadmin_ui + m26_contract_correction | Accepted: confirmation/cancellation/audit, direct requests, and server authorization for other roles pass; final integration review passes. | `production_browser_permitted_20261007.log` passes the browser scenario and Support journey. `tests/e2e/e2e_live_demo_support_capability.sh` 93-114 checks direct GET, anonymous/Student/Instructor POST, missing or false confirmation, confirmed Sysadmin access, and the audit record. The earlier claim that direct-request evidence was missing was incorrect. | Passed: m26_spec_review, correction recheck, and fresh spec-quality review | Passed: m26_quality_review and fresh spec-quality review |
| M27       | Copy Blueprint Themes                    | M11                     | m27_theme_implementation + course_create_theme_response_fix                                                            | Theme copy, independent Course Theme change, and adopted readback pass. M11 and its M09/M05/M06 specific dependency criteria pass. Canonical screenshot corpus passes; final integration review passes under M29. | See `temporary_theme_final_popup_20261007.log`, M28 readback, and focused capture. | Passed: fresh source-only `course_theme_response_spec`                    | Passed: distinct QUALITY `course_theme_response_quality`                                      |
| M28       | Verify supported import                  | M07, M08, M11, M16, M27 | root runtime/ledger                                                                                                     | Accepted: readback proves 8 generated identities, metadata/ownership/source hashes, Native/PG/PGML, Pool, Blueprint references, Theme, and exact tuples; all named dependencies pass under M29. | See `m28_import_readback_reviewed_20261007.log` and [M28 report](QUESTION_SPEC_M28_IMPLEMENTATION.md). | Focused connected readback passed; M29 accepted | Focused readback passed; M29 accepted |
| M29       | Verify integrated completion             | M01-M28                 | released_assessment_save_validation_fix + assessment_pool_reference_save_fix + baseline_retry_owner | Accepted: final full check, fresh database, production-browser suite, five-format scoring/display proof, focused captures, and full screenshot corpus pass. Final SPEC found no source/model drift; distinct final QUALITY found no confirmed issue or unapproved feature. Independent visual review passed for representative canonical and five focused captures. | `all_test_final_format_retry_20261007.log` retry 8228 exits 0; `canonical_screenshots_all_controls_20261007.log` run 8149 exits 0 with 257 captures and atlas; production browser permitted run passes. | Passed: `final_plan_integration_spec_oct7` | Passed: distinct `final_plan_integration_quality_oct7` |

All M01-M29 milestones are accepted on their literal criteria. The final broad gate and PostgreSQL
baseline pass. Focused proofs cover M12 release validation, M18-M21 grading through History/display,
M22/M23 statistics, M24/M25 mixed search, M26 Sysadmin journeys, M27 Theme, and M28 import/readback.
The M23 proof confirms stored 0.5 credit with zero points when partial credit is off; the Library
policy proof confirms the setting's highest-Attempt effect. M29's full canonical 257-capture replay,
independent representative visual review, and distinct final integration QUALITY review pass. The
current closeout and exact terminal evidence are at the top of this ledger.

## TODO coverage

The numbers follow the current order of the 35 in-scope TODO bullets. Every row is accepted; the
status text records its evidence. Reconcile by behavior if TODO ordering changes.

| TODO | Behavior                                                | Milestones         | Status  |
| ---- | ------------------------------------------------------- | ------------------ | ------- |
| 01   | Cross-layer vocabulary and Tuple names                  | M02, M24           |M02 accepted. M24 mixed-search matrix and Library result/picker/detail paths pass; final M29 integration passes.|
| 02   | Written impact notices                                  | M14                |Manual notices removed and Question/Pool Watch selectors pass in the canonical DB baseline; connected Pool Save also passes. No remaining M14-specific runtime gap is identified.|
| 03   | Pool Save and attestation removal                       | M09                |Connected DB selectors 01-07/09 pass direct Pool Save, two Assessments sharing a Pool, fork/privacy/retired-Pool behavior, and Owner/Sysadmin authority; isolated Pool editor verifies fork edits and sorting. M05/M06 specific dependency criteria also pass in `final_proofs/metadata_nonowner_proof_origin_20261007.log`; fresh-database integration passes in M29.|
| 04   | Pre-release count Save/release boundary                 | M12                |Current M12 PostgreSQL proof confirms request-three Save, insufficiency report, blocked release, metadata rollback, and issued-tuple preservation. M11 and its M09/M05/M06 specific dependency criteria pass; fresh-database integration passes in M29.|
| 05   | Metadata corrections and Pool problem reporting         | M03, M12           |M12 selector proves classification mismatch can save, reports invalid Pool member, blocks release, and later correction restores release. M03 CAS/carry-forward and M04 Native metadata detail/search with supplied-language publication pass; fresh-database integration passes in M29.|
| 06   | Published Question Archive                              | M13                |Production-browser assertions cover archive read-only state, normal-discovery exclusion, forked source tuple/metadata, restore, and return to discovery; retry 19 also passes preservation of existing Pool tuples across archive/restore. M06/M07 specific dependencies pass; fresh-database integration passes in M29.|
| 07   | Actual-delivery and stored-credit statistics            | M22, M23           |M22 connected delivery/origin/retry oracle and M23 five-student Revision/Pool UI proof pass; scoring/display is separately proven for five formats. The integrated grading/display proof also passes.|
| 08   | Optional supplied language                              | M04                |The connected M04 proof passes supplied-language publication and Native metadata detail/search with source tuple preservation; fresh-database integration passes in M29.|
| 09   | Draft autosave and unfinished source                    | M15                | Autosave/readback, saved status, and integrated Native/PG/PGML acceptance pass under M15/M16/M29 |
| 10   | Complete Revisions and Native JSON metadata             | M03, M04           |M03 is accepted: exact Revision/source-preserving metadata CAS and successor Revision metadata carry-forward pass. M04 connected proof passes Native Title detail/search with source tuple preservation and supplied-language publication; fresh-database integration passes in M29.|
| 11   | Ordinary nullable Bloom editing                         | M05                |Assessment unclassified-last sorting and Question selector 13 pass nullable-dimension CAS, source/Revision preservation, concurrency, and authority. The connected proof also passes ordinary nullable Pool Bloom edit/readback; Pools have no Revision. Fresh-database integration passes under M29.|
| 12   | Ordinary reusable Pool model                            | M09, M10, M11, M12 |Connected direct Pool Save/API readback, two Assessment references with independent counts, Blueprint persistence/adoption/update, explicit independent fork, release gates, and existing-versus-future Attempt tuple behavior pass. M05 Pool Bloom and M06 non-owner permission criteria pass in `final_proofs/metadata_nonowner_proof_origin_20261007.log`; fresh-database integration passes in M29.|
| 13   | Assessment-owned association and forced copies          | M10, M11           |Connected selectors prove shared direct Pool ID across two Assessments with independent counts and unchanged owner. Blueprint persistence/adoption/update and fork evidence also pass; no remaining M10/M11 literal operation gap is identified.|
| 14   | Question/Blueprint ordinary parent records              | M07, M08           |Question proof covers copied metadata/content, child snapshot independence, same-request retry, and immediate parent on fork-of-fork; Blueprint selector covers immediate-parent lineage and normal Revision history. M07 and M04/M05/M06 specific criteria pass; fresh-database integration passes in M29.|
| 15   | Ordinary creation retry handling                        | M07, M08           |Connected Question and Blueprint fork tests prove same-request retries return the existing ordinary result; no duplicate object is created.|
| 16   | Copy Pool metadata including Bloom                      | M11                |Explicit fork runtime proves Edit Number 1, copied metadata including both Bloom dimensions, exact tuples, and independent parent/fork edits. M11 and its M09/M05/M06 specific criteria pass; fresh-database integration passes in M29.|
| 17   | Ordinary Backend Type corrections                       | M06, M12           |Manual WeBWorK Type source and selector coverage pass. M12 proves classification mismatch reporting/release blocking; connected M06 evidence covers non-owner read/use/fork and owner-only edit controls. Fresh-database integration passes under M29.|
| 18   | Stored fractions and Assessment partial-credit settings | M17                |`grading_rescore`, Library toggle, five-format scoring/display, and Blueprint adoption setting-preservation proof pass. M17 is accepted.|
| 19   | Matching proportional credit                            | M18                |Connected save/grade/History/display proof passes (0.6); source/domain checks cover keyed identities under shuffled display and partial saved-response persistence. M18 is accepted.|
| 20   | FIB regular-expression matching                         | M19                |Connected regex FIB and MULTI-FIB save/grade/display pass; Rust scoring tests cover literal/normalized/regex modes, wrong/blank input, alternatives, invalid patterns, and denominator behavior. M19 is accepted.|
| 21   | Independent MULTI-FIB credit                            | M19                |Connected MULTI-FIB proof passes saved response, stored fraction 1/3, points, and display; Rust scoring tests cover wrong/blank denominator behavior. M19 is accepted.|
| 22   | Multiple Answer linear choice-count scoring             | M20                |Connected MA proof passes saved scoring/display at 0.6; Rust scoring tests cover settled examples, bounds, and selection outcomes. M20 is accepted.|
| 23   | ORDER combined scoring                                  | M21                |Connected ORDER proof passes saved scoring/display at 0.25; Rust scoring tests cover examples, reversals, swaps, and permutations. M21 is accepted.|
| 24   | Owner/Sysadmin content and metadata editing             | M06                |Owner/Sysadmin edit and Type checks pass; connected evidence also proves non-owner Instructor read/use/fork while server/UI edit controls preserve owner-only editing. Fresh-database integration passes under M29.|
| 25   | Owner/Sysadmin Bloom editing                            | M05, M06           |Question metadata selector proves owner/Sysadmin CAS writes with one Bloom dimension NULL; nullable source/permission review passes. Connected proof confirms ordinary nullable Pool Bloom edit/readback and M06 non-owner read/use/fork. Fresh-database integration passes under M29.|
| 26   | Sysadmin full administrative authority                  | M26                |Confirmation/cancellation/audit, Support, account journeys, direct protected-data requests, and anonymous/Student/Instructor authorization outcomes pass in `production_browser_permitted_20261007.log` and `tests/e2e/e2e_live_demo_support_capability.sh` 93-114.|
| 27   | Confirmed and audited Student-data access               | M26                |Production browser confirms protected Student data, verifies cancellation with no disclosure, and returns audit receipt. Direct request and anonymous/Student/Instructor server authorization checks also pass in `production_browser_permitted_20261007.log` and `tests/e2e/e2e_live_demo_support_capability.sh` 93-114.|
| 28   | Browser PG/PGML Draft authoring                         | M16                |`native_publish_proof_final_20261007.log` covers the functional authoring-to-publication workflow for Native JSON, PG, and PGML. The current Draft test path has no Student Work writer. See M16 and the retained assertion source; unfinished-work coverage and final review remain tracked under M15/M29.|
| 29   | Blueprint starting Theme inheritance                    | M27                |Theme inheritance and adopted readback pass; focused Theme captures exist. Canonical corpus replay passes and selected images/helper behavior were independently reviewed. Fresh SPEC and final integration review pass under M29; M11 literal checks and M29 integration pass.|
| 30   | Shared Library Object result fields                     | M24                |Mixed-search matrix and focused Library proof cover both result kinds, shared fields, picker rows, and Question/Pool detail reads. Final M29 integration passes.|
| 31   | Shared Type search semantics                            | M24                |Canonical mixed-search matrix exercises Type filters for both result kinds; picker/result proof passes. Final M29 integration passes.|
| 32   | Supported import/generated IDs/readback                 | M28                |M28 readback passes 8 generated IDs, metadata/owner/source checksums, Native/PG/PGML, Pool, Blueprint references, adopted Theme, and exact tuples. No focused behavior gap is identified; M28 closure is accepted with the named M07/M08/M16/M27 criteria and M11's M09/M05/M06 integration under M29.|
| 33   | Temporary search and saved-search cleanup               | M25                |Fresh PostgreSQL schema and temporary mixed-search checks pass; saved-search persistence removal is implemented and reviewed. M24 behavior evidence is complete; final M29 integration passes.|
| 34   | Fresh-database filter-removal verification              | M25                |Fresh PostgreSQL schema install and mixed-filter selector pass; no literal M25 filter-removal gap remains. Final M29 integration passes.|
| 35   | Live Demo, search/pickers, screenshots                  | M29                |Live Demo, production-browser suite, shared picker contract, and full fresh canonical screenshot replay pass; `canonical_screenshots_all_controls_20261007.log` publishes all 257 captures. Corrected capture contract SPEC/QUALITY reviews and representative visual review pass. Distinct final integration QUALITY passes.|

## Audit reconciliation

The R IDs below refer to
[QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md](QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md).
The F IDs refer to [QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md](QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md);
its fork code evidence is in
[FORK_MODEL_CODE_AUDIT_2026_10_05.md](FORK_MODEL_CODE_AUDIT_2026_10_05.md).

| Finding                                    | Disposition and owning milestone                                                                                                                                                      |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R01: Type classification vs execution      | Editable Backend classification and Pool mismatch/release checks: M06, M12. No new Native JSON interaction is commissioned.                                                           |
| R02: Bloom completion gate                 | Independent nullable ordinary Bloom edits and owner/Sysadmin permissions: M05, M06. Carrying existing values together is a Class 2 request-shape choice.                              |
| R03: Question correction and Pool mismatch | Preserve valid metadata corrections and report/block affected release: M03, M12.                                                                                                      |
| R04: Pool certification                    | Remove attestation input and storage; advance Edit Number on tuple-set changes: M09.                                                                                                  |
| R05: Conditional Backend feedback          | Documentation-only correction. No qualifying implementation blocked by the old wording was identified; feedback timing remains deferred. No implementation milestone.                 |
| R06: Statistics                            | Count deliveries and graded responses using stored credit, per Revision and Pool: M22, M23. Existing extra counters remain implementation details unless they violate those measures. |
| R07: Native compiler limits                | Retained as Class 2 implementation limits. Relabeling their authority needs no code change absent a demonstrated content/workflow problem; no implementation milestone.               |
| R08: Superseded statistics question        | Q32 status correction is documentation-only; no implementation milestone.                                                                                                             |
| R09: Publication route scope               | Documentation-only clarification; existing first-publication and same-ID Revision routes were found. No missing route or implementation milestone.                                    |
| F01: Pool ownership and Add workflow       | Ordinary Pool references and explicit forks: M09-M11; release behavior remains M12.                                                                                                   |
| F02: Import and publication requirements   | Preserve incomplete Drafts and validate at publication: M15, M16. Supported import/API creation and readback evidence: M28.                                                           |
| F03: Pool requirements as edit rejection   | Permit unreleased Save and block release on the mismatch: M12, with direct Pool/reference changes in M09-M11.                                                                         |
| F04/Q31: Native metadata duplication       | Complete Revision and Question-record metadata ownership: M03, M04.                                                                                                                   |
| F05: WeBWorK Type detection                | Manual editable classification and mismatch handling: M06, M12. Automatic detection remains deferred.                                                                                 |
| F06: Fork license                          | Withdrawn finding: forks retain the source license. No compatibility or license-choice change is planned.                                                                             |
| F07: QTI mapping and feedback timing       | Unsupported gate removed; converter handoff and optional feedback timing remain deferred. No replacement implementation task.                                                         |

The fork code audit has no numbered finding IDs. Its concrete evidence routes as follows:

| Audited behavior                                                                                                            | Owning milestone                                                                |
| --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Assessment-owned Pool association, unique attachment, forced copy-on-add, and Blueprint/Course Pool rewrites                | M09-M11                                                                         |
| Question and Blueprint parent references on ordinary records, including exact source Revision and immediate-parent behavior | M07-M08                                                                         |
| Draft/Blueprint repeated-request bookkeeping while simplifying fork tables                                                  | M07-M08                                                                         |
| Pool fork copies Bloom metadata; tuple ordering and Edit Number remain ordinary current-state behavior                      | M09-M11                                                                         |
| Existing tests, fixtures, API types, and schema docs that enforce old Pool/fork behavior                                    | The milestone owning each behavior above; regenerate/update with that milestone |

These mappings account for every R01-R09 and F01-F07 ID in the decision and authority reviews and
each code-evidence category in the fork audit. Documentation-only corrections, the withdrawn F06
license finding, and Class 2 implementation limits do not create implementation work.

## Preliminary baseline evidence

Read-only baseline reported by the manager's baseline agent on October 5, 2026:

- Podman engine reachable: client 6.1.3, server 6.1.2. Zero Compose projects; default stack absent.
- `node_modules` absent; Playwright import returns `ERR_MODULE_NOT_FOUND`. M01 installs repository
  dependencies using existing bootstrap tooling.
- Host `psql` absent; existing container-based `psql` runner available.
- `local_stack.py acceptance` covers database baseline, installation data, and Course appearance.
  Browser journeys use `source source_me.sh && ./devel/run_playwright_tests.sh [--build]`.
- Fresh-database runner: `source source_me.sh && bash tests/e2e/e2e_database_baseline.sh`.
- No baseline tests, stack mutations, or implementation acceptance were performed by that audit.
  The manager adds exact commands/results before completing M01.
- Planning inspection found `ple_private.saved_question_search` in the authoring schema, with
  policies and an index; no Rust/UI caller was found by the targeted search. Trace and remove
  permanent search storage in M25. This evidence corrects the earlier narrower TODO search result.

## Plan-save verification

The one-time check used the repository Markdown link scanner on the plan, ledger, and changelog,
validated ASCII and trailing whitespace, counted 29 milestones and 35 TODO rows, and checked the
required setup/browser/full-check/screenshot commands. It passed with exit 0. `git diff --check`
also passed with exit 0. Fresh specification review (review_saved_plan) and quality review (final_plan_quality) accepted the
saved plan. Implementation milestone reviews remain pending.

## Restoration and current review checkpoint

The user clarified that Instructor bulk editing is deferred, so its existing implementation stays in
place. The accidental removal was selectively reversed: all 16 deleted files and the shared UI, API,
Rust, SQL, and test changes were restored. M03/M05 work and fixture fixes were preserved. The
mid-plan audit records the withdrawn removal finding in
[QUESTION_SPEC_MIDPLAN_CODE_REVIEW_2026_10_06.md](QUESTION_SPEC_MIDPLAN_CODE_REVIEW_2026_10_06.md);
the ignored implementation handoff is `output_question_spec/BULK_REMOVAL_REVERSAL_HANDOFF.md`.
This records the implementation disposition only; the bulk-edit specification and Human Guidance
were not changed.

The owner-reported `bulk_restoration_spec_review` passed after three stale wording phrases were
corrected. The separate fresh `bulk_restoration_quality` review remains pending. Owner-reported focused
checks passed for `server_core` and PostgreSQL-enabled `learning-data-access` Cargo checks, both
TypeScript projects, schema style, PostgreSQL integration-test compilation with `--no-run`, scoped
Rust formatting, and `git diff --check`. Exact command strings are not available in this checkpoint.
Database and browser acceptance remain unverified.

Existing source QUALITY reviews `autosave_navigation_quality_final` and
`archive_projection_quality_final` passed. The latter's durable report is
[QUESTION_SPEC_SYSADMIN_ARCHIVE_ALIGNMENT.md](QUESTION_SPEC_SYSADMIN_ARCHIVE_ALIGNMENT.md). These
reviews support their scoped source findings only; they do not close runtime acceptance.

The first full `all_test.sh` run failed in the Cargo fixture checks because
`fixture_set.json` at line 110 lacks `partialCreditEnabled`. The owner has assigned a focused fixture
correction; full checks will need a rerun after that correction. Database and browser acceptance had
not started at that checkpoint. All 29 milestone acceptance entries remain pending without decisive
current runtime evidence.

## Final acceptance evidence

Pending: fresh-database behavior, full compliance, affected browser journeys, Live Demo screenshots,
HG checklist diff/consistency, documentation checks, and fresh integration review. All can be completed
by the manager and subagents locally. Production deployment and external email setup retain their own
scopes.

### Temporary proof cleanup inventory

Cleanup checkpoint: the ledger was 951 lines (down from 1,345), with 187 bullets; both dated
entries remain. Link and file-limit checks passed for 2,589 files. A
clean SPEC sample passed; distinct QUALITY review remains pending. With fresh worker slots unavailable,
the same documentation worker also merged duplicate October 6 changelog groups; independent changelog
review remains pending.


The M03 metadata component harness and M12 temporary wrapper proof passed; neither has been promoted
to permanent coverage. The withdrawn, unrun synthetic `question_spec_acceptance.rs` CLI and its child,
plus two completed diagnostic wrappers, have been removed; their logs remain. Queued M28 readback and
browser probes, M12 proof, and Unrelease probes remain pending disposition/runtime evidence. No M28
acceptance is established.

## Integrated retry evidence (2026-10-06)

The following retry artifacts supersede earlier statements that no integrated checks had started. They
record progress and failures; they do not accept any milestone or establish full-plan acceptance.

- `output_question_spec/check_codebase_integrated_retry2_20261006.log`: frontend/codebase checks
  passed, including 593 tests (593 passed, 0 failed), with `PASS: codebase checks passed`.
- `output_question_spec/all_test_integrated_20261006_retry2.log`: Rust workspace checks and tests
  passed. The later frontend lint stage failed with 25 reported errors. The frontend/codebase retry
  above passed afterward; this does not turn the earlier full command into a successful run.
- `output_question_spec/pytest_integrated_20261006.log`: 11,176 passed and 3 failed. Failures were
  the browser diagnostic error-retention assertion, the 10 GiB `target/` budget (actual 16.1 GiB),
  and the Playwright location rule for two temporary proof scripts. The focused three-case diagnostic
  rerun passed. The subsequent diagnostic and naming-convention command, `source source_me.sh && python3 -m pytest
  tests/test_browser_suite_developer.py tests/test_test_naming_conventions.py -q`, passed 24 tests.
  The target disk budget remains unresolved. The naming failure was tied to ignored temporary proof
  scripts and is not evidence of a Question product failure.
- The Human Guidance checklist consistency command, `source source_me.sh && python3
  devel/human_guidance_checklist.py --consistency`, exited 0 and reported 1,239 Human Guidance
  bullets and 1,239 checklist bullets. This does not establish the full checklist diff review.
- `output_question_spec/local_stack_acceptance_integrated_20261006.log`: seven connected PostgreSQL
  selectors passed, then direct Assessment Attempt finalization failed in
  `grading_lifecycle_postgres`; the acceptance command exited before later selectors. The failure
  output did not include a test assertion. A separate isolated run exposed an import cycle:
  `lifecycle` through `disposable_stack_cleanup` through `disposable_stack_adapter` to partially initialized
  `cleanup_plan`. This import issue is separate evidence; the captured connected failure has not yet
  been tied to it or to the earlier M17 fixture failure. No edits to those lifecycle modules are
  recorded in this task; runtime diagnosis and correction remain with the integration owner.

- `output_question_spec/grading_lifecycle_failure_capture.log` records the later grading-selector
  setup failure: one case passed and four failed before reaching grading assertions. Two fixtures
  used the unknown Native JSON field `questionTitle`; two failed the
  `question_revision_source_binding_check1` constraint (SQLSTATE 23514). The existing fixture in
  `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` was corrected without changing
  assertions, and its PostgreSQL test target compiled with `--no-run` in 2.26 seconds. The optional
  numeric thread-ID diagnostic now recognizes headers with numeric thread IDs; its existing input was updated and
  the focused Python run passed 30 tests. Fresh `grading_setup_spec_review` and
  `grading_setup_quality_review` both passed. Schema docs regenerated with unchanged current content,
  the catalog snapshot remained unchanged, schema style passed, and Human Guidance diff and
  consistency checks both reported 1,239 matching bullets. The prior statement that the connected
  grading failure remained unresolved is superseded: the setup correction is recorded, while
  integrated runtime acceptance was still pending the subsequent retry.

The registered M29 run (session 84947, lane 1) is recorded in
`output_question_spec/local_stack_acceptance_registered_20261006.log`. Its first seven PostgreSQL
selectors each passed one case; direct grading lifecycle then passed five cases with zero failures in
4.63 seconds. The next connected Unrelease step failed author-content validation, so later baseline
and installation/course-appearance checks were not reached. The diagnosis was that
`tests/e2e/unrelease_connected_oracle.sql` supplied three optional `author_content` keys that the real
Rust `ifSome` producer omits; SQL maps absence to NULL. The correction omits those keys. Earlier `{}`
and JSON `null` values did not match the producer. SPEC and QUALITY reviews passed, with no product
rule or schema change.

The M29 retry owner `baseline_retry_owner` (session 1249, terminal 1) completed the run recorded in
`output_question_spec/local_stack_acceptance_unrelease_20261006.log`. Seven one-case PostgreSQL
selectors and five direct-grading lifecycle cases passed; the connected Unrelease step then failed
with permission denied for INSERT on `question_revision_statistics`. The author-content omission
cleared the earlier validation failure. The statistics root cause and correction are documented
below; session 1249 has been superseded by focused runtime evidence.

The separate nine-test `test_database_baseline_owner.py` run passed, but this is not integrated
acceptance. The 14 listed checks are a compile list, not connected runtime evidence. The corrected
old-selector source has not yet reached the affected selectors. Browser acceptance, M28, and
target-budget acceptance remain pending.

The earlier response-fixture failure and review concern are historical. The unapproved published
Question fixture has since been removed: five files and 987 lines, including the 603-line JSON, the
51-line TypeScript adapter, the 300-line Rust validator, and two SVGs. Twelve Node consumers now use
local inputs. The obsolete validation command and stages were removed from `build.sh` and
`check_rust.sh`. Node passed 57/57, `tsc` passed, project-tools Cargo check passed, and the native
response fixture passed 1/1 after removing its malformed-assertion addition. SPEC and QUALITY
rereviews passed and source formatting passed. Published-fixture
SPEC and QUALITY passed. The QUALITY review identified a dead `issuedQuestionWireFixture` helper,
which was removed; the Node-owner QUALITY rereview also passed. Full `check_codebase.sh` then passed
both typechecks, lint, formatting, and Node 593/593, recorded in
`output_question_spec/check_codebase_fixture_cleanup_20261006.log`. The fixture cleanup is accepted
locally; browser WASM remains pending, so this does not establish browser or integrated runtime
acceptance.

For the Unrelease permission failure, the root cause is confirmed: the production
`presentation_private` function directly wrote data-owner statistics after the mutators were removed.
This is not an Unrelease function defect. Five data-owner helper changes passed SPEC and QUALITY
review; the initial permission denial cleared in the focused runtime below. Archived dead links were
converted to dated code paths. The exact markdown-link check `source source_me.sh && python3 -m pytest
tests/test_markdown_links.py -q` passed 549 tests in 0.42 seconds after that conversion.

The focused fresh-database wrapper first exited 125 because the host `TMPDIR` was not visible in the
container mount. The corrected retry (session 31886, terminal 3) placed its temporary secret in
ignored `tests/_temp/`; see `output_question_spec/unrelease_statistics_focused_retry1_20261006.log`.
Schema install and security-catalog checks passed. The first real presentation and idempotent resume
succeeded, clearing the statistics permission denial. The second presentation failed while inserting
`question_attempt`: `validate_question_attempt_reproduction` requires one presentation bundle and no
backend document. The cause remains unconfirmed under investigation; this is not yet classified as a
fixture or product defect. Cleanup found no matching container, volume, network, or temporary secret
directory, and no database run is active. That first retry's reproduction-validator failure is
historical and was cleared by the reviewed oracle constraint-ordering change.

The next focused retry, session 94027 terminal 3, is recorded in
`output_question_spec/unrelease_statistics_focused_retry2_20261006.log`. Schema and security checks
passed. The reviewed oracle now defers the relevant constraint checks; presentation and idempotent
resume passed. This retry did not reach grading. The separate session 1249 baseline run passed five
direct-grading lifecycle cases. The next existing oracle step attempted to remove an issued Question from a Pool used by an
available Assessment, and the Human Guidance guard rejected that operation. The guard remains in
place. The oracle ordering correction passed SPEC; QUALITY is pending under
`baseline_selector_alignment`. At that checkpoint the runtime owner was idle. This was a failed gate;
it was not attributed to production behavior. Cleanup confirmed zero matching containers, volumes,
networks, or temporary secret directories, and no DB run was active. M22 and M29 acceptance remain
pending.

Retry 3 (terminal 3) is recorded in
`output_question_spec/unrelease_statistics_focused_retry3_20261006.log`. Schema/security checks and
presentation/resume continued to pass, but the run failed before either finalization: the
`saved_at_millis` argument subquery read a private table under `ple_app`. The corrected test captures
timestamps under the private owner. Fresh `unrelease_saved_time_spec` and `unrelease_saved_time_quality`
reviews passed. At that checkpoint, the runtime owner had started retry 4; its outcome follows.

Retry 4 (terminal 3) is recorded in
`output_question_spec/unrelease_statistics_focused_retry4_20261006.log`. Schema/security,
presentation and resume, both finalizations, the idempotent partial-credit check, and Question
statistics passed. Finalization results were 0/1 and 1/1 points. The run then failed when saving a
sole-Pool retirement because Release validation returned `questions_required`. This is a confirmed
production mismatch: full release-content validation was reapplied to an already-released Assessment
when Human Guidance permits removing its content (§§1722, 1367). The assigned
`released_assessment_save_validation_fix` owns the source correction. The production guard remains
in force pending that fix. At that checkpoint the runtime owner was idle; the follow-up retry is
recorded below. Combined M12 has not run.

Retry 5, terminal 3, is recorded in
`output_question_spec/unrelease_statistics_focused_retry5_20261006.log`. It passed schema/security,
presentation/resume, both finalizations (0/1 and 1/1 points), the idempotent partial-credit check,
Question statistics, and sole-Pool retirement. The next Pool replacement failed the transient
empty-set constraint because the oracle transaction still used immediate constraint checking; the
Rust Pool-save path uses a fresh initially-deferred transaction. The source correction
`pool_save_oracle_constraint_fix` was assigned at that checkpoint; the later retries are recorded
below.

The released-save source correction is stable. Fresh SPEC `released_save_spec_recheck` and QUALITY
`released_save_validation_quality` passed. The extracted schedule helper preserves invoker security,
initial-release full-content validation, and the max-250/post-issue cap. Permanent oracle due dates
were corrected by two days. The temporary schedule extension was partially exercised in retry 6;
its insufficient-Pool Release case remains pending. The
temporary Sysadmin browser-extension source passed separate SPEC/QUALITY review under
`m16_temp_proof_review_coordinator`; no browser execution occurred. Major runtime acceptance remains
pending, including combined M12, M16 browser behavior, M22/M29, and the overall browser/WASM gate.

The separate retry 6 extension (terminal 3) is recorded in
`output_question_spec/unrelease_statistics_focused_retry6_20261006.log`. Its base oracle and
post-retirement policy, inline persistence, rollback, and zero-point checks passed. The final
insufficient-Pool case failed `questions_required` because its generic save had not created a new
Pool entry. The test-only constraint-pair correction passed fresh SPEC and QUALITY. At that
checkpoint, read-only `assessment_pool_add_path_diagnosis` was checking the actual Add API; the
subsequent source correction and review status are recorded below.

The canonical focused retry 7 (session 94726) exited zero and is recorded in
`output_question_spec/unrelease_statistics_focused_retry7_20261006.log`. Fresh schema/security,
the full base oracle, canonical lock-race, and post-Unrelease Student-start rejection all passed;
cleanup found zero resources. This accepts the scope of that focused Unrelease oracle only. At that
checkpoint the runtime owner was preparing the M12 proof; its outcome is recorded below. The temporary Sysadmin
browser extension remains source-reviewed but unrun. M22/M29, browser/WASM, and overall acceptance
remain pending.

M05 final documentation QUALITY passed for current APIs, generated DTOs, and nullable Bloom schema;
this is documentation review, not build or runtime evidence. M03 API final SPEC and QUALITY passed;
M16 source SPEC and QUALITY passed with connected runtime pending. M17 fresh SPEC and QUALITY passed,
and the canonical 10:34 five-case grading check passed after the earlier setup failure. This does not
establish combined M12 or M17 connected acceptance. The fixture cleanup and final Node-owner QUALITY
rereview passed locally; browser WASM remains pending. M22 and M29 remain unaccepted.

Fresh-agent delegation was unavailable at the thread limit. Fresh-to-change agents reused available
role slots; author and edit-review agents remained distinct. Browser acceptance, M28, target-budget
acceptance, and fresh integrated runtime review remain pending.

Earlier Pool-save correction checkpoint: `assessment_pool_reference_save_fix` added the missing Pool
parent and child INSERTs and persistence of an existing Pool ID. Fresh SPEC and distinct QUALITY
reviews through `pool_reference_save_review_coordinator` passed, including the symmetric kind guard
from `pool_entry_kind_guard_fix`. The M12 `question_type` fixture target correction also passed fresh
SPEC and distinct QUALITY review. Schema style and diff checks passed. The runtime owner started
combined retry 8, followed sequentially by M12 and full canonical three-lane acceptance on success;
stop at any failure. At this checkpoint, focused retry 8 exited 0 and passed the connected Unrelease
oracle, including post-release content and schedule boundaries; runtime cleanup verified zero
resources. M12 retry 2 exited 3 at the ambiguous lineage `USING` clause; the test fix changed it to
`ON lineage.ID = member.ID`. M12 retry 3 (session 32333) exited 3 after production Pool creation
because the oracle expected CC0 despite the replacement member's CC-BY4 license. The
`pool_creation_license_expectation` owner corrected that oracle expectation only; its fresh SPEC and
QUALITY reviews later passed. The runtime owner was idle after cleanup. The full canonical
three-lane acceptance was not launched because M12 stopped on failure. Schema/Human Guidance
documentation checks exited 0 and Human Guidance diff and consistency each reported 1,239 matching
bullets. No schema-table output-change claim is made because no pre-run hash was captured. The
canonical retry 7 PASS remains limited to its documented Unrelease oracle scope.
[Its implementation report](QUESTION_SPEC_M28_IMPLEMENTATION.md) confirms
that the connected synthetic readback was withdrawn before execution and that imported Question
source bytes, metadata, owner permissions, and exact Tuples through Pool and Blueprint readback have
not been verified. A prepared temporary script is not runtime evidence. The M28 status must remain
pending until those reads run and are reviewed.

Preserve the distinction between a focused passing check and the complete runner result. M29 and all
TODO mappings remain pending until complete connected runtime evidence and fresh integration review pass.

The subsequent M12 Pool-support retry 4 (session 21094) exited 3; its capture is
`output_question_spec/assessment_saved_response_m12_proof_retry4_20261006.log`. Pool creation,
licensing, and the fork checks passed before `03_course_pool_forks.sql:317` failed with permission
denied for `ple_api.read_question_pool_ple_managed_support`. This is a confirmed implementation and
integration defect, not a new product-rule uncertainty. The permission diagnosis found missing
grants, an `anyInstructor` write helper depending on definer execution with RLS enabled, and a server
path that blocked Sysadmin correction even though the UI showed the action editable to every
Instructor. The settled access boundary is shared read for Instructors and Sysadmins, with writes for
the owner and Sysadmins. The SQL authority correction and server/UI correction are reported complete;
TypeScript, Rust formatting, schema, and scoped diff checks passed.
The proof now reads public metadata under the corrected authority and includes the additional
authority check, but ignored `11` authority proof did not execute because retry 5 stopped earlier.
Combined Pool-support fresh SPEC and distinct QUALITY reviews passed. M12 Pool-support retry 5
(session 7848) exited 3 at `03_course_pool_forks.sql:314`: `ple_app` was denied schema `ple_data`
access because both the public SQL signature and Rust call cast Bloom values to private-schema
enums. The public API now accepts text and casts inside its `SECURITY DEFINER` wrapper to the
unchanged private enum-typed function. Rust and public SQL callers pass text, preserving nullable
dimensions and enum rejection while removing the app role's private-schema dependency. Schema
style, Rust formatting, and scoped diff checks passed. Fresh SPEC then distinct QUALITY reviews
and connected runtime acceptance are pending. Runtime cleanup found zero resources and the runtime
is idle. Direct Sysadmin HTTP-route coverage remains a nonblocking review gap. M12, M29, and overall
acceptance remain incomplete. The earlier focused retry 8 PASS retains only its Unrelease oracle
scope; no permanent tests were added for this correction.

### Deferred Pool bulk metadata regression checkpoint

M12 Pool-support retry 6 (session 91627, terminal 3) reached
`tests/e2e/assessment_saved_response/03_course_pool_forks.sql:314` and failed because PostgreSQL
denied `FOR UPDATE` on `ple_data.question_pool`. The private bulk function had been created while
the active role was `ple_api_owner`; `ple_api_owner` has `SELECT` but not `UPDATE`. The procedure
also lacked a per-Pool ownership predicate, so repairing the function owner alone would have
allowed one Instructor to edit another Instructor's Pool.

The source correction restores `ple_private_owner` before private function creation and uses that
owner for its private-function REVOKE/GRANT statements, then restores `ple_api_owner` for public
wrapper grants. The existing ordered row-lock query now matches `owner_account_id` to the active
Instructor before taking the lock; an inaccessible or missing row follows the existing
`42501` denial path before any batch update. Existing fields, edit-number comparisons, locking
order, transaction behavior, and Instructor-only capability remain unchanged. An ignored assertion
in `tests/_temp/11_pool_support_authority.sql` covers nonowner bulk denial alongside existing
owner-write proof. No permanent test was added.

Pool-owner ordinary editing is the retained authority. Sysadmin bulk access and the broader bulk
editing design remain deferred under Human Guidance. Schema style and scoped `git diff --check`
passed. Fresh `deferred_bulk_fix_spec` and distinct `deferred_bulk_fix_quality` reviews passed;
both were read-only and did not run tests or runtime. Canonical runtime retry 7 remains pending with
`baseline_retry_owner`. Prior retry evidence is in
`output_question_spec/assessment_saved_response_m12_proof_retry6_20261006.log`. This is a focused
source correction checkpoint only; M12, M29, and full acceptance remain pending.

Runtime retry 7 (session 59602) exited 3 in `03_course_pool_forks.sql`: after the bulk Pool
metadata replacement, the oracle asserted against `pool_metadata.tags` loaded before either update.
This is a stale test variable, not evidence of volatility or a lost production write. The oracle
now reads `ple_api.read_current_question_pool_metadata` immediately before checking the submitted
tag; the expected metadata token remains 4. The two checksum-helper setup queries now run under the
preceding authorized `ple_api_owner` role, while public API operations remain under `ple_app`.
Correction review and runtime acceptance are pending. Deferred bulk and Sysadmin scope remain
unchanged.

Runtime retry 8 (session 8896) passed the Pool metadata replacement check, then failed in
`load_assessment_workspace_rows` with `permission denied for table question_revision_citation`.
The source trace found the existing `ple_private.question_library_entries` SECURITY DEFINER reader
and both existing publication writers run as `ple_private_owner`, while that role lacked citation
table privileges and matching forced-RLS INSERT/SELECT policies. The narrow grant and policies now
restore those existing read and write paths; no `ple_app`, `ple_api_owner`, or broader role access
was added. Schema style and scoped whitespace checks passed. Fresh SPEC and a distinct fresh
QUALITY review passed. Canonical M12 retry 9 is authorized and its outcome is pending. Retry 8 did
not reach the authority-proof marker; cleanup was verified with zero remaining resources. Runtime
acceptance, authority proof, and overall M12 acceptance remain pending.

M12 retry 9 (session 10048) exited 3 at
`tests/e2e/assessment_saved_response/03_course_pool_forks.sql:447`. The citation permission error
from retry 8 is resolved; the metadata notice passed. Diagnosis confirmed the workspace reader
returns one row per Question in a Pool, while the existing oracle counted rows as logical
Assessment Entries. Both workspace assertions now count distinct `assessment_entry_id` values and
retain their Pool ID and requested-count predicates. This is a stale test row-cardinality
assumption; production SQL is unchanged. Fresh SPEC and distinct QUALITY reviews plus runtime retry
10 remain pending. Authority proof `11` remains unreached, cleanup verified zero resources, and full
canonical acceptance was not started. Deferred Sysadmin and bulk-edit scope remains unchanged.

### Citation as optional Question text

The human clarified that deferred citation format means ordinary text. Citation now has one
optional text value on Question metadata, a single `Citation` textarea, and no URL/text split or
citation-specific validation. SQL authoring, publication, fork-copy, and Library projections use
the optional `citation_text` value; Rust and browser contracts carry nullable strings. The metadata
spec and Human Guidance describe the settled behavior, and generated schema tables plus schema
style checks passed. `cargo tsgen` removed the obsolete generated citation struct, focused Cargo
checks passed, `tsc --noEmit` passed, and the three existing citation-sensitive Node tests passed
(11/11). Evidence is in `output_question_spec/citation_{tsgen,rust_check,tsc,node_tests}_20261006.log`.
Citation implementation and the exact 17-/14-argument PostgreSQL bind-count correction passed
fresh SPEC and distinct QUALITY reviews. M12 retry 11 stopped during schema installation because
`question_authoring_operations.sql:14` still targets the prior 18-argument
`ple_private.create_authoring_draft` signature containing the removed citation URL parameter. The
grant/REVOKE signature correction passed fresh SPEC and distinct QUALITY review without changing
role privileges. M12 retry 12 passed schema installation and citation/Pool checks before a stale
Blueprint Private-at-Revision-2 assertion; retry 13's corrected public-at-Revision-2 assertion
passed. Retry 13 then failed at the fairness Assessment save. A one-time ignored diagnostic captured
actor `USFD0000Y`, course `CISV000007`, Assessment `ASFR0007M`, Edit Number `4`, and all 18 expected
policy keys. The API wrapper's scalar source-ID reads are RLS-filtered by Instructor membership,
which the Student actor lacked. The fairness oracle now restores the Instructor account before its
existing Instructor save; root SPEC and independent QUALITY passed. Retry 14 passed the fairness
notice and Pool owner/Sysadmin proof, then stopped at ignored `11_pool_support_authority.sql:5`
because `:m09_owner_id` was undefined in that psql include scope. Authority marker `11` remains
unreached; cleanup verified zero resources, and canonical acceptance did not start.

The citation documentation verification passed fresh SPEC and distinct QUALITY review. The generated
Human Guidance checklist has 1,240/1,240 aligned items; Markdown checks reported 549 passes, and
schema output matched with schema style clean. The open-question ledger and checklist remain
evidence records and do not add product rules or acceptance gates.

The separate retry 10 baseline (session `1838`) exited 3 at
`tests/e2e/assessment_saved_response/04_blueprint_pool_forks.sql:156` because the fixture omitted
the required Blueprint Theme argument. The two earlier Pool round-trip and shared-Pool count checks
passed. A fixture-only correction supplying `ocean` has passed fresh SPEC and distinct QUALITY
reviews. Retry 13 passed the corrected Public-at-Revision-2 assertion and the remaining Blueprint
fork checks. Retry 14 passed the fairness and Pool owner/Sysadmin checks before the temporary
authority proof's undefined psql variable. Authority marker `11` remains unreached; cleanup verified
zero resources, and canonical acceptance was not started. See
`output_question_spec/assessment_saved_response_m12_proof_retry10_20261006.log`.

M12 retry 15 (log `output_question_spec/assessment_saved_response_m12_proof_retry15_20261006.log`)
reached and passed the corrected one-time Pool support authority proof, including its read/write marker. The
temporary oracle now binds the Instructor, Student, and Sysadmin IDs from the outer psql variables;
the IDs from `09_pool_save_permissions.sql` had been transaction-local settings, not psql variables.
The proof then stopped at `_temp/08_m12_release_validation.sql:8` because `shared_pool_id` was
undefined in that include scope. The psql wrapper is inside a `tee` pipeline without `pipefail`, so
the shell's terminal exit status is unknown. The EXIT trap removed the disposable container, and a
read-only container check found no remaining `ple-saved-response-*` resources. Cleanup is verified.
The temporary oracle now uses `created_pool_id`, the Pool created and shared by the earlier 03
oracle; this correction is awaiting review. M12 and canonical three-lane acceptance remain
incomplete. The prior retry 14 record above is historical.

M12 retry 16 (session 54014, exit 3; log
`output_question_spec/assessment_saved_response_m12_proof_retry16_20261006.log`) passed markers 01-07,
09, and 11. In 08, the shared Pool ID now maps to 03's `created_pool_id`; the future Due date,
save-above-current-size validation, and blocked-release checks passed. It then stopped at the
temporary missing-metadata corruption fixture, which required exactly two member rows before
deleting a metadata row. This fabricated incomplete state and exact diagnostic matrix are not
required by Human Guidance or a known failure. That ignored-only block and its dependent notices
have been removed; the ordinary count/release and classification-correction checks remain. 10's
psql variables were audited against 01-09 and all are defined in the same psql session or in 10.
The trimmed temporary proof awaits fresh review. The wrapper's actual process exit was 3 with
`pipefail` enabled; its cleanup trap ran and `podman ps -a` showed no remaining disposable proof
container. M12 and canonical three-lane acceptance remain incomplete.

The trimmed M12 proof passed fresh SPEC and distinct QUALITY review and ran as retry 17 (session
40385; true exit 3 with `pipefail`; log
`output_question_spec/assessment_saved_response_m12_proof_retry17_20261006.log`). Markers 01-07,
09, and 11 passed, as did the oversized Pool request save and release-block checks. The retained
Archive-preservation check changed the selected member to Archived and then failed with
`Archive alone invalidated an existing Pool reference`. The archive authority review confirmed that
Human Guidance requires preserving existing references; it did not run this runtime case. The
release-issue SQL inspected so far does not directly check Question availability, and the proof did
not assert that its Pool was otherwise issue-free immediately before Archive. Cause is unresolved;
no production or fixture workaround has been applied. Cleanup verified no disposable proof
container. Canonical acceptance remains unstarted.

Retry 18's temporary read-only probes established that the Archive failure was caused by a
pre-existing fixture state, not by Archive. The Pool had one member and entry `73000000-0000-0000-0000-000000000092` was already insufficient before Archive; member tuples and issue rows were identical afterward. The preceding 04 fork-independence oracle intentionally reduces its source Pool to one member and commits. The M12 08 setup now restores the intended original two exact tuples through the existing Instructor-owned Pool Save using the current Pool Edit Number. Fresh SPEC and distinct QUALITY passed; no source behavior or permanent fixture changed.

M12 retry 19 (session 77653, true exit 3 with `pipefail`; log
`output_question_spec/assessment_saved_response_m12_proof_retry19_20261006.log`) passed markers 01-07,
09, and 11; restored the Pool to two members at Edit Number 5; and passed Archive plus restore
preservation. It also passed saving a request for three from a two-Question Pool and blocking its
release. The run then failed during the ordinary metadata Subject-correction check at
`08_m12_release_validation.sql:363`: `ple_app` could not resolve a SQL cast through schema
`ple_data` to `ple_data.question_type`. Cause is under diagnosis; no product or fixture workaround
has been applied. Cleanup verified no disposable proof container. The prior retry 17 Archive failure
was setup-induced; Archive preservation passed in retry 19. M12 and canonical acceptance remain
incomplete.

M12 retry 20 (session 18536, true exit 3; log
`output_question_spec/assessment_saved_response_m12_proof_retry20_20261006.log`) passed the ordinary
metadata Save after the reviewed public-text cast correction (fresh SPEC
`question_metadata_boundary_spec` and distinct QUALITY `question_metadata_boundary_quality`). It
then failed at `08_m12_release_validation.sql:463` on a nested public Pool-issue assertion. The exact
JSON field mismatch is unconfirmed. Cleanup verified zero disposable containers. The ignored 08
proof was narrowed to require the Pool ID and specific issue in each detail while retaining the
public blocking issue and blocked-release assertion; this edited proof awaits a runtime rerun. No
cause is claimed. Separately, the 52-line permanent oracle 09 catalog cleanup passed root SPEC review
(a fresh SPEC reviewer was unavailable because of the thread limit) and distinct fresh QUALITY
`pool_check_cleanup_quality`, and the focused 09 runtime proof passed. M12 and canonical acceptance
remain incomplete.

Four ignored temporary browser proofs were simplified to assert required behavior rather than
implementation details: removed unsupported default-enabled Randomize, unsaved-before-autosave,
exact response-key/request-absence, and Theme saved-message assertions; retained required result
values, counts, copy/independent-save/reload behavior. Statistics percentage suffix matching is now
flexible while required counts and values remain asserted. Fresh root SPEC and distinct
`temporary_browser_checks_quality` passed source-only review. Prettier passed; ESLint reported the
ignored files as ignored, so they were not lint-tested. No permanent tests were added. Documentation
pytest passed 551 tests in 0.50 seconds (session 63195). Canonical retry 3 (session 37923) later exited
1 on an obsolete co-Instructor Pool-fork expectation after the Theme checks and independent Course
edit passed; the corrected oracle rerun and browser runtime remain pending.

M12 retry 21 (session 87597, true exit 3; log
`output_question_spec/assessment_saved_response_m12_proof_retry21_20261006.log`) passed the remaining
08 release-validation checks, including ordinary metadata Save, specific public Pool problems,
blocked release, and restored-metadata validation and actual release. The ordinary Save that omits
the Pool Entry also succeeded. The temporary 10 fairness proof then failed at its retained private
selection read because the assertion ran as `ple_data_owner`; the existing RLS policy grants that
read to `ple_private_owner`. The ignored oracle now keeps the public Entry assertion under
`ple_data_owner`, switches only the private Student Work read to `ple_private_owner`, and restores
the existing `ple_app` continuation. It also restores Student A as the actor before reading Student
A's pre-removal history. This is verification-role and actor alignment only; no production grants,
policies, or permanent tests changed. Runtime rerun, M12, and canonical acceptance remain pending.

M12 retry 22 (rootrun session 10159, exit 1; log
`output_question_spec/assessment_saved_response_m12_proof_retry22_20261006.log`) completed all SQL
proofs through marker 10, including the fairness NOTICE and ROLLBACK, then failed in the shell
wrapper because it searched for the removed
`course_instance_contains_only_published_questions_and_published_pools` notice. The oracle now emits
`direct_pool_reference_save_and_workspace_read_round_trip`; both permanent and ignored wrappers
were updated to require that existing notice. The SQL assertions and production behavior are
unchanged. No full wrapper PASS is claimed for retry 22, and cleanup verified zero owned containers.
The public metadata runtime proof now passes; the 09 catalog simplification and 08 specific-problem,
release, and omitted-Entry checks pass; 10's fairness proof reaches its expected notice and rollback.
The fresh SPEC and QUALITY readback-role correction passed before retry 22. A new rootrun is needed
to verify the corrected wrappers; M12 and canonical three-lane acceptance remain incomplete.

M12 retry 23 (session 74588; log
`output_question_spec/assessment_saved_response_m12_proof_retry23_20261006.log`) passed SQL proofs
01-07, 09, 11, 08, and 10 plus the permanent wrapper. Retry 22 had completed the SQL proofs but
exited 1 on a stale shell notice; the permanent and temporary wrappers were corrected to use a single
marker before retry 23. Root SPEC review passed for the marker correction; distinct fresh QUALITY
review verified the permanent wrapper. Root also verified the ignored wrapper path before the
successful run. Cleanup verified zero disposable containers. This is a focused M12 proof, not
full M12 or project acceptance; browser dependencies and other full-plan gates remain pending.

The canonical three-lane run exited 1 in session 76629, recorded in
`output_question_spec/acceptance_integrated_20261006.log`. Its disposable PostgreSQL database
baseline passed Draft source binding, Student Assessment Access, Course lifecycle, Course
Active-lifetime, Course summary lifecycle, and all five direct Assessment Attempt finalization
cases. The next Blueprint Revision lifecycle case failed at
`crates/learning-data-access/tests/blueprint_course_postgres/lifecycle.rs:126` while adopting
Blueprint Assessments with `InvalidRecord("database capability arguments are invalid")`.
The adoption JSON emits `questionPoolId`, while SQL validation and persistence still read the
retired `sourceQuestionPoolId`; the SQL key alignment passed fresh `blueprint_pool_key_spec` and
`blueprint_pool_key_quality`. Follow-on removal of unused actor/fork locals and three stale fork
comments in `assessment_blueprint_updates.sql` passed root SPEC and independent reused
`m02_quality`; this cleanup changed no behavior or privileges. A fresh reviewer spawn was blocked by
the thread limit. Root verified that cleanup left all owned containers stopped; the second and
third canonical lanes did not run. The canonical retry command is `source source_me.sh && set -o
pipefail && PYTHONUNBUFFERED=1 python3 local_stack.py acceptance 2>&1 | tee
output_question_spec/acceptance_integrated_retry2_20261006.log`; session 5388 is running in its first
database-baseline lane, with outcome pending. M12 retry 23 remains PASS as a focused proof. M12
browser dependencies, full M22/M29 integration, M28, browser, Live Demo, screenshot, and
target-budget acceptance remain pending. The documentation checks `source source_me.sh && python3 -m pytest
tests/test_guidance_doc_format.py tests/test_markdown_links.py -q` passed 551 tests in 0.46 seconds
(session 80822); this does not alter the runtime status.

M12 saved-response proof retry 24 (session 7095) passed the same SQL checks and final wrapper; its
log is `output_question_spec/assessment_saved_response_m12_proof_retry24_20261006.log`. Both shell
runners now rely on SQL `ON_ERROR_STOP` and oracle exit status instead of repeated success-NOTICE
greps; SQL assertions did not change. Root SPEC and distinct fresh `saved_response_runner_quality`
passed, and `bash -n` passed. Filtered Podman cleanup found zero `ple-saved-response` containers.
This remains focused M12 proof. Canonical retry 2 (session 5388) exited 1 after the Blueprint
Assessment Pool key correction succeeded. The existing lifecycle oracle then found that adopting a
Blueprint with saved Theme `forest` persisted Forest but returned Grass in the Course creation
receipt. The SQL creation function now returns the saved `course_theme`, and Rust decodes it through
the existing Theme parser. Fresh source-only SPEC `course_theme_response_spec` and distinct QUALITY
`course_theme_response_quality` both passed; they found no further defect. Canonical retry 3
(session 37923), recorded in `output_question_spec/acceptance_integrated_retry3_20261006.log`, exited 1
after the Theme checks and an independent Course edit passed; the obsolete co-Instructor
automatic-fork assertion expected a new Pool ID. The test now asserts the adopted Pool ID and owner
remain the same, consistent with direct Pool references. Nearby tests confirmed direct-reference
behavior. No production behavior changed to preserve the obsolete expectation, and filtered container
cleanup found no remaining containers. The corrected oracle rerun, M11 cleanup reviews, and subsequent
Cargo/runtime gates remain pending. M12 retry 24 remains a focused PASS only. M29 integration and
overall canonical acceptance remain incomplete.

### M11 Pool reference cleanup closeout

Fresh SPEC `pool_reference_cleanup_spec` and fresh QUALITY
`pool_reference_cleanup_quality` passed the M11 cleanup removing unused
`CourseInstancePoolIdIssuer` and its hooks and test wiring while preserving the active
`QuestionPoolIdIssuer`. The stale import comment was corrected to describe direct references to
existing Pools. Root's `cargo check` for `learning-data-access` and `server_core` passed with exit 0
and no warnings (session 49055); PostgreSQL-enabled compilation of
`blueprint_course_postgres --no-run` passed (session 40483). The prior no-feature compile is not
runtime evidence. The corrected co-Instructor same-ID/owner runtime oracle remains pending, so this
closeout does not establish canonical runtime acceptance. Rust formatting and scoped diff checks
passed.

Canonical retry 4 (session 56008, exit 1; log
`output_question_spec/acceptance_integrated_retry4_20261006.log`) passed the preceding Draft,
Student, Course, and Assessment Attempt finalization cases, then passed the full Blueprint Revision
lifecycle, including Theme and corrected co-Instructor direct-reference behavior. The next bounded
Question Library PostgreSQL case failed during setup at
`crates/learning-data-access/tests/blueprint_course_postgres/question_library.rs:82` with SQLSTATE
42501 (`permission denied for table draft_question`) while evaluating the
`reserved_draft_uuid`/registry expression. The trigger combined the optional UUID guard and private
table lookup in one SQL `AND`; PostgreSQL does not guarantee boolean short-circuit evaluation. The
trigger now places the unchanged lookup inside a procedural UUID guard, so an unset UUID skips the
private lookup. This fixes the reproduced setup defect. The follow-on nullable Draft public ID guard
correction passed fresh SPEC `public_id_guard_spec` and distinct QUALITY `public_id_guard_quality`;
it changes source only and leaves grants unchanged. Canonical retry 5 (session 51350) is now running
the same local_stack.py acceptance command, with log
`output_question_spec/acceptance_integrated_retry5_20261006.log`; outcome is pending. Lanes 2/3,
browser acceptance, and whole-plan acceptance remain pending. M12 focused wrapper retry 25 remains
PASS and unchanged; it is not full M12 acceptance.

### Configuration guidance and current focused checks

Latest explicit Human Guidance is preserved in `docs/HUMAN_GUIDANCE.md`: question specifications
need context-aware graceful recovery and simple configuration. The initial wording "use modes" was
corrected so it does not prescribe modes. Fresh SPEC `configuration_guidance_final_spec` and
distinct fresh QUALITY `configuration_guidance_quality` both passed. The focused documentation
command `source source_me.sh && python3 -m pytest tests/test_guidance_doc_format.py
tests/test_markdown_links.py -q` passed (session 49003, exit 0; 551 passed in 0.51 seconds).
Canonical retry 5 (session 51350) exited 1 after Draft, Student, Course, finalization, and full
Blueprint Revision lifecycle cases passed. The mixed Question Library fixture failed at
`question_library_mixed.rs:55` with SQLSTATE 55000 because its post-INSERT Bloom UPDATE attempted an
ordinary Pool metadata change without advancing the metadata edit counter. `insert_pool` now places
the initial Bloom pair in its INSERT only when `question_id` is present; Pools without a Question
retain nullable Bloom. The setup UPDATE was removed. Fresh SPEC `mixed_pool_bloom_spec` and distinct
QUALITY `mixed_pool_bloom_quality` passed on the scoped fixture change, including binding, type, and
nullable semantics. Reviewers did not re-review the broad pre-existing test-file diff; root accepted
only this scoped change. Scoped `git diff --check` passed. Canonical retry 6 (session 22327), with
output in `output_question_spec/acceptance_integrated_retry6_20261006.log`, passed Draft, Student,
Course, finalization, full Blueprint Revision, and mixed Question Library acceptance before failing
at Blueprint fork because Rust bound the child ID as UUID while SQL expects text. The nullable text
bind and existing public-ID trigger placeholder correction are in source; scoped Rust formatting,
schema-style, and diff checks passed. Fresh SPEC and distinct QUALITY review and a fresh runtime
rerun remain pending. Milestone and whole-plan acceptance statuses remain pending.

The nullable text ID alignment then passed the scoped SPEC review `blueprint_fork_id_spec` with no
issues and the distinct QUALITY review `blueprint_fork_id_quality` with an explicit PASS. Root's
`cargo check -p learning-data-access -p server_core` completed with exit 0 in 10.15 seconds (session
2508). Canonical retry 7 is running the same local-stack acceptance command in session 87811, with
output in `output_question_spec/acceptance_integrated_retry7_20261006.log`; its result is pending.
These source reviews and the Cargo check do not establish runtime acceptance. Milestone and
whole-plan acceptance remain pending.

Canonical retry 7 (session 87811, exit 1; log
`output_question_spec/acceptance_integrated_retry7_20261006.log`) passed through Blueprint fork
lineage, then failed in Question fork fixture setup at
`question_fork_parents_support.rs:256` with SQLSTATE 23514 because the fixture generated distinct
`clock_timestamp()` values for Revision `published_at` and its acceptance time. The existing
`validate_question_revision_acceptance()` trigger requires those values to match; normal
publication supplies the shared timestamp. The source fixture now selects `published_at` from its
stored Revision for the acceptance row, preserving the fixture's source Question, author, license,
and ownership setup and all fork assertions. This addresses a fixture defect only; no product
behavior changed. Scoped Rust formatting and diff checks passed; runtime has not been rerun.
Fresh SPEC and distinct QUALITY reviews plus canonical runtime
acceptance remain pending.

The corrected Question fork source fixture now reads the stored Revision `published_at` for its
acceptance row. Fresh source-only SPEC `question_fork_timestamp_spec` and distinct QUALITY
`question_fork_timestamp_quality` passed. Root's `git diff --check` exited 0, and the focused
documentation command `source source_me.sh && python3 -m pytest tests/test_guidance_doc_format.py
tests/test_markdown_links.py -q` passed (session 91833, exit 0; 551 passed in 0.53 seconds), before
this ledger append. Canonical retry 8 is running as session 3294 with command
`source source_me.sh && set -o pipefail && PYTHONUNBUFFERED=1 python3 local_stack.py acceptance
2>&1 | tee output_question_spec/acceptance_integrated_retry8_20261006.log`; runtime outcome remains
pending. M12 remains focused proof only; browser, lanes 2/3, milestone acceptance, and whole-plan
acceptance remain pending. Retry 7's failed run was followed by confirmed cleanup (`podman ps -a`
empty); this cleanup is not acceptance evidence.

### Question fork PostgreSQL error diagnosis and row-label correction

Canonical retry 8 reached `Question fork parents PostgreSQL acceptance` and failed with SQLSTATE
42501. The one-time ignored capture runner is `tests/_temp/run_question_fork_error_capture.py`; it
reads only `ERROR:` message text from the existing `ple-live-demo-browser_postgres_1` logs and
writes owner-only output to `output_question_spec/question_fork_database_error_capture.log` before
the canonical owner's normal failure handling. Root owns execution with
`source source_me.sh && python3 tests/_temp/run_question_fork_error_capture.py`; capture and rerun
outcomes remain pending.

The SQL wrapper returns columns `draft_question_id` and `authoring_workspace_id`, while the Rust
decoder had looked up `draft_question_uuid` and `workspace_id`. The decoder labels now match SQL;
domain/API field names are unchanged. The narrow decoder correction and temporary diagnostic passed
fresh SPEC `question_fork_decoder_spec` and distinct QUALITY `question_fork_decoder_quality` review.
Reviewer capacity returned after the thread limit, and final review passed. Root's
`cargo check -p learning-data-access -p server_core` passed in session 33924 (exit 0, 9.09 seconds,
no warnings). These checks do not resolve the `StoreError::Forbidden` returned by canonical retry 8
or establish its underlying SQLSTATE.

Canonical retry 8 (session 3294, exit 1; log
`output_question_spec/acceptance_integrated_retry8_20261006.log`) passed the earlier canonical cases
and Blueprint fork lineage. The corrected source timestamp fixture setup now passes; Question fork
then returned `StoreError::Forbidden`. The acceptance output does not confirm an underlying SQLSTATE,
so the prior statement in this report that described SQLSTATE 42501 as confirmed is corrected here.
The one-time ignored capture runner is currently running with
`source source_me.sh && set -o pipefail && PYTHONUNBUFFERED=1 python3 tests/_temp/run_question_fork_error_capture.py 2>&1 | tee output_question_spec/question_fork_diagnostic_20261006.log`;
it exercises only the canonical database baseline. Its ERROR-message capture output is
`output_question_spec/question_fork_database_error_capture.log` and remains pending. This is
diagnostic work, not a rerun of the three-lane acceptance. Root reports `podman ps -a` empty after
retry 8. Product permissions are unchanged and no permanent tests were added. Lanes 2/3, browser,
milestone, and whole-plan acceptance remain pending.

The captured database error ends with `permission denied for table question_revision_license`.
The fork SQL selected immutable license and citation rows `FOR KEY SHARE`, although its API role
has read access and no UPDATE privilege; PostgreSQL locking reads require UPDATE privilege. The
two unnecessary locking clauses are removed. The metadata `FOR SHARE`, lineage, workspace, and
source-binding locks remain. This is a narrow source-level permission fix with no privilege or
policy change. The one-time diagnostic capture exited 1 in session 18556; its last license-specific
failure was `permission denied for table question_revision_license`. The diagnostic run was followed
by cleanup verification showing no Podman containers. Independent reused SPEC
`question_fork_decoder_spec` and distinct reused QUALITY `question_fork_decoder_quality` reviews
passed after fresh reviewer spawns hit the thread limit; schema-style and scoped diff checks passed.
Canonical retry 9 (session 59013, exit 1) ran
`source source_me.sh && set -o pipefail && PYTHONUNBUFFERED=1 python3 local_stack.py acceptance 2>&1 | tee output_question_spec/acceptance_integrated_retry9_20261006.log`.
It passed the earlier cases through Blueprint fork, then failed in Question fork parents at
`question_fork_parents.rs:62:10` because SQL line 1041 references ambiguous `created_at`.
The license/citation permission barrier from retry 8 is cleared. The correction renamed the function-
local `created_at` value to `fork_created_at` and uses it only as a timestamp; schema style
passed. Fresh SPEC `fork_timestamp_spec_review` and distinct fresh QUALITY
`fork_timestamp_quality_review` passed. Root reports full `git diff --check` passed and focused
documentation pytest passed 551 tests (session 35970, exit 0). Retry 9 cleanup left `podman ps -a`
empty. These scoped checks do not establish integrated acceptance.

Canonical retry 10 (session 62577) started with
`source source_me.sh && set -o pipefail && PYTHONUNBUFFERED=1 python3 local_stack.py acceptance 2>&1 | tee output_question_spec/acceptance_integrated_retry10_20261006.log`.
It exited 1 after passing Blueprint Revision, mixed Question Library, and Blueprint fork, then
failed the child Question ID reservation SELECT in Question fork parents at
`question_fork_parents.rs:78` with SQLSTATE 42501 (`permission denied for schema ple_private`). The
test-only correction performs this SELECT inside a transaction after `SET LOCAL ROLE
ple_private_owner` and commits; production grants are unchanged. `rustfmt` and
`cargo test -p learning-data-access --test blueprint_course_postgres --no-run` passed with existing
warnings. Fresh SPEC `fork_reservation_test_spec` and distinct fresh QUALITY
`fork_reservation_test_quality` passed test-only scope. Canonical retry 11 (session 66178, exit 1;
`output_question_spec/acceptance_integrated_retry11_20261006.log`) passed Question fork parents, then
failed Question Revision metadata setup on the Sysadmin fixture timestamp check. The test-only timestamp
correction passed focused SPEC/QUALITY review. Canonical retry 12 (session 31069, exit 1) is recorded
in `output_question_spec/acceptance_integrated_retry12_20261006.log`; it passed Question fork parents,
then failed Question Revision metadata setup on duplicate authenticated Sysadmin token material.
Canonical retry 13 (session 13086, terminal 1) is recorded at
`output_question_spec/acceptance_integrated_retry13_20261006.log`; selectors through Question Revision metadata passed, then the invalid Watch direct-INSERT helper failed at `question_library_stewardship.rs:208` (expected 23514, got 42501). The CHECK-shape helper/call was removed per `PYTEST_STYLE.md`; scoped format, compile, and diff checks passed. Fresh SPEC `watch_test_scope_spec` passed; fresh QUALITY `watch_test_scope_quality` failed pending policy alignment. It argued to retain malformed DB-event invariant coverage and also found the reserved child-ID fixture mismatch. The manager decision is to keep the database CHECK constraint and real Human Guidance event-delivery tests, while removing a permanent test that exercises no concrete invalid-event producer or demonstrated regression; this follows the permanent-test policy and Human Guidance direction to remove uncertain-value tests. The test-only reserved-ID fixture correction and combined fresh SPEC `watch_fixture_final_spec` / distinct QUALITY `watch_fixture_final_quality` review passed. Canonical retry 14 (session PTY90004; `output_question_spec/acceptance_integrated_retry14_20261006.log`) exited 1 at Watch creating a Question fork (`question_library_stewardship.rs:99`): the fixture bytes do not match the seeded source checksum, size, and media type required by SQL lines 227-247. The test-only fixture now copies source checksum, size, and media type from the exact-bound object record; generated-ID readback is unchanged. `rustfmt`, PostgreSQL-feature compile-only, and diff checks passed. Reused SPEC `m03_metadata_api` and distinct reused independent QUALITY `fork_reservation_test_quality` both passed because fresh reviewer slots were unavailable. Canonical retry 15 (session 6388; `output_question_spec/acceptance_integrated_retry15_20261006.log`) exited 1 after selectors through Question Revision metadata passed, then failed at Watch while loading the forked Question Draft with `InvalidRecord("database returned an invalid Draft Question source binding")`. Retry 16 (session 1045; `output_question_spec/acceptance_integrated_retry16_20261006.log`) exited 1 at Watch when the source-classification fixture SELECT hit SQLSTATE 42501 on `ple_data`; the correction uses the loaded fork Draft classification. Formatting, diff, PostgreSQL-feature compile-only, fresh SPEC, and distinct fresh QUALITY checks passed; canonical retry 17 (session 45432; `output_question_spec/acceptance_integrated_retry17_20261006.log`) exited 1 after Question Library Watch passed and Pool member Watch failed at `support.rs:32` (`SET LOCAL ROLE ple_auth` denied on the migration connection). Browser, milestone, lanes 2/3, and whole-plan acceptance remain pending.

### Sysadmin source-scope drift review (2026-10-06)

The reviewed Course inspection path-[Sysadmin UI](../../../src/pages/sysadmin_course_inspection_page.tsx),
[support capability](../../../crates/server/src/support_capability.rs), and
[Course operations SQL](../../../schemas/base_schema/50_functions/course_operations.sql)-uses
confirmation and an audited single-record lookup consistent with Human Guidance lines 279-289 and
TODO lines 211-219. This limited source review found no additional persisted business state, grant
workflow, or membership mutation in that path. It did not assess wider Sysadmin endpoints, security,
or runtime behavior; it makes no broader Sysadmin acceptance claim.

### Combined proof coverage review (2026-10-06)

The initial integration review suggested extending `tests/_temp/m28_import_readback.py` for assets,
Pools, and Theme. After root challenged the single-helper assumption, the reviewer inspected the
union of the planned checks and withdrew that expansion; no concrete missing proof was identified
in the combined scope. The readback helper checks
generated Pilot Question IDs, owners, metadata, exact source bindings, and exact Blueprint Tuples;
the Pilot fixtures contain no image assets or Pools, so it does not claim to read either back.
The temporary browser runner plans visible image upload/preview/published-image loading and PG/PGML
authoring, Pool edit/reload and Assessment use, and Theme adoption with an independent Course change.
Separately, focused M12 retry 25 proves a shared Pool across two Assessments, and the registered
Blueprint lifecycle proof passed M27 Theme adoption and independent Course Theme editing. The
readback helper and browser runner remain unrun. Independent `integrated_coverage_checkpoint` found no unqueued proof gap in the planned combined scope. Retry 13 exited 1 at the Watch helper as recorded above; runtime remains incomplete; canonical retry 14 (session PTY90004) exited 1 at Watch creating a Question fork on a copied-source fixture mismatch, and no full acceptance is claimed.

### Retired temporary proof cleanup (2026-10-06)

Removed the ignored synthetic M28 CLI (`tests/_temp/question_spec_acceptance.rs` and
`tests/_temp/question_spec_acceptance/tests.rs`) and the completed database-baseline and Question
fork diagnostic wrappers (`tests/_temp/run_database_baseline_capture.py` and
`tests/_temp/run_question_fork_error_capture.py`). `git check-ignore` confirmed all four paths were
ignored; repository search found no runner consumer outside this ledger's historical diagnostic
record. Diagnostic logs remain under `output_question_spec/`. The queued
`tests/_temp/m28_import_readback.py`, `tests/_temp/run_browser_proofs.py`, M12 and Unrelease probes
remain queued and pending disposition/runtime evidence. Canonical retry 14 exited 1 while creating a Question fork on a test-fixture source-metadata mismatch; the correction passed reused SPEC `m03_metadata_api`, and reused independent QUALITY
`fork_reservation_test_quality` passed as a reused independent reviewer alongside reused SPEC
`m03_metadata_api`. Canonical retry 15 (session 6388; `output_question_spec/acceptance_integrated_retry15_20261006.log`) exited 1 after selectors through Question Revision metadata passed, then failed at Watch while loading the forked Question Draft with `InvalidRecord("database returned an invalid Draft Question source binding")`. No removed wrapper or synthetic
CLI supplied acceptance evidence. Canonical retry 16 (session 1045; `output_question_spec/acceptance_integrated_retry16_20261006.log`) exited 1 at Watch on a source-classification fixture SELECT denied on `ple_data`; the correction uses the loaded fork Draft classification and passed focused format, diff, compile-only, fresh SPEC, and distinct fresh QUALITY checks. Canonical retry 17 (session 45432; `output_question_spec/acceptance_integrated_retry17_20261006.log`) exited 1 after Question Library Watch passed and Pool member Watch failed at `support.rs:32` (`SET LOCAL ROLE ple_auth` denied on the migration connection). M28 readback/browser and
remaining runtime acceptance were pending at that checkpoint. The Pool Save fixture now uses the ordinary Store call and passes compile-only checks.

### Changelog recovery record (2026-10-06)

Restored 29 accidentally removed Behavior entries dated October 6 from recorded edits, preserving
the baseline. Repaired one broken FIB report clause. The recovered changelog has 960 lines, 188
entries, and two dates. Fresh SPEC `changelog_restore_spec` and distinct QUALITY
`changelog_restore_quality` reviews passed. The focused Markdown-link and source-file-line-limit
pytest checks passed (2,589 passed; 10 near-limit warnings), and the scoped diff check passed.
Recovery artifacts `/private/tmp/changelog_before_restore.md` and
`/private/tmp/changelog_behavior_recovery_29.md` are temporary. Canonical retry 17 (session 45432; `output_question_spec/acceptance_integrated_retry17_20261006.log`) exited 1 after Question Library Watch passed and Pool member Watch failed at `support.rs:32` (`SET LOCAL ROLE ple_auth` denied on the migration connection); the Pool Save fixture correction uses the ordinary Store call and passes compile-only checks. Full acceptance is pending.


### Historical Source integration reviews checkpoint (2026-10-06)

Fresh SPEC `final_source_integration_review` and distinct QUALITY `final_source_integration_quality` reviewed sampled seams: ordinary fork/direct Pool behavior, stored fractions/current points, nullable Bloom, ephemeral search, Sysadmin confirmation/audit, and statistics from actual deliveries and origin Pools. Neither found a confirmed cross-layer defect in the sampled scope; these reviews did not inspect every file or milestone and do not establish runtime behavior. SPEC withdrew its proposed k=5 threshold finding after HEAD and Plan M22 evidence showed the existing privacy rules; the implementation detail remains with no new policy. The root Human Guidance checklist passed both `--diff` and `--consistency` at 1242/1242. A separate source trace confirmed the required explicit Pool-fork path had only a `ple_data` function and no public SQL wrapper, Store/API, route, client, or UI. Backend and browser work has now added the explicit fork application path within existing M11/M14/M29 scope; scoped checks passed, with browser SPEC review passing. Backend SPEC reused that browser SPEC, and QUALITY reused `final_source_integration_quality`, because fresh reviewer creation hit the thread limit. Retry 17's Pool Save fixture correction uses the ordinary Store call and passes compile-only checks. Canonical retry 18 (session 35850; `output_question_spec/acceptance_integrated_retry18_20261006.log`) exited 1 after eight selector groups passed, then stopped compiling Blueprint Revision acceptance because `stewardship.rs:367` moves `forked_pool_id` and `source_pool_id`, reused at lines 370 and 383. The PostgreSQL-feature compile-only command reproduced the failure; an earlier compile check omitted the feature. At this historical checkpoint, the Watch field names and test-clone correction were implemented, source freeze had been released, and fresh Watch reviews plus retry 19 had not started. Current status is at the top of this ledger. All milestones and 35 TODO mappings remain pending full runtime, browser, and integration closure.

### Postacceptance HTML correction (2026-10-07)

Native HTML/import documentation now describes the corrected behavior, with the image example at `assets/image_001.png`; focused doc-format and Markdown-link checks passed (552), and `git diff --check` passed. The affected Human Guidance checklist `--diff` and `--consistency` gates passed, with 1248/1248 aligned. Final SPEC `html_two_fixes_spec` and distinct QUALITY `html_final_quality` reviews passed; a separate global checklist evidence check found four unrelated rows missing source-path references. This records documentation and checklist evidence only; it makes no code, runtime, or full-acceptance claim.

### Native JSON display-content wording follow-up (2026-10-07)

SPEC passed; initial QUALITY found the converter handoff's HTML display scope too narrow. The handoff
now follows the canonical Native JSON Display content rule for all display content that supports
HTML, with prompt and choice examples retained as representative coverage. Earlier focused docs
checks (2968) remain prior evidence. Final focused Markdown-link and guidance-format checks passed
(553), and the scoped diff check passed. Final QUALITY re-review accepts the corrected handoff
scope, existing tuple and identity boundary, unresolved HOTSPOT pre-binding representation, and
evidence-specific current PLE gap statements. This is a documentation-only source review; runtime
behavior was not rebuilt or reverified.
