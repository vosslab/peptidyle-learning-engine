## Question specifications

- [x] Questions are subject agnostic. Properly classified Published Questions from all subjects belong in
  the same Question Library.
  - Evidence (source): `crates/server/src/question_library/paging.rs` `QuestionSearchFilter` supplies the shared Library query filter without a subject partition.
- [ ] Questions are strictly and deterministically automated; grading does not require an **Instructor**.
  - Mismatch: needs runtime grading evidence for every supported backend.
- [x] Questions have one canonical title. Compact interfaces may truncate that title.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question_metadata` stores one lineage-level title.
- [x] Every Question stored by PLE has its own internal Question record.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question` owns the internal Question record.
- [ ] Answer-choice randomization belongs to the Question.
  - Mismatch: native answer-choice randomization ownership has not been verified.
- [ ] PLE-native Questions control their own answer-choice randomization.
  - Mismatch: no native answer-choice randomization implementation was found.

### Draft Question specifications

- [x] Draft Questions are private working content.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `ple_private.draft_question` stores draft state in the private schema.
- [ ] Draft Questions are not part of the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `published_question_metadata` queries only Published Question metadata; Draft working state is stored separately in `schemas/base_schema/50_functions/question_authoring_state.sql` `authoring_draft`.
  - Verification pending: re-evaluate the current Library search/Pool projections and publication boundary to establish explicit Draft exclusion across all Library paths.
- [x] Draft Questions use current state rather than immutable Revisions.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `draft_question_edit_number` is current-state concurrency data, separate from `question_revision`.
- [x] Saving a Draft Question replaces its previous working state.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `save_authoring_draft` replaces the current draft aggregate values.
- [x] **Instructors** may delete Draft Questions they no longer need.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `delete_draft_question` resolves only the current Instructor-owned Draft, locks and compares its Edit Number, then deletes that private aggregate without considering the separate Published Question lineage.
  - Evidence (source): `crates/learning-data-access/src/postgres/authoring.rs` `delete_authoring_draft` carries the SQL compare-and-swap through the authenticated Store.
  - Evidence (source): `crates/server/src/authoring.rs` `delete_draft` requires the parsed `If-Match` Edit Number and maps a concurrent change to 412; `src/pages/question_drafts_page.tsx` `QuestionDraftsPage` supplies explicit Keep/Delete confirmation.
  - Evidence (runtime): `crates/server/src/authoring.rs` `delete_draft` passed accepted isolated PostgreSQL 17/MinIO actual-server and focused browser proof: cancel, confirm, and list reload; valid-current-ETag collaborator, unrelated Instructor, Student, Sysadmin, and anonymous 404 denials while owner source/Edit Number remained unchanged; 428 missing, 400 malformed, and 412 stale preconditions; preserved parsed Published Question lineage and Revision JSON after a published-origin Draft deletion; and 404 repeat DELETE/PUT. Artifact: `/private/tmp/ple-draft-delete-artifacts.km9ybM`.
- N/A PLE may clean up abandoned Draft Questions after an appropriate warning and recovery period.
  - Reason: Automated abandoned-Draft cleanup is an explicitly optional future capability; HG sets no clock or durations.
- [ ] A Draft Question must pass Question Publication Validation before becoming a Published Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `validate_question_publication` guards publication.
  - Verification pending: audit ordinary Draft publication, not only fork publication, against current validation and required Library metadata.
- [ ] Question Publication Validation requires Discipline, Subject, and all other required Question
  Library metadata before publication.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.

### Question formats and type specifications

- [x] PLE flat-question JSON is the canonical machine format for simple static Questions.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` validates the PLE JSON source form.
- [x] QTI is for import, export, and archival interchange rather than the internal source model.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `workspace_import` treats `qti` as an import format, not a source binding.
- [x] MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT Question Types should be supported.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` CHECK lists all eight types.
  - Evidence (runtime): `tests/playwright/screenshot_corpus/scenarios_student_types.ts` `captureTypes` supplied current authorized Student delivery of each eight released native types at laptop and phone widths (18 unanswered captures including WeBWorK); exact issued Question Revision membership and permitted-response privacy checks passed. This is private presentation coverage, not an eight-type interaction matrix. Receipt: `/private/tmp/ple-resumed-types-20260916.md`.
- [x] Question Type is immutable author-declared educational metadata on a Published Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision_is_immutable` protects `question_type` on a revision.
- [x] PLE uses Question Type for search, filtering, labeling, and presentation.
  - Evidence (source): `src/api/question_library_repository.ts` `questionSearchRequest` sends the selected Question Type as the Library search filter; `src/pages/library_page.tsx` `questionTypeLabel` supplies learner-facing type labels and the Question Type selector presents the type facets.
- [x] Question Type comes from the author rather than inference from backend controls.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `draft_question_source_binding` records authoring input independent of backend.
- [x] Question importers are transient translators from external formats into PLE-managed Question representations.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `workspace_import` stages external-format imports before committed PLE state.

### Native PLE JSON Question specifications

- [x] The native PLE JSON Question format is private, unversioned, and unpublished.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` accepts the unversioned internal source shape.
- [x] Stored native JSON Questions may be upgraded together when the internal format changes.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` is the single internal reader for stored PLE JSON.
- [x] The native PLE JSON Question format is a strictly validated internal source shape without an external API.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` validates the internal source document.
- [x] Native JSON Questions are static, not algorithmic nor random, and receive no random seed.
  - Evidence (source): `crates/question_model/src/generation.rs` `QuestionReproduction` distinguishes static source reproduction from the inseparable seeded generator pair; `crates/adapters/ple/src/lib/question_json_source.rs` `presentation` issues native PLE JSON with `QuestionReproduction::Static`.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempts.sql` `validate_issued_question_reproduction` rejects a seed for a `ple` source and requires one for renderer-backed sources.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_attempts.sql` `validate_issued_question_reproduction` passed in `/private/tmp/ple-native-seed-proof.sh --isolated --native-seed-http` against PostgreSQL 17: shuffled-position-2 native seed/hash were null, real WeBWorK retained numeric seed/64-character hash privately, public start/read/save/resume/restored payloads omitted both fields, resume retained the same issued Questions and saved native response, and invalid native seed insertion failed. Artifact: `/private/tmp/ple-native-seed-artifacts.KfY7Op`.
- [x] Native PLE JSON supports MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonResponse` defines all eight native types.
  - Evidence (runtime): `tests/playwright/screenshot_corpus/scenarios_student_types.ts` `captureTypes` supplied current authorized Student delivery of each released native type at laptop and phone widths with exact published Revision checks. The 16 native captures establish presentation only; response interaction, save/reload, and grading remain separately scoped per type. Receipt: `/private/tmp/ple-resumed-types-20260916.md`.
- [x] External URLs used by native JSON Questions are explicitly recorded and reviewable.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` records the author-declared `externalResources` inventory as source metadata only, without fetching or browser permission; `validate_external_resources` bounds and de-duplicates recorded URLs.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `validate_external_resource_url` accepts only bounded, printable, absolute HTTPS URLs without user information.
  - Decision: A one-time parser proof accepted all five resource kinds and legacy omission, while rejecting invalid and duplicate URLs; it is temporary evidence and will be removed rather than retained as a permanent implementation-inventory test.
- [x] Recorded external URLs include links, images, scripts, stylesheets, and other resources.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonExternalResourceKind` is the closed Link, Image, Script, Stylesheet, and Other category set for every `externalResources` entry.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonExternalResource` binds each recorded URL to exactly one reviewed category under `deny_unknown_fields` parsing.

#### Native Question response presentation

- [x] Native MATCH Questions should present prompts with a shared choice bank on laptop and desktop
  screens. Display the full set of choices once alongside the prompts.
  - Evidence (source): `src/components/question_response_controls/matching.tsx` `MatchingResponse` has one bank, prompt slots, assignment/replacement/Clear controls, same-bank drag, and filtered partial/reset serialization.
  - Evidence (runtime): `src/components/question_response_controls/matching.tsx` `MatchingResponse`, supplied `/private/tmp/ple-matching-saved-1280.png` and `/private/tmp/ple-attempt-compact-1280.png` show all four bank choices once beside the prompt slots. Independent source review `/private/tmp/ple-demo-ui-source-review.md` and bounded acceptance `/private/tmp/ple-ui-bounded-acceptance.md` support this shared-bank laptop/desktop presentation only; keyboard changing/clearing and whole-Attempt grading are separate requirements.
- [x] MATCH Questions should support drag-and-drop and an equally capable keyboard-only method for
  assigning, changing, and clearing matches.
  - Evidence (source): `src/components/question_response_controls/matching.tsx` `MatchingResponse` has one bank, prompt slots, assignment/replacement/Clear controls, same-bank drag, and filtered partial/reset serialization.
  - Evidence (runtime): `src/components/question_response_controls/matching.tsx` `MatchingResponse`, supplied parent 2026-09-16 actual current-demo Avery R-4 laptop proof (session 87294, exit 0): normal Tab traversal without programmatic focus plus Space/Enter cleared the first two saved matches, selected bank choices, swapped both assignments, then cleared/reassigned the original choices. All four original choice strings were restored exactly, and Tab/Enter Save was accepted. The existing native mouse drag and accepted Save receipt is independently accepted in `/private/tmp/ple-ui-bounded-acceptance.md`; fresh keyboard evidence is recorded in `/private/tmp/ple-latest-hg-checklist-reconciliation.md`. This closes assigning/changing/clearing parity only, not adapted grading or full pointer/touch bank reachability.
- [ ] Question response layouts may adapt to available screen space while preserving the same content,
  response meaning, and grading behavior. Narrow layouts may repeat choices when that improves use.
  - Evidence (source): `src/components/question_response_controls/matching.tsx` `MatchingResponse` has one bank, prompt slots, assignment/replacement/Clear controls, same-bank drag, and filtered partial/reset serialization.
  - Verification pending: bounded partial/reset and exact Save/reload receipts retain response identity; adapted narrow response layouts still need proof of preserved content, response meaning, and grading behavior. No particular choice-repetition design is imposed.
- [ ] MATCH Questions should make each prompt's assigned choice easy to recognize and keep the choice
  bank reachable while Students assign, change, and clear matches using keyboard, pointer, or touch.
  - Evidence (source): `src/components/question_response_controls/matching.tsx` `MatchingResponse` has one bank, prompt slots, assignment/replacement/Clear controls, same-bank drag, and filtered partial/reset serialization.
  - Verification pending: supplied desktop captures show assigned choice text, and bounded receipts show keyboard/click assignment and mouse drag. Bank reachability throughout changing/clearing with keyboard, pointer, and touch remains unobserved. Grading is not an acceptance prerequisite for this interaction-reachability row.

#### Native PLE JSON Questions and JavaScript

- [ ] Native JSON Questions may contain author-supplied JavaScript, including chemistry content using RDKit.
  - Mismatch: no author JavaScript field or RDKit integration was found in the native JSON schema.
- [ ] Author-supplied JavaScript may provide client-side rendering or interaction without access to a random seed.
  - Mismatch: author-supplied JavaScript is not implemented.
- [ ] Author-supplied JavaScript runs in an isolated browser environment.
  - Mismatch: no isolated author-script runtime was found.
- [ ] Author-supplied JavaScript is treated as untrusted content.
  - Mismatch: author-supplied JavaScript is not implemented.
- [ ] Author-supplied JavaScript is isolated from PLE application state, credentials, and privileged browser context.
  - Mismatch: no sandbox boundary for author JavaScript was found.
- [ ] Author-supplied JavaScript is limited to client-side rendering and interaction.
  - Mismatch: author-supplied JavaScript is not implemented.
- [ ] Author-supplied JavaScript operates independently of PLE application APIs and privileged state.
  - Mismatch: author-supplied JavaScript is not implemented.
- [x] Native interactive Question Types such as HOTSPOT use PLE-owned interaction code.
  - Evidence (source): `src/components/question_response_controls/question_response_control.tsx` `QuestionResponseControl` dispatches a delivered `hotspot` format to `HotspotResponse`; `src/components/question_response_controls/hotspot.tsx` `HotspotResponse` owns the image overlay, labeled native region controls, response serialization, and Save handoff.
  - Evidence (runtime): `tests/playwright/screenshot_corpus/hotspot_workflow.ts` `exerciseHotspot` passed unchanged for Avery's pointer input and Jack's keyboard Space input: each selected the PLE-owned region, saved, reloaded the exact issued Question ID and Revision with the selection intact, submitted the whole Attempt, and received `Marked correct.` from server grading. Receipt: `/private/tmp/ple-hotspot-connected-interaction-20260916.md`.
- [ ] HOTSPOT content uses supported static assets such as images and SVG.
  - Evidence (source): `schemas/base_schema/50_functions/draft_question_assets.sql` `validate_draft_question_asset` accepts only bounded PNG, JPEG, and WebP raster evidence; `src/components/question_response_controls/hotspot.tsx` `HotspotResponse` renders the revision-pinned image surface.
  - Verification pending: Connected canonical proof now covers a prepared published raster image, its loaded Student surface, and selection. SVG remains unsupported because the accepted media types exclude `image/svg+xml`, so the images-and-SVG requirement remains open.
- [ ] Grading and correctness decisions remain server-owned and independent of author-supplied JavaScript.
  - Mismatch: author JavaScript is absent; no runtime proof covers this interaction boundary.
- [ ] External JavaScript dependencies and CDN domains are explicitly recorded and reviewable.
  - Mismatch: no external JavaScript dependency inventory was found.
- [ ] Approved external dependencies may initially load from recorded CDN sources.
  - Mismatch: no approval or recorded-CDN mechanism was found.
- [ ] Supported external dependencies should eventually become PLE-owned and served locally.
  - Mismatch: no dependency-localization workflow was found.

### Question Backend specifications

#### Supported Question Backends

- [x] WeBWorK is a PLE-managed Question Backend.
  - Evidence (source): `crates/adapters/webwork/src/lib.rs` `WebworkAdapter` is the current PLE-managed WeBWorK integration boundary.
- [x] The initial primary Question Backends are PLE-native JSON and WeBWorK.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_presentation.sql` `backend IN ('ple', 'webwork')` is the delivered presentation boundary.
- N/A iMathAS and H5P are desired secondary Question Backends governed by Deferred product behavior.
  - Reason: Human Guidance explicitly defers both Backends, so they are desired product behavior rather than current implementation requirements. Current production Backends are PLE and WeBWorK.
- [x] PLE-native Questions use the PLE Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `question_source_binding_fields_are_valid` maps `ple` to `pleQuestionJson`.
- [ ] WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
  - Mismatch: the isolated opaque adapter proves renderer documents, ordered pairs, score, partial credit, and stateless state. Connected live-ownership proof remains required; PLE is not required to capture historic renderer feedback.
- N/A H5P owns its runtime, interactions, state, and scoring.
  - Reason: H5P is desired but explicitly deferred and is not a current implementation requirement.
- N/A iMathAS owns its rendering and evaluation.
  - Reason: iMathAS is desired but explicitly deferred and is not a current implementation requirement.

#### Question Backend responsibilities

- [x] PLE owns and stores the Question representation used for each Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `question_revision_source_binding` stores backend representations.
- N/A Imported backend source may be transformed into the form PLE stores and manages.
  - Reason: Optional transformation does not require a current backend-import behavior.
- [x] PLE preserves the information needed to reproduce the Question through its backend.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `question_revision_source_binding` retains backend selectors and source checksum.
- [x] PLE-managed Question representations participate in Question revision history.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `question_revision_source_binding` keys source bindings to immutable revisions.
- [ ] Question Backends own rendering, interaction, response, grading, feedback, and backend-specific state.
  - Mismatch: needs backend render-and-grade runtime evidence; current native and WeBWorK boundaries are not proof for all supported backends.
- [ ] PLE owns authorization, Question ID, revisions, persistence, lifecycle, and stored outcomes.
  - Mismatch: schema ownership is evidence for records, but no connected runtime or test evidence proves the complete authorization and stored-outcome boundary.
- [ ] PLE uses the same basic interface for every Question Backend, each backend handles its own internal details.
  - Mismatch: `crates/question_model/src/question_library.rs` `QuestionBackend` is only an enum discriminator. Issuance and finalization branch separately on backend in `crates/server/src/assignment_delivery.rs` `issue_new_presentations` and `crates/server/src/assignment_delivery/direct_finalization.rs` `evaluate_one`; no common adapter interface covers every backend.
- [ ] Each Question Backend adapter retains its backend-specific interaction knowledge.
  - Verification pending: `crates/adapters/webwork/src/lib.rs` `WebworkAdapter` establishes the current production external-backend boundary. Current PLE/WeBWorK connected acceptance remains required; deferred iMathAS/H5P behavior does not block this row.
- [ ] Question Backends may support more complex interactions without requiring PLE to implement those interactions.
  - Verification pending: `crates/adapters/webwork/src/lib.rs` `WebworkAdapter` keeps current production WeBWorK interaction details outside PLE. Connected current-backend acceptance of this broad capability remains pending; deferred iMathAS/H5P behavior is not a blocker.

#### Question Backend grading and feedback

- [x] Question Backend feedback is transient unless the backend provides a robust way for PLE to preserve it.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` projects recorded native PLE feedback from the exact retained response and source, while the WeBWorK branch does not reconstruct or persist transient renderer feedback.
  - Evidence (runtime): the C910 isolated actual-HTTP proof exercised `crates/server/src/assessment_delivery/history.rs` `student_history`, stopping the renderer after issuance and then submitting and reading exact WeBWorK Revision history without a backend-feedback field.
- [x] PLE does not extract or reconstruct transient feedback from Question Backend source or output.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` invokes recorded teaching-content projection only for the native PLE source variant; the WeBWorK source remains opaque.
  - Evidence (runtime): the C910 actual-HTTP proof exercised `crates/server/src/assessment_delivery/history.rs` `student_history`; the history read succeeded after the renderer stopped and exposed no choice, correct, or incorrect feedback reconstructed from the PGML source or rendered output.
- [ ] PLE-managed Hints, Question Feedback, and Worked Solutions remain separate from backend-generated
  interaction feedback.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] A Question Backend returns an immutable credit fraction for each complete response it evaluates.
  - Mismatch: needs test or runtime evidence for backend evaluation and immutable outcome creation.
- [ ] PLE stores the immutable credit fraction as the grading outcome.
  - Mismatch: needs test or runtime evidence linking backend credit to stored outcome.
- [ ] When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
  - Mismatch: No test/runtime proof of immediate backend grading outcome was recorded.
- [ ] Assessment scores are calculated from stored credit fractions and current Question point values.
  - Mismatch: needs test or runtime scoring evidence.
- [ ] Changing Question point values recalculates scores without another Question Backend interaction.
  - Mismatch: needs test or runtime rescoring evidence.

#### WeBWorK source and algorithmic Questions

- [x] Preserve the distinction between WeBWorK PG and PGML source. A Question should be identified as PGML only when its source is fully PGML-compliant; otherwise identify it as PG.
  - Evidence (source): `crates/project-tools/src/pilot_content.rs` `validated_question_format` maps only explicit PG or PGML declarations with matching extensions.
  - Evidence (test): `crates/project-tools/src/pilot_content/tests.rs` `pilot_publication_preserves_explicit_source_formats` exercises the format/extension refusals.
  - Evidence (runtime): `crates/project-tools/src/pilot_content/publication.rs` `existing_publication` passed accepted Pilot binding proof preserving source SHA, size, path, and exact explicit format through immutable replay; format/path refusal remains static-only. Artifact: `/private/tmp/ple-pilot-format-binding-artifacts.CKzka1`.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content.rs` `validate_selected_parameterized_manifest` passed selected ordinary-Instructor CLI proof publishing one canonical PGML source with exact bytes, checksum, and provenance; replay made no additional publication. Artifact: `/private/tmp/ple-canonical-family-artifacts.TOOlBJ`.
- [x] BiologyProblems.org imports should preserve whether the canonical algorithmic source is PG or PGML rather than treating both formats generically as PG/PGML.
  - Evidence (source): `crates/project-tools/src/curriculum_content.rs` `validate_selected_parameterized_manifest` validates each explicit source format/path pair.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication reading all 42 canonical entries as explicit PGML source paths and ordinary WeBWorK Question lineages. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.
- [x] When parameterized WeBWorK PG or PGML source exists, prefer it to importing static variants.
  - Evidence (source): `crates/project-tools/src/curriculum_content/publication.rs` `validate_loaded_content` requires direct Fixed Question entries.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication using its 42 canonical parameterized PGML sources as direct Fixed entries rather than generated static variants. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.
- [x] Preserve backend-native algorithmic variation rather than expanding one algorithmic Question into static variants.
  - Evidence (source): `crates/project-tools/src/curriculum_content/parameterized_publication.rs` `publish_source` publishes a parameterized source without static expansion.
  - Evidence (runtime): `docs/active_plans/active/human_guidance_implementation_compliance_plan.md` `C839` accepts canonical source hashes with repeatable/reseeded renderer variation and deterministic grading.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/parameterized_publication.rs` `publish_source` passed fresh publication of 42 ordinary WeBWorK lineages, and exact replay made no mutation. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.
- [x] One algorithmic Question remains one Published Question regardless of how many variants its Question Backend can generate.
  - Evidence (source): `crates/project-tools/src/curriculum_content/publication.rs` `existing_source_revisions` resolves one ordinary lineage per canonical source.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted canonical installation creating 42 Question lineages and 42 Revision 1 records from 42 sources, with no variant expansion. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/parameterized_publication.rs` `publish_selected_with_context` and `publish_source` passed selected ordinary-Instructor CLI proof publishing one available Revision-1 Question from one canonical source, with no Pool or Blueprint; replay made no additional publication. Artifact: `/private/tmp/ple-canonical-family-artifacts.TOOlBJ`.
- [x] Use a Question Pool with algorithmic Questions only when the **Instructor** wants selection among distinct Questions, not to represent variants of one algorithmic Question.
  - Evidence (source): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` resolves selected Published Question Revisions and requires the Instructor's interchangeability attestation; `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` selects distinct immutable members without backend-specific variant expansion.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted fresh PostgreSQL 17/MinIO actual-server and private bundled-main HTTP-proxy browser proof: 42 canonical Genetics Questions installed with zero implicit Pools, then the Instructor visibly selected distinct DNA structure and nucleotide components Revision-1 PGML Questions, attested interchangeability, created a reusable Pool, and imported a distinct Assessment-owned fork with `selection_count=1`. Real WeBWorK rendering, radio-response save/resume, exact fork Pool/Question Revision, issued ID, seed/hash preservation, whole-Attempt submit, and fresh new-Attempt selection/issued IDs passed; a new Attempt may select the same Question and need not have different seeds. Artifact: `/private/tmp/ple-algorithmic-pool-artifacts.K2Kk6Z`. Release used a 3600-second time limit and Correct answer Never; answer disclosure, full Live Demo/authentication/TLS, and all-backend acceptance are outside this receipt. Browser error arrays were empty after route teardown completed.
- [x] BiologyProblems.org WeBWorK problems should be imported from their canonical algorithmic PG or PGML source rather than from generated static variants.
  - Evidence (source): `crates/project-tools/src/curriculum_content.rs` `validate_selected_parameterized_manifest` validates canonical source pins before publication.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication importing all 42 C839-accepted canonical PGML sources (41 BiologyProblems.org sources plus HLA), preserving source pins and producing ordinary available WeBWorK Question lineages. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.
- [x] Multiple static BiologyProblems.org questions generated from one algorithmic source represent one Published Question, not separate Published Questions or a Question Pool.
  - Evidence (source): `crates/project-tools/src/curriculum_content/publication.rs` `validate_loaded_content` rejects non-Fixed entries in the canonical Blueprint.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication creating one Revision-1 Question lineage per canonical source, 42 direct Fixed entries, and zero Pools; exact replay was unchanged and a same-short-name conflict made no mutation. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.

### Published Question specifications

- [ ] A Published Question is an immutable-revision Question available for reuse through the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision_is_immutable` trigger protects revision rows.
  - Verification pending: reconcile the current immutable-revision Library reuse projection against this consolidated requirement.
- [x] Published Questions are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `question_library_entries` requires an active Instructor Account and exposes available Question summaries.

#### Published Question identity specifications

- [x] Published Questions receive a public `XXXX-ZXXX` Crockford Base32 ID.
  - Evidence (source): `crates/server/src/question_publication.rs` `RandomQuestionIdIssuer` mints the exact public form, and `schemas/base_schema/50_functions/question_lineages.sql` `published_question_id_is_crockford_shape` enforces it on stored lineages.
- [x] Question IDs have the canonical form `XXXX-ZXXX`.
  - Evidence (source): `crates/question_model/src/question_library.rs` `impl std::str::FromStr for QuestionId` accepts only the exact nine-character hyphenated syntax with its embedded checksum.
- [x] The hyphen is part of the canonical ID and makes Question IDs immediately recognizable.
  - Evidence (source): `crates/question_model/src/question_library.rs` `impl std::str::FromStr for QuestionId` requires the hyphen at `QUESTION_ID_HYPHEN_INDEX` in every canonical value.
- [x] Human-entered Question IDs may omit the hyphen.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` accepts an eight-character explicit human-entry value and inserts the canonical hyphen before validation.
- [x] Normalize accepted human input to the canonical hyphenated form before validation and lookup.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` normalizes explicit human entry, inserts the hyphen, and invokes `validateCanonicalQuestionIdSyntax`.
- [ ] PLE always stores, transmits, displays, and copies the canonical hyphenated form.
  - Verification pending: strict model, SQL, and browser validators are current, but every storage, transport, display, and copy surface has not been inventoried.
- [x] Seven Crockford Base32 characters are cryptographically random and provide the identity.
  - Evidence (source): `crates/server/src/question_publication.rs` `RandomQuestionIdIssuer` draws seven characters from operating-system randomness before `QuestionId` appends the checksum.
- [x] IDs never encode creation order, Question Type, ownership, subject, or other metadata.
  - Evidence (source): `crates/server/src/question_publication.rs` `RandomQuestionIdIssuer` derives the seven identity characters only from operating-system random bytes.

#### Published Question metadata

- [x] Published Questions have metadata specific to the individual Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question_metadata` keys individual metadata to `question_id` and requires nonempty `question_title` and `question_description` independently of Course placement.
- [x] Published Question metadata includes Title and Description.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question_metadata` keys individual metadata to `question_id` and requires nonempty `question_title` and `question_description` independently of Course placement.
- [x] Published Question metadata may include authorship, attribution, license, and source information.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `validate_question_publication` requires exact source, contiguous revision authorship and license records, keeping them associated with the Published Question Revision.
- [ ] Published Questions may include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Published Questions also use the shared Question Library metadata required for publication.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/50_functions/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.

#### Published Question revisions, edits, and forks

- [x] **Published Questions** maintain immutable revision history.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision_is_immutable` trigger protects revision rows.
- [ ] Assessments and Student Work remain pinned to exact immutable Published Question Revisions.
  - Mismatch: exact revision columns are source evidence only; no connected test verifies an Assessment and Student Work stay pinned across a later publication.
- [x] Publishing a new Question Revision does not silently change existing Assessments or Student Work.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` publication appends `next_revision_number` rather than rewriting prior rows.
- [x] The Question owner may publish corrections, wording changes, accessibility improvements, answer changes, grading changes, and other updates as a new Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` appends an owner-authored revision.
- [ ] Changing Question source, answer content, grading rules, Hints, Question Feedback, Worked Solutions,
  or Question assets creates a new Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Changes to the Question title, description, Discipline, Subject, Topic, Subtopic, Tags, or other
  search metadata update the Published Question metadata while preserving the current Question Revision.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [x] Search metadata belongs to the Published Question as a whole rather than to one Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question_metadata` keys metadata to `question_id` only.
- [x] Any **Instructor** may fork a Published Question to create a separate Question with a new Question ID.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionForkControl` invokes the exact-Revision server command, which mints the separate Question ID and opens only the returned private Draft.
  - Evidence (runtime): `src/pages/question_detail_page.tsx` `QuestionForkControl` passed accepted C879 connected PostgreSQL/server/browser proof with two Instructors, exact source attribution, private cross-account denial, retry/concurrency, a distinct server-issued identity, and prevalidation-publication denial; exact canonical-ID proof remains required.
- [x] A fork starts as a private **Draft Question** with its own authorship and lineage.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `draft_question_fork_source` records a private draft fork source.
  - Evidence (runtime): `src/pages/question_detail_page.tsx` `QuestionForkControl` passed accepted C879 connected browser proof that opened only the returned private Draft for the invoking Instructor and denied the other Instructor.
- [x] A fork must pass Question Publication Validation before joining the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `validate_question_publication` guards publication.
- [x] Published forks retain source attribution.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `question_fork_source` records published fork provenance.
- [x] Forced corrections are audited **Sysadmin** actions reserved for critical flaws.
  - Evidence (source): `schemas/base_schema/20_tables/corrections.sql` `forced_question_correction` and its immutable audit targets record correction actions.
- [x] Question authorship, contributor credit, history, attribution, and compatible CC licensing are preserved across Revisions and forks.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `question_revision_authorship` and `question_revision_license` preserve revision stewardship.
  - Evidence (runtime): `schemas/base_schema/50_functions/question_publication_operations.sql` `ple_private.publish_new_question_lineage` passed the accepted C879 3-by-3 PostgreSQL publication proof: each exact source Revision license was preserved across three supported compatible CC licenses and every mismatched requested license was rejected.
- [ ] Watching a Published Question drives in-app notifications for new Revisions, forks, improvement
  threads, and impact notices.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records fresh PostgreSQL 17 actual-role proof for all four private Watch event kinds.
  - Verification pending: final connected HTTP/UI and browser notification presentation remain open.

#### Published Question behavior specifications

- [ ] Published Questions may include optional PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
  - Owner: 07_questions.md / Published Question metadata (first occurrence; identical requirement and status).
- [ ] PLE-managed Hints, Question Feedback, and Worked Solutions are separate from Question Backend-generated content.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] WeBWorK Questions may use PLE-managed Hints, Question Feedback, and Worked Solutions even when similar material also exists in the WeBWorK source.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Question Feedback is shown when its disclosure rules allow it.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Hints and Worked Solutions use their own disclosure settings.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Student workflows remain complete when a Question has none of this optional support content.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.

### Question Pool specifications

- [x] A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` persists an ordered nonempty set of exact Published Question Revision members, and `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` selects from that Pool for Student delivery.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `start` passed accepted actual-server proof that selected an exact Pool member for Student Attempt 1, preserved it on resume, and selected again for Attempt 2. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Pool contents should represent reasonably interchangeable assessments of the intended learning.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` requires the creating Instructor's true `interchangeability_attested` value; it does not substitute an automatic pedagogical evaluator.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted actual-main proof that required the Instructor's attestation before creating the ordered reusable Pool and before its later Assessment-owned reorder. Artifacts: `/private/tmp/ple-course-empty-artifacts.bzwXEa` and `/private/tmp/ple-course-empty-artifacts.lgyOMK`.
  - Evidence (runtime): `crates/server/src/question_pool_creation.rs` `create_question_pool` passed accepted actual-server proof that false or missing attestation returned 422 and left no Pool behind. Artifact: `/private/tmp/ple-course-empty-artifacts.hvS4KT`.
- [ ] Question Pools may contain Questions from any Question Backend.
  - Mismatch: C885 supplies backend-neutral Pool membership, but no completed Instructor Pool workflow proves this behavior.
- [ ] Question Pools are created from a Published Question and enter the Question Library immediately.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [x] A Question Pool is an independently reusable Question Library object.
  - Evidence (source): `crates/server/src/question_pool_library.rs` `current_pool` reads a Pool independently of any Assessment.
  - Evidence (runtime): `crates/server/src/question_pool_library.rs` `current_pool` passed accepted actual-main Instructor proof: Pool `SBQR-N5RE` was created from the Question Library and its ordered member pins were read through `/api/question-pools/SBQR-N5RE`; separate actual-server proof then imported another reusable Pool into an Assessment.
- [x] Question Pools are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` and `read_current_published_question_pool` authorize active Instructors and project only public Pool/Revision/member facts.
  - Evidence (runtime): `crates/server/src/question_pool_library.rs` `list_pools` passed accepted actual-server proof that a second vetted Instructor listed and read root Pool `1N6T-MZRD` and child Pool `J1BX-8V8F` with exact public member pins and no Course facts. A nonmember Assessment-fork PUT returned 404 without mutation; Student and anonymous Pool list/read calls returned no-store 404. Artifact: `/private/tmp/ple-course-empty-artifacts.hvS4KT`.
- [x] A Question Pool has its own public `XXXX-ZXXX` Crockford Base32 ID and immutable Revisions.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `question_pool` stores its canonical public identity, `question_pool_revision` stores sequential immutable Revisions, and `question_pool_public_id_is_reserved` enters the ID in the shared registry.
- [x] Importing a Question Pool into a new Assessment automatically forks the Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_pool_forks.sql` `import_assessment_question_pool_fork` atomically creates a fresh child Pool Revision and Assessment Entry from an exact reusable source Revision without accepting raw member pins.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `import_fork` passed accepted actual-server proof that imported source Pool `P8H3-QYX9` into a direct Assessment and returned distinct fork `VFH9-CQKS`, Revision 1, at Assessment Edit 2.
- [x] The fork belongs to the new Assessment and can be changed without changing the source Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `assessment_question_pool_fork` owns each child Pool through exactly one Assessment Entry, and `schemas/base_schema/50_functions/question_pools.sql` retains exact source-Revision provenance.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `append_fork_revision` passed accepted actual-server proof that appended the fork's Revision 2 with the two exact member pins reversed, then reread the reusable source unchanged at Revision 1 with its original order. Artifact: `/private/tmp/ple-course-empty-artifacts.BbKFFd`.
- [x] Forking a Question Pool preserves its list of Published Questions by their public `XXXX-ZXXX` IDs.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `construct_question_pool_revision_fork` copies the source Revision's ordered exact member Question IDs and Revision Numbers into the new Pool lineage.
- [ ] Question Pools work the same way regardless of the Question Backend.
  - Mismatch: incomplete secondary backend delivery leaves this unverified.
- [x] **Instructors** choose the contents of a Question Pool and how many Questions are selected.
  - Evidence (source): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` submits the Instructor's ordered current Published Question Revisions with interchangeability attestation; `src/pages/assessment_workspace/assessment_pool_entry_editor.tsx` `AssessmentPoolEntryEditor` exposes the Assessment-owned fork's exact members and bounded selection count.
  - Evidence (runtime): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `updatePoolSelectionCount` passed accepted actual-main proof that created the reusable Pool from ordered Published Questions, then imported it, changed its selection count from 2 to 1, attested and reordered its exact members, and reloaded its Revision 2 while the source remained unchanged. Artifacts: `/private/tmp/ple-course-empty-artifacts.bzwXEa` and `/private/tmp/ple-course-empty-artifacts.lgyOMK`.
- [x] PLE selects from the Question Pool; the selected Question Backend controls the Question interaction.
  - Evidence (source): `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` performs server-owned selection.
- [x] Question Pool selection and backend-native randomization are separate forms of variation.
  - Evidence (source): `crates/domain/src/question_pool_selection.rs` `QuestionPoolSelectionEntropy` is separate from Question backend state.
- [x] Returning to an Attempt preserves the Question Pool selections already made.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_operations.sql` `assessment_attempt_start_gate` returns an unfinished resumable Attempt before new issuance, while `crates/server/src/assessment_delivery.rs` `issue_native_assessment_batch` returns its retained committed presentations rather than selecting again.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `issue_native_assessment_batch` passed accepted actual-server proof that returned Attempt 1 with `resumed: true`, the same selected pin, and the same presentation nonce after its first start. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Starting a new Attempt makes fresh selections from its Question Pools.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_operations.sql` `assessment_attempt_start_gate` has no prior-Pool-selection reuse branch; after a submitted Attempt it authorizes a new Attempt, whose new selection payload is persisted by `start_assessment_attempt`.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `start` passed accepted actual-server proof that submitted Attempt 1, then started Attempt 2 with `resumed: false`, a distinct Pool selection ID, and a new presentation nonce. The same selected member remained valid with a two-member Pool. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Student Work preserves the exact Question Pool Revision and Published Question Revision delivered.
  - Evidence (source): `crates/question_model/src/student_work.rs` `QuestionPoolSelection` retains issued Question revision references.
  - Evidence (test): `crates/question_model/src/student_work/model_tests.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the issued revision link.
- [x] Grading and historical evidence follow the exact Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history_response_sources` retains `question_id` and `revision_number`.
  - Evidence (test): `crates/question_model/src/student_work/model_tests.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the exact issued linkage.
- [x] Each member of a Question Pool is a **Published Question**.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `question_pool_revision_member` stores each exact Published Question revision reference.
- [ ] Question Pools contain only **Published Questions**; Question Pools cannot be members of Question Pools.
  - Verification pending: Source-contributor audit must confirm only exact Published Question Revision members and no Pool-member input; broad runtime evidence remains pending.
- [ ] Watching a Question Pool drives in-app notifications for new Revisions, forks, improvement
  threads, and impact notices.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records fresh PostgreSQL 17 actual-role proof for all four private Watch event kinds.
  - Verification pending: final connected HTTP/UI and browser notification presentation remain open.

#### Question Pool metadata

- [x] Question Pools have metadata specific to the individual Question Pool.
  - Evidence (runtime): `schemas/base_schema/50_functions/question_pools.sql` `question_pool` owns independent metadata. Accepted SQL/rollback/concurrency and final SQL, Rust/API, and browser reviews combine with root-supplied rebuilt `8147` HTTP/browser proof at `/private/tmp/ple-pool-metadata-connected-report.md`: independent metadata survives list/current reads and real Library UI creation/retry. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.
- [x] Question Pool metadata includes Title and Description.
  - Evidence (runtime): Required independent Title/Description in `schemas/base_schema/50_functions/question_pools.sql` have accepted SQL and source review. Rebuilt `8147` proof at `/private/tmp/ple-pool-metadata-connected-report.md` rejects missing fields, retains exact list/current text, and preserves both fields after denied mixed-member UI creation. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.
- [x] The first Published Question establishes the Question Pool's Discipline and Subject.
  - Evidence (runtime): Accepted actual-role SQL creation proof and final source reviews establish first-member classification. Rebuilt `8147` HTTP/browser proof at `/private/tmp/ple-pool-metadata-connected-report.md` retains exact ordered pins and first-member Discipline/Subject, rejects mixed Subject with `422` and unchanged public list, then creates after ordinary picker reselection. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.
- [ ] Every additional Published Question added to the Pool has the same Discipline and Subject as the Pool.
  - Verification pending: Accepted actual-role Blueprint-owned append proof rejects classification mismatch atomically, and witnessed two-connection admission/reclassification wait and dual commit preserve Pool classification. Rebuilt connected append acceptance remains pending.
- [ ] Published Questions retain their own Topic, Subtopic, Tags, and other Library Object metadata
  when included in a Question Pool.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Question Pools may have their own authorship, attribution, license, and source information where
  appropriate.
  - Mismatch: The accepted Pool Title/Description/classification/Tags slice does not establish Pool-owned authorship, attribution, license, or source information; audit these separate fields and their authoring/read boundaries.
- [x] Question Pool metadata describes the Pool rather than duplicating metadata from its member
  Published Questions.
  - Evidence (runtime): Accepted SQL/source proof establishes independent Title/Description, empty creation Tags, optional narrower hierarchy, classification retention after Question reclassification, and historical fork preservation. Rebuilt `8147` HTTP/browser proof at `/private/tmp/ple-pool-metadata-connected-report.md` confirms separately authored Pool text through creation, retry, list, and current reads. No historical Pool HTTP route is claimed. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.
- [ ] Question Pools may include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Question Pools also use the shared Question Library metadata required for publication.
  - Verification pending: Pool-owned required Title/Description and Discipline/Subject, optional Topic/Subtopic and unbounded-count Tags have accepted source/SQL proof and rebuilt `8147` connected creation/list/current-read proof. Complete shared publication metadata remains open, including the separately unproved authorship/attribution/license/source and Bloom boundaries; this slice does not establish optional teaching support.

### Question Library specifications

- [x] Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
  - Evidence (source): `src/pages/library_route_page.tsx` `LibraryRoutePage` is the production Instructor Library surface.
- [ ] The Question Library is one global collection of Published Questions and Question Pools.
  - Verification pending: Pool metadata source and actual-role SQL proof now exist alongside Published Question metadata. Audit the complete global collection/discovery boundary on rebuilt connected HTTP/browser surfaces; independent metadata proof does not establish the whole collection.
- [ ] Draft Questions are not part of the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `published_question_metadata` queries only Published Question metadata; Draft working state is stored separately in `schemas/base_schema/50_functions/question_authoring_state.sql` `authoring_draft`.
  - Verification pending: re-evaluate the current Library search/Pool projections and publication boundary to establish explicit Draft exclusion across all Library paths.
  - Owner: 07_questions.md / Draft Question specifications (first occurrence; identical requirement and status).
- [ ] **Published Questions** and Question Pools are available to all vetted **Instructors**.
  - Verification pending: Accepted Pool SQL proof covers stated vetted-Instructor operations and concealed Student/nonowner denials; prior Published Question evidence remains bounded. Rebuilt connected availability across all vetted Instructors and both Library Object kinds remains pending.
- [x] **Students** access Question content through their Coursework rather than through the Question Library.
  - Evidence (source): `src/route_contract.ts` `ROUTE_CONTRACT` reserves both Question Library routes for Instructors, and `src/route_access_boundary.tsx` `withRouteAccessBoundary` fail-closes every protected route before its page component mounts.
  - Evidence (runtime): `src/route_access_boundary.tsx` `withRouteAccessBoundary` passed accepted actual-main Student proof that denied three Library routes without any Question Library API request, while the Student Ribbon allowed Coursework navigation to a Released Assessment. Earlier accepted native Student Attempt proof delivered Question content through that Assessment. Artifacts: `/private/tmp/ple-course-empty-artifacts.9s89JA` and `/private/tmp/ple-course-empty-artifacts.ZquNiI`.
- [x] Question Library content remains discoverable when used by a private **Course Instance**.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionSearchResult` is global and separately reports course use.
- [ ] With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.
  - Mismatch: no product test or design enforcement establishes archive as non-primary at this scale.
- [ ] Question Library workflows should support bulk operations because an **Instructor** may manage thousands of Questions.
  - Evidence (test): temporary compiled Chromium component and strict-client proof accepted sorted selection/Edit Numbers, closed replace/clear patches, virtualization, busy controls, blank-replace rejection, pre-fetch canonical-ID rejection, stale/ambiguous refresh, denial, filter clearing, no page errors, and zero critical/serious axe findings; the mock/injected transport was not server-connected and the proof was removed.
  - Mismatch: connected HTTP and practical-scale workflow evidence remains pending.
- [ ] **Instructors** should be able to select many Library objects and update shared metadata such as
  Discipline, Subject, Topic, Subtopic, Tags, or other search fields together.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.
  - Mismatch: search, filters, and an accepted mock-transport browser metadata workflow exist, but connected HTTP and 13k practical-cleanup evidence remains pending.

#### Question Library metadata

- [ ] **Library Objects** use shared metadata for organization, search, filtering, and discovery.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Question Library metadata boundary.
- [ ] Required Question Library metadata must be complete before a Library Object enters the Question Library.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Question Library metadata boundary.
- [ ] Library metadata should describe the Library Object rather than its location in a Course, Assessment, or textbook.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Question Library metadata boundary.
- [ ] Library Objects use the shared **Discipline**, **Subject**, **Topic**, **Subtopic**, and **Tag**
  vocabulary.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Every Library Object has exactly one **Discipline** and one **Subject**.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
  - Owner: Content classification (first occurrence).
- [ ] **Topic** and **Subtopic** are optional for Library Objects.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
  - Owner: Content classification (first occurrence).
- [ ] Library Objects may have any number of **Tags**, including none.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Question Publication Validation requires Discipline and Subject before publication.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Library Object classification follows Discipline -> Subject -> Topic -> Subtopic.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Questions and Question Pools retain their Library Object classification when used in an Assessment.
  - Verification pending: Current Human Guidance requirement is new or changed; independent implementation audit and applicable proof remain pending.
- [ ] Library classification supports searching, filtering, sorting, and bulk editing.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Question Library metadata boundary.
- [ ] Published Questions and Question Pools may have PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Support content may be attached at the level where it applies rather than duplicated across individual Questions.
  - Mismatch: `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.

#### Question Library object statistics

- [ ] Published Questions and Question Pools may retain privacy-safe aggregate statistics.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` and `question_revision_choice_statistics` retain identity-free Question Revision counts; `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` preserves those aggregate rows while deleting Course Student evidence.
  - Verification pending: the 2026-09-16 actual-role PostgreSQL 17 purge gate had no submitted Student Work or aggregate-statistics fixture. Pool Revision/use/selection statistics, privacy thresholds, product display, and connected retention acceptance remain open.
- [ ] Statistics are kept separately for each Published Question Revision and Question Pool Revision.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` keys Question statistics by `(question_id, revision_number)` and preserves those identity-free rows through `ple_api.delete_course_student_records`.
  - Verification pending: no Pool Revision/use/selection statistics model exists, and the 2026-09-16 actual-role PostgreSQL 17 purge gate exercised neither submitted Work nor aggregate rows.
- [ ] Each Published Question Revision may retain aggregate counts of correct, incorrect, partial-credit,
  and unanswered results.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` retains accepted graded-Attempt and correct counts by exact Question Revision, and `question_revision_choice_statistics` retains eligible choice counts.
  - Verification pending: the complete result-count model, released Instructor Statistics surface, disclosure/privacy rules, and connected acceptance remain open.
- [x] Eligible Question Types may also retain aggregate answer-choice counts.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `selected_count` stores aggregate choice counts.
  - Owner: 06_data.md / Student and FERPA data (first occurrence; identical requirement and status).
- [ ] Each Question Pool Revision may retain aggregate statistics for its use and Question selections.
  - Verification pending: `schemas/base_schema/20_tables/statistics.sql` supplies only Question Revision aggregates, which the deletion transition preserves. Pool Revision/use/selection statistics and their privacy/retention oracle are not implemented.
- [ ] Published Question and Question Pool statistics may combine Revisions when clearly labeled and
  privacy thresholds are met.
  - Verification pending: `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` supplies Question-only aggregate context; this requirement now also applies to Pool Revisions/use/selection or revised privacy/retention semantics. Audit the exact aggregate model and privacy/retention oracle; Question-only evidence is insufficient.
- [ ] Aggregate statistics contain counts rather than Student Attempts or identifiable Student records.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` stores count fields by Question Revision and choice, while `schemas/base_schema/50_functions/course_retention_transitions.sql` deletes private Attempt roots and keeps existing identity-free aggregate rows.
  - Verification pending: no accepted aggregate disclosure/small-cohort rule, Pool aggregate model, or connected product surface proves that all exposed statistics prevent reconstruction of Student activity.
- [ ] Privacy-safe aggregate statistics remain after the underlying Student records are deleted.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` explicitly preserves existing identity-free aggregate rows; `schemas/base_schema/20_tables/statistics.sql` documents retained Question Revision statistics after Course Student-record deletion.
  - Verification pending: the 2026-09-16 actual-role PostgreSQL 17 purge gate exercised no submitted Student Work or aggregate rows. Pool statistics, privacy thresholds, and connected retention acceptance remain open.
- [ ] Student data retention removes the underlying Student evidence without removing approved aggregate
  statistics.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` removes private Attempt roots and Course Student evidence while preserving existing identity-free aggregate rows.
  - Verification pending: the 2026-09-16 actual-role PostgreSQL 17 purge gate proved bounded deletion but had no submitted Student Work or aggregate-statistics fixture. Approved aggregate policy, Pool coverage, and connected acceptance remain open.
- [ ] Removing Student names alone does not make statistics anonymous.
  - Mismatch: no released Question Statistics policy establishes this behavior.
- [ ] Shared statistics should be shown only when individual Students cannot reasonably be identified
  from the aggregate.
  - Mismatch: `QuestionStatistics` is currently `Unavailable`; no shared-view privacy threshold exists.
- [ ] Course-specific analysis remains FERPA-sensitive when individual Students could be inferred.
  - Mismatch: aggregate analysis structures exist, but no complete FERPA-sensitive product workflow was verified.

#### Question Library Bloom classification metadata

- [ ] Published Question Revisions and Question Pool Revisions have a Bloom Cognitive Process and Bloom
  Knowledge Dimension.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` stores non-null pairs by exact immutable Question or Pool Revision. `crates/question_model/src/bloom_classification.rs` defines the browser-safe pair and its independent Edit Number; Question and exact Pool reads project it through `src/pages/library_page_model.ts` and `src/pages/library_pool_discovery.tsx`.
  - Verification pending: classifier/provider selection and orchestration plus connected browser reads remain open; fresh actual-role proof closes the SQL boundary.
- [ ] The two Bloom dimensions are independent and together determine the object's Bloom Classification.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` validates the two independent closed-vocabulary fields and stores a complete pair rather than a derived matrix value. `crates/learning-data-access/src/question_library.rs` and `crates/learning-data-access/src/question_pool_library.rs` return the pair with its exact-Revision Edit Number.
  - Verification pending: classifier/provider selection and orchestration plus connected browser reads remain open; fresh actual-role proof closes the SQL boundary.
- [ ] Bloom Classification describes the cognitive work required for full credit, not Question Difficulty.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` stores the two guide-defined classification dimensions separately from Question source, scoring, and immutable content Revision data. `src/components/bloom_classification.tsx` presents the exact pair and links its correction help to `docs/BLOOM_TAXONOMY_GUIDE.md`.
  - Verification pending: AI semantic classification, classifier/provider orchestration, and connected Instructor interpretation remain open; SQL publication admission is closed.
- [ ] Bloom Classification supports Question Library search and Assessment item sorting.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` projects every fixed Entry, including retained Entries, from its exact pinned Question Revision pair and every Pool Entry from its exact Assessment-owned fork Pool Revision pair. `src/pages/assessment_workspace/assessment_workspace_questions_model.ts` orders Cognitive Process, Knowledge Dimension, then prior position; equal pairs remain stable. The existing whole-Assessment Save retains its Edit Number CAS.
  - Verification pending: source implementation is present, but connected Instructor proof must sort mixed fixed and Pool Entries, save, reload, and show persisted order plus a concurrent-save conflict. Library discovery has source evidence but still needs connected proof, so this combined requirement remains open.
- [ ] A Question Pool's Bloom Classification describes the intended cognitive work of the Pool as a whole.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` stores a Pool Revision's own pair by `(question_pool_id, revision_number)`, rather than deriving it from member Questions.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records fresh PostgreSQL 17 actual-role proof of protected candidate/receipt binding and Pool Library admission.
  - Verification pending: classifier/provider selection and orchestration, typed API/UI projection, and connected search/reporting remain open.
- [ ] Bloom Classification is required before a Published Question or Question Pool enters the Question
  Library.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records fresh PostgreSQL 17 actual-role proof of deferred completeness and Question plus Pool Library admission through one-use protected receipts.
  - Verification pending: configured classifier/provider orchestration and connected publication/browser acceptance remain open.
- [ ] AI assigns the initial Bloom Classification as part of publication.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records the closed SQL preparation boundary: protected candidate/receipt binding, one use, rollback restoration, and publication admission.
  - Verification pending: application-owned classifier/provider selection, semantic classification, and connected publication proof remain open.
- [ ] An **Instructor** can correct either Bloom dimension without creating a new Published Question or
  Question Pool Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` CAS-updates only paired metadata and its classification Edit Number. Typed Question and Pool Stores bind complete-pair commands to exact Revisions; `crates/server/src/question_library.rs` and `src/api/http_client/bloom_classification.ts` expose their routes. `src/components/bloom_classification.tsx` retains drafts, reloads stale state without retrying, and returns focus after completion; Question and Pool detail editors bind exact Revision targets.
  - Verification pending: the 2026-09-16 PostgreSQL 17 gate proved bounded authorization/no-op/stale behavior. Connected two-Instructor, denied-role, and browser correction/focus proof remains open.
- [ ] Question Library search and reporting should make both Bloom dimensions useful to **Instructors**.
  - Evidence (source): `crates/question_model/src/question_search.rs` retains two independent exact Bloom filters, unchanged sorts, and normalized-query-bound cursors. `crates/learning-data-access/src/postgres/question_library.rs` applies them to the whole Library relation and computes all six plus all four guide-order counts; `src/pages/library_search_parameters.ts`, `src/pages/library_page.tsx`, and `src/components/library_bloom_discovery.tsx` retain URL/saved-search values, zeros, and empty results.
  - Verification pending: connected multi-page, role, and browser proof remains required. It stays open independently of the connected mixed-entry Assessment-sort/save/reload/concurrent-save proof required by the preceding row.
- [ ] Follow `docs/BLOOM_TAXONOMY_GUIDE.md` for Bloom classification and teaching interpretation.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` accepts only the guide's six Cognitive Process and four Knowledge Dimension spellings.
  - Verification pending: fresh PostgreSQL 17 actual-role proof closes storage and publication-required attachment; classifier/provider semantics, Instructor-facing teaching interpretation, and connected search/reporting remain open.

#### Question Library stewardship specifications

- [ ] Question Library stewardship should use a GitHub-like model.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records current Question/Pool Star and Watch SQL/LDA proof plus four-event private Watch delivery.
  - Verification pending: connected Question/Pool workflows and browser presentation remain open.
- [ ] Published Questions and Question Pools can be starred and watched.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records current Question/Pool Star and private Watch persistence proof.
  - Verification pending: connected Question/Pool controls and browser proof remain open.
- [x] Star means favorite and visible endorsement.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `set_current_question_star` records an active Instructor's Star only for a Published Question; `src/components/question_star_control.tsx` `QuestionStarControl` provides the visible Star and count surface.
  - Evidence (test): `tests/e2e/e2e_question_star_name_privacy.sh` `Question Star name privacy E2E` passed on 2026-09-15 with an active vetted Instructor's actual HTTP Star action and exact closed Star projection.
- [ ] Vetted **Instructors** can see the star count and which vetted **Instructors** starred a Published
  Question or Question Pool.
  - Verification pending: current SQL/LDA proof covers Question/Pool stewardship persistence; connected authorized identity-list projection and browser proof remain open.
- [ ] Watch means subscription.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records current private Question/Pool Watch persistence proof.
  - Verification pending: connected subscription controls and browser proof remain open.
- [ ] Watching a Published Question or Question Pool drives in-app notifications for new Revisions,
  forks, improvement threads, and impact notices.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records fresh PostgreSQL 17 actual-role proof for private Revision, fork, improvement-thread, and impact-notice delivery.
  - Verification pending: final connected HTTP/UI and browser notification presentation remain open.
- [ ] An **Instructor's** watch list remains private.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records private Question/Pool Watch persistence and recipient delivery.
  - Verification pending: connected privacy and browser proof remain open.
- [ ] **Students** and anonymous users do not receive **Instructor** identity lists or watch information.
  - Evidence (runtime): `docs/active_plans/audits/sql_human_guidance_audit.md` records the private recipient SQL boundary.
  - Verification pending: connected anonymous/Student denial and browser proof remain open.
