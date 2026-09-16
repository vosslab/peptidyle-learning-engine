## Question specifications

- [x] Questions are subject agnostic. Properly classified Published Questions from all subjects belong in
  the same Question Library.
  - Evidence (source): `crates/server/src/question_library/paging.rs` `QuestionSearchFilter` supplies the shared Library query filter without a subject partition.
- [ ] Questions are strictly and deterministically automated; grading does not require an **Instructor**.
  - Mismatch: needs runtime grading evidence for every supported backend.
- [x] Questions have one canonical title. Compact interfaces may truncate that title.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question_metadata` stores one lineage-level title.
- [x] Every Question stored by PLE has its own internal Question record.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question` owns the internal Question record.
- [ ] Answer-choice randomization belongs to the Question.
  - Mismatch: native answer-choice randomization ownership has not been verified.
- [ ] PLE-native Questions control their own answer-choice randomization.
  - Mismatch: no native answer-choice randomization implementation was found.

### Draft Question specifications

- [x] Draft Questions are private working content.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `ple_private.draft_question` stores draft state in the private schema.
- [ ] Draft Questions are not part of the Question Library.
  - Evidence (source): `schemas/base_schema/question_library_operations.sql` `published_question_metadata` queries only Published Question metadata; Draft working state is stored separately in `schemas/base_schema/question_authoring_state.sql` `authoring_draft`.
  - Verification pending: re-evaluate the current Library search/Pool projections and publication boundary to establish explicit Draft exclusion across all Library paths.
- [x] Draft Questions use current state rather than immutable Revisions.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `draft_question_edit_number` is current-state concurrency data, separate from `question_revision`.
- [x] Saving a Draft Question replaces its previous working state.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `save_authoring_draft` replaces the current draft aggregate values.
- [x] **Instructors** may delete Draft Questions they no longer need.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `delete_draft_question` resolves only the current Instructor-owned Draft, locks and compares its Edit Number, then deletes that private aggregate without considering the separate Published Question lineage.
  - Evidence (source): `crates/learning-data-access/src/postgres/authoring.rs` `delete_authoring_draft` carries the SQL compare-and-swap through the authenticated Store.
  - Evidence (source): `crates/server/src/authoring.rs` `delete_draft` requires the parsed `If-Match` Edit Number and maps a concurrent change to 412; `src/pages/question_drafts_page.tsx` `QuestionDraftsPage` supplies explicit Keep/Delete confirmation.
  - Evidence (runtime): `crates/server/src/authoring.rs` `delete_draft` passed accepted isolated PostgreSQL 17/MinIO actual-server and focused browser proof: cancel, confirm, and list reload; valid-current-ETag collaborator, unrelated Instructor, Student, Sysadmin, and anonymous 404 denials while owner source/Edit Number remained unchanged; 428 missing, 400 malformed, and 412 stale preconditions; preserved parsed Published Question lineage and Revision JSON after a published-origin Draft deletion; and 404 repeat DELETE/PUT. Artifact: `/private/tmp/ple-draft-delete-artifacts.km9ybM`.
- N/A PLE may clean up abandoned Draft Questions after an appropriate warning and recovery period.
  - Reason: Automated abandoned-Draft cleanup is an explicitly optional future capability; HG sets no clock or durations.
- [ ] A Draft Question must pass Question Publication Validation before becoming a Published Question.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `validate_question_publication` guards publication.
  - Verification pending: audit ordinary Draft publication, not only fork publication, against current validation and required Library metadata.
- [ ] Publication requires all required Question Library metadata.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.

### Question formats and type specifications

- [x] PLE flat-question JSON is the canonical machine format for simple static Questions.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` validates the PLE JSON source form.
- [x] QTI is for import, export, and archival interchange rather than the internal source model.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `workspace_import` treats `qti` as an import format, not a source binding.
- [x] MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT Question Types should be supported.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `question_revision` CHECK lists all eight types.
- [x] Question Type is immutable author-declared educational metadata on a Published Question Revision.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `question_revision_is_immutable` protects `question_type` on a revision.
- [x] PLE uses Question Type for search, filtering, labeling, and presentation.
  - Evidence (source): `src/api/question_library_repository.ts` `questionSearchRequest` sends the selected Question Type as the Library search filter; `src/pages/library_page.tsx` `questionTypeLabel` supplies learner-facing type labels and the Question Type selector presents the type facets.
- [x] Question Type comes from the author rather than inference from backend controls.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `draft_question_source_binding` records authoring input independent of backend.
- [x] Question importers are transient translators from external formats into PLE-managed Question representations.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `workspace_import` stages external-format imports before committed PLE state.

### Native PLE JSON Question specifications

- [x] The native PLE JSON Question format is private, unversioned, and unpublished.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` accepts the unversioned internal source shape.
- [x] Stored native JSON Questions may be upgraded together when the internal format changes.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` is the single internal reader for stored PLE JSON.
- [x] The native PLE JSON Question format is a strictly validated internal source shape without an external API.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` validates the internal source document.
- [x] Native JSON Questions are static, not algorithmic nor random, and receive no random seed.
  - Evidence (source): `crates/question_model/src/generation.rs` `QuestionReproduction` distinguishes static source reproduction from the inseparable seeded generator pair; `crates/adapters/ple/src/lib/question_json_source.rs` `presentation` issues native PLE JSON with `QuestionReproduction::Static`.
  - Evidence (source): `schemas/base_schema/assessment_attempts.sql` `validate_issued_question_reproduction` rejects a seed for a `ple` source and requires one for renderer-backed sources.
  - Evidence (runtime): `schemas/base_schema/assessment_attempts.sql` `validate_issued_question_reproduction` passed in `/private/tmp/ple-native-seed-proof.sh --isolated --native-seed-http` against PostgreSQL 17: shuffled-position-2 native seed/hash were null, real WeBWorK retained numeric seed/64-character hash privately, public start/read/save/resume/restored payloads omitted both fields, resume retained the same issued Questions and saved native response, and invalid native seed insertion failed. Artifact: `/private/tmp/ple-native-seed-artifacts.KfY7Op`.
- [x] Native PLE JSON supports MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonResponse` defines all eight native types.
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
- [ ] Native interactive Question Types such as HOTSPOT use PLE-owned interaction code.
  - Mismatch: HOTSPOT source editing exists, but delivered interaction evidence was not found.
- [ ] HOTSPOT content uses supported static assets such as images and SVG.
  - Mismatch: no delivery validation for HOTSPOT static assets was found.
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

- [ ] WeBWorK, iMathAS, and H5P are PLE-managed Question Backends.
  - Mismatch: C870 removed every current H5P source, import, adapter, and runtime seam; H5P is not a delivered backend.
  - Question: Which H5P content type(s) are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [x] The initial primary Question Backends are PLE-native JSON and WeBWorK.
  - Evidence (source): `schemas/base_schema/assessment_attempt_presentation.sql` `backend IN ('ple', 'webwork')` is the delivered presentation boundary.
- [ ] iMathAS and H5P are supported secondary Question Backends.
  - Mismatch: iMathAS has a launch boundary, while C870 leaves no current H5P source, import, or delivered backend seam.
  - Question: Which H5P content type(s) are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [x] PLE-native Questions use the PLE Question Backend.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `question_source_binding_fields_are_valid` maps `ple` to `pleQuestionJson`.
- [ ] WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
  - Mismatch: the isolated opaque adapter proves renderer documents, ordered pairs, score, partial credit, and stateless state. Connected live-ownership proof remains required; PLE is not required to capture historic renderer feedback.
- [ ] H5P owns its runtime, interactions, state, and scoring.
  - Mismatch: C870 leaves no current H5P source, import, adapter, or runtime seam, so H5P cannot yet own delivered runtime behavior.
  - Question: Which H5P content type(s) are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [ ] iMathAS owns its rendering and evaluation.
  - Mismatch: needs runtime rendering and evaluation proof for iMathAS.

#### Question Backend responsibilities

- [x] PLE owns and stores the Question representation used for each Question Backend.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `question_revision_source_binding` stores backend representations.
- N/A Imported backend source may be transformed into the form PLE stores and manages.
  - Reason: Optional transformation does not require a current backend-import behavior.
- [x] PLE preserves the information needed to reproduce the Question through its backend.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `question_revision_source_binding` retains backend selectors and source checksum.
- [x] PLE-managed Question representations participate in Question revision history.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `question_revision_source_binding` keys source bindings to immutable revisions.
- [ ] Question Backends own rendering, interaction, response, grading, feedback, and backend-specific state.
  - Mismatch: needs backend render-and-grade runtime evidence; current native and WeBWorK boundaries are not proof for all supported backends.
- [ ] PLE owns authorization, Question ID, revisions, persistence, lifecycle, and stored outcomes.
  - Mismatch: schema ownership is evidence for records, but no connected runtime or test evidence proves the complete authorization and stored-outcome boundary.
- [ ] PLE uses the same basic interface for every Question Backend, each backend handles its own internal details.
  - Mismatch: `crates/question_model/src/question_library.rs` `QuestionBackend` is only an enum discriminator. Issuance and finalization branch separately on backend in `crates/server/src/assignment_delivery.rs` `issue_new_presentations` and `crates/server/src/assignment_delivery/direct_finalization.rs` `evaluate_one`; no common adapter interface covers every backend.
- [ ] Each Question Backend adapter retains its backend-specific interaction knowledge.
  - Mismatch: `crates/adapters/webwork/src/lib.rs` `WebworkAdapter` establishes an opaque WeBWorK boundary, and `crates/adapters/imathas/src/imathas_question_backend.rs` defines an iMathAS seam, but C870 leaves no current H5P adapter or runtime seam. Evidence from WeBWorK alone cannot establish this claim for each backend.
  - Question: Which H5P content type(s) are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [ ] Question Backends may support more complex interactions without requiring PLE to implement those interactions.
  - Mismatch: incomplete secondary backends leave the general capability unverified.

#### Question Backend grading and feedback

- [x] Question Backend feedback is transient unless the backend provides a robust way for PLE to preserve it.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` projects recorded native PLE feedback from the exact retained response and source, while the WeBWorK branch does not reconstruct or persist transient renderer feedback.
  - Evidence (runtime): the C910 isolated actual-HTTP proof exercised `crates/server/src/assessment_delivery/history.rs` `student_history`, stopping the renderer after issuance and then submitting and reading exact WeBWorK Revision history without a backend-feedback field.
- [x] PLE does not extract or reconstruct transient feedback from Question Backend source or output.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` invokes recorded teaching-content projection only for the native PLE source variant; the WeBWorK source remains opaque.
  - Evidence (runtime): the C910 actual-HTTP proof exercised `crates/server/src/assessment_delivery/history.rs` `student_history`; the history read succeeded after the renderer stopped and exposed no choice, correct, or incorrect feedback reconstructed from the PGML source or rendered output.
- [ ] PLE-managed Hints, Question Feedback, and Worked Solutions remain separate from backend-generated
  interaction feedback.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
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
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `question_revision_is_immutable` trigger protects revision rows.
  - Verification pending: reconcile the current immutable-revision Library reuse projection against this consolidated requirement.
- [x] Published Questions are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/question_library_operations.sql` `question_library_entries` requires an active Instructor Account and exposes available Question summaries.

#### Published Question identity specifications

- [x] Published Questions receive a public `AAAA-ZBBB` Crockford Base32 ID.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionId` defines and displays the canonical `AAAA-ZBBB` public Question ID; `crates/server/src/question_publication.rs` `NewQuestionLineagePublisher` issues it for a new Published Question lineage.
  - Evidence (runtime): `crates/server/src/question_publication.rs` `NewQuestionLineagePublisher` passed accepted actual-server proof that published two native Questions, whose exact public IDs then formed a reusable Pool's members. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Seven Crockford Base32 characters are cryptographically random and provide the identity.
  - Evidence (source): `crates/server/src/question_publication.rs` `question_id_from_random_bytes` derives the identifier from random bytes.
- [x] The middle character is an HMAC-derived check character calculated from the seven identity characters.
  - Evidence (source): `crates/server/src/question_publication.rs` `question_id_validation_character` derives the validation character with HMAC.
- [x] The check character detects mistyped or malformed IDs; it is not a security boundary.
  - Evidence (source): `crates/server/src/question_publication.rs` `validates_question_id` validates syntax/check character separately from authorization.
- [ ] ID generation enforces database uniqueness and retries when a random collision occurs.
  - Mismatch: database uniqueness exists, but collision retry behavior was not found in the issuer or publication store.
- [x] IDs never encode creation order, Question Type, ownership, subject, or other metadata.
  - Evidence (source): `crates/server/src/question_publication.rs` `question_id_from_random_bytes` uses random bytes and a secret only.

#### Published Question metadata

- [x] Published Questions have metadata specific to the individual Question.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question_metadata` keys individual metadata to `question_id` and requires nonempty `question_title` and `question_description` independently of Course placement.
- [x] Published Question metadata includes Title and Description.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question_metadata` keys individual metadata to `question_id` and requires nonempty `question_title` and `question_description` independently of Course placement.
- [x] Published Question metadata may include authorship, attribution, license, and source information.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `validate_question_publication` requires exact source, contiguous revision authorship and license records, keeping them associated with the Published Question Revision.
- [ ] Published Questions may include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Published Questions also use the shared Question Library metadata required for publication.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.

#### Published Question revisions, edits, and forks

- [x] **Published Questions** maintain immutable revision history.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `question_revision_is_immutable` trigger protects revision rows.
- [ ] Assessments and Student Work remain pinned to exact immutable Published Question Revisions.
  - Mismatch: exact revision columns are source evidence only; no connected test verifies an Assessment and Student Work stay pinned across a later publication.
- [x] Publishing a new Question Revision does not silently change existing Assessments or Student Work.
  - Evidence (source): `schemas/base_schema/question_publication_operations.sql` publication appends `next_revision_number` rather than rewriting prior rows.
- [x] The Question owner may publish corrections, wording changes, accessibility improvements, answer changes, grading changes, and other updates as a new Revision.
  - Evidence (source): `schemas/base_schema/question_publication_operations.sql` `publish_question_revision` appends an owner-authored revision.
- [ ] Changing Question source, answer content, grading rules, Hints, Question Feedback, Worked Solutions,
  or Question assets creates a new Question Revision.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [x] Changing the Question title, description, Tags, Subject, Topic, or other search metadata does not
  create a new Question Revision.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question_metadata` is separate from `question_revision`.
- [x] Search metadata belongs to the Published Question as a whole rather than to one Revision.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question_metadata` keys metadata to `question_id` only.
- [ ] Any **Instructor** may fork a Published Question to create a separate Question with a new Question ID.
  - Mismatch: draft-fork source support is source evidence only; no authorization or behavior test verifies any eligible Instructor can publish a separate ID.
- [x] A fork starts as a private **Draft Question** with its own authorship and lineage.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `draft_question_fork_source` records a private draft fork source.
- [x] A fork must pass Question Publication Validation before joining the Question Library.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `validate_question_publication` guards publication.
- [x] Published forks retain source attribution.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `question_fork_source` records published fork provenance.
- [x] Forced corrections are audited **Sysadmin** actions reserved for critical flaws.
  - Evidence (source): `schemas/base_schema/corrections.sql` `forced_question_correction` and its immutable audit targets record correction actions.
- [x] Question authorship, contributor credit, history, attribution, and compatible CC licensing are preserved across Revisions and forks.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `question_revision_authorship` and `question_revision_license` preserve revision stewardship.
- [ ] Watching a Published Question drives in-app notifications for new Revisions, forks, improvement
  threads, and impact notices.
  - Verification pending: `schemas/base_schema/question_stewardship.sql` `validate_question_publication` supplies Published-Question stewardship context only. Re-audit this exact Question/Pool obligation, including private identity/watch projection and all named notification kinds; existing Question-only evidence does not establish Pool scope.
  - Mismatch: source-bound Watch events exist for revisions and forks, but improvement threads and impact notices have no product-defined model or private delivery behavior.
  - Question: For improvement threads, who may create/read/reply/edit/resolve them, which identity/attachments/linkage/notification/retention rules apply; and for impact notices, who may create them, under what condition, with what text/category/severity/manual-or-derived/linkage/audience/update/cancel rules?

#### Published Question behavior specifications

- [ ] Published Questions may include optional PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
  - Owner: Question specifications > Draft Question specifications > Published Question specifications > Published Question metadata (first identical Human Guidance occurrence).
- [ ] PLE-managed Hints, Question Feedback, and Worked Solutions are separate from Question Backend-generated content.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] WeBWorK Questions may use PLE-managed Hints, Question Feedback, and Worked Solutions even when similar material also exists in the WeBWorK source.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Question Feedback is shown when its disclosure rules allow it.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Hints and Worked Solutions use their own disclosure settings.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Student workflows remain complete when a Question has none of this optional support content.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.

### Question Pool specifications

- [x] A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `create_question_pool` persists an ordered nonempty set of exact Published Question Revision members, and `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` selects from that Pool for Student delivery.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `start` passed accepted actual-server proof that selected an exact Pool member for Student Attempt 1, preserved it on resume, and selected again for Attempt 2. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Pool contents should represent reasonably interchangeable assessments of the intended learning.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `create_question_pool` requires the creating Instructor's true `interchangeability_attested` value; it does not substitute an automatic pedagogical evaluator.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted actual-main proof that required the Instructor's attestation before creating the ordered reusable Pool and before its later Assessment-owned reorder. Artifacts: `/private/tmp/ple-course-empty-artifacts.bzwXEa` and `/private/tmp/ple-course-empty-artifacts.lgyOMK`.
  - Evidence (runtime): `crates/server/src/question_pool_creation.rs` `create_question_pool` passed accepted actual-server proof that false or missing attestation returned 422 and left no Pool behind. Artifact: `/private/tmp/ple-course-empty-artifacts.hvS4KT`.
- [ ] Question Pools may contain Questions from any Question Backend.
  - Mismatch: C885 supplies backend-neutral Pool membership, but no completed Instructor Pool workflow proves this behavior.
- [x] Question Pools are always published and have no draft or unpublished state.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` model only a stable published lineage and immutable Revisions, with no draft, publication-status, or unpublished state.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted actual-main Instructor proof: it created a reusable Pool from two Published Questions and immediately read its server-issued Revision 1; the UI and API expose no draft or publish transition.
- [x] A Question Pool is an independently reusable Question Library object.
  - Evidence (source): `crates/server/src/question_pool_library.rs` `current_pool` reads a Pool independently of any Assessment.
  - Evidence (runtime): `crates/server/src/question_pool_library.rs` `current_pool` passed accepted actual-main Instructor proof: Pool `SBQR-N5RE` was created from the Question Library and its ordered member pins were read through `/api/question-pools/SBQR-N5RE`; separate actual-server proof then imported another reusable Pool into an Assessment.
- [x] Question Pools are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `list_published_question_pools` and `read_current_published_question_pool` authorize active Instructors and project only public Pool/Revision/member facts.
  - Evidence (runtime): `crates/server/src/question_pool_library.rs` `list_pools` passed accepted actual-server proof that a second vetted Instructor listed and read root Pool `1N6T-MZRD` and child Pool `J1BX-8V8F` with exact public member pins and no Course facts. A nonmember Assessment-fork PUT returned 404 without mutation; Student and anonymous Pool list/read calls returned no-store 404. Artifact: `/private/tmp/ple-course-empty-artifacts.hvS4KT`.
- [x] A Question Pool has its own public `AAAA-ZBBB` Crockford Base32 ID and immutable Revisions.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool` stores the unique compact public Pool ID, while `question_pool_revision` and `question_pool_revision_member` have immutable update/delete triggers and ordered exact member pins.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted actual-main proof that returned canonical Pool ID `SBQR-N5RE`, Revision 1, then read the same identity and exact ordered Question Revision pins.
- [x] Importing a Question Pool into a new Assessment automatically forks the Question Pool.
  - Evidence (source): `schemas/base_schema/assessment_pool_forks.sql` `import_assessment_question_pool_fork` atomically creates a fresh child Pool Revision and Assessment Entry from an exact reusable source Revision without accepting raw member pins.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `import_fork` passed accepted actual-server proof that imported source Pool `P8H3-QYX9` into a direct Assessment and returned distinct fork `VFH9-CQKS`, Revision 1, at Assessment Edit 2.
- [x] The fork belongs to the new Assessment and can be changed without changing the source Question Pool.
  - Evidence (source): `schemas/base_schema/assessments.sql` `assessment_question_pool_fork` owns each child Pool through exactly one Assessment Entry, and `schemas/base_schema/question_pools.sql` retains exact source-Revision provenance.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `append_fork_revision` passed accepted actual-server proof that appended the fork's Revision 2 with the two exact member pins reversed, then reread the reusable source unchanged at Revision 1 with its original order. Artifact: `/private/tmp/ple-course-empty-artifacts.BbKFFd`.
- [x] Forking a Question Pool preserves its Published Questions by their public `AAAA-ZBBB` IDs.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool_revision_member` pins each ordered public Question identity and Revision, and `import_assessment_question_pool_fork` copies those exact immutable source members.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `import_fork` passed accepted actual-server proof that returned both source Question IDs and Revision 1 pins unchanged and in order in the fresh fork; subsequent Student selection retained one exact member pin.
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
  - Evidence (source): `schemas/base_schema/assessment_attempt_operations.sql` `assessment_attempt_start_gate` returns an unfinished resumable Attempt before new issuance, while `crates/server/src/assessment_delivery.rs` `issue_native_assessment_batch` returns its retained committed presentations rather than selecting again.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `issue_native_assessment_batch` passed accepted actual-server proof that returned Attempt 1 with `resumed: true`, the same selected pin, and the same presentation nonce after its first start. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Starting a new Attempt makes fresh selections from its Question Pools.
  - Evidence (source): `schemas/base_schema/assessment_attempt_operations.sql` `assessment_attempt_start_gate` has no prior-Pool-selection reuse branch; after a submitted Attempt it authorizes a new Attempt, whose new selection payload is persisted by `start_assessment_attempt`.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `start` passed accepted actual-server proof that submitted Attempt 1, then started Attempt 2 with `resumed: false`, a distinct Pool selection ID, and a new presentation nonce. The same selected member remained valid with a two-member Pool. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Student Work preserves the exact Question Pool Revision and Published Question Revision delivered.
  - Evidence (source): `crates/question_model/src/student_work.rs` `QuestionPoolSelection` retains issued Question revision references.
  - Evidence (test): `crates/question_model/src/student_work/model_tests.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the issued revision link.
- [x] Grading and historical evidence follow the exact Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `read_student_assessment_attempt_history_response_sources` retains `question_id` and `revision_number`.
  - Evidence (test): `crates/question_model/src/student_work/model_tests.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the exact issued linkage.
- [x] Each member of a Question Pool is a **Published Question**.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool_revision_member` stores each exact Published Question revision reference.
- [ ] Question Pools contain only **Published Questions**; Question Pools cannot be members of Question Pools.
  - Verification pending: Source-contributor audit must confirm only exact Published Question Revision members and no Pool-member input; broad runtime evidence remains pending.
- [ ] Watching a Question Pool drives in-app notifications for new Revisions, forks, improvement
  threads, and impact notices.
  - Verification pending: `schemas/base_schema/question_stewardship.sql` `validate_question_publication` supplies Published-Question stewardship context only. Re-audit this exact Question/Pool obligation, including private identity/watch projection and all named notification kinds; existing Question-only evidence does not establish Pool scope.
  - Mismatch: source-bound Watch events exist for revisions and forks, but improvement threads and impact notices have no product-defined model or private delivery behavior.
  - Question: For improvement threads, who may create/read/reply/edit/resolve them, which identity/attachments/linkage/notification/retention rules apply; and for impact notices, who may create them, under what condition, with what text/category/severity/manual-or-derived/linkage/audience/update/cancel rules?

#### Question Pool metadata

- [ ] Question Pools have metadata specific to the individual Question Pool.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Question Pool metadata includes Title and Description.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Question Pools may have their own authorship, attribution, license, and source information where
  appropriate.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Question Pool metadata describes the Pool rather than duplicating metadata from its member
  Published Questions.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Question Pools may include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Question Pools also use the shared Question Library metadata required for publication.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.

### Question Library specifications

- [x] Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
  - Evidence (source): `src/pages/library_route_page.tsx` `LibraryRoutePage` is the production Instructor Library surface.
- [ ] The Question Library is one global collection of Published Questions and Question Pools.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Draft Questions are not part of the Question Library.
  - Evidence (source): `schemas/base_schema/question_library_operations.sql` `published_question_metadata` queries only Published Question metadata; Draft working state is stored separately in `schemas/base_schema/question_authoring_state.sql` `authoring_draft`.
  - Verification pending: re-evaluate the current Library search/Pool projections and publication boundary to establish explicit Draft exclusion across all Library paths.
  - Owner: Question specifications > Draft Question specifications (first identical Human Guidance occurrence).
- [ ] **Published Questions** and Question Pools are available to all vetted **Instructors**.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
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
  Tags, Subject, Topic, or other search fields together.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.
  - Mismatch: search, filters, and an accepted mock-transport browser metadata workflow exist, but connected HTTP and 13k practical-cleanup evidence remains pending.

#### Question Library metadata

- [ ] Published Questions and Question Pools use shared metadata for organization, search, filtering,
  and discovery.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Required Question Library metadata must be complete before content enters the Question Library.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Library metadata should describe the Published Question or Question Pool rather than its location
  in a Course or textbook.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Library classification uses **Discipline** -> **Subject** -> **Topic** -> **Subtopic** as its
  primary hierarchy.
  - Mismatch: `schemas/base_schema/question_lineages.sql` stores Question-only nullable Subject/Topic metadata; a managed Discipline vocabulary/lifecycle, Instructor selection, shared Pool classification, and Subtopic contract are not implemented.
- [ ] Discipline is the broad academic field, such as Biology, Chemistry, or Mathematics.
  - Mismatch: `schemas/base_schema/question_lineages.sql` stores Question-only nullable Subject/Topic metadata; a managed Discipline vocabulary/lifecycle, Instructor selection, shared Pool classification, and Subtopic contract are not implemented.
- [ ] Sysadmins exclusively manage the Discipline vocabulary and its lifecycle.
  - Mismatch: `schemas/base_schema/question_lineages.sql` stores Question-only nullable Subject/Topic metadata; a managed Discipline vocabulary/lifecycle, Instructor selection, shared Pool classification, and Subtopic contract are not implemented.
- [ ] Discipline is a stable vocabulary expected to change infrequently.
  - Mismatch: `schemas/base_schema/question_lineages.sql` stores Question-only nullable Subject/Topic metadata; a managed Discipline vocabulary/lifecycle, Instructor selection, shared Pool classification, and Subtopic contract are not implemented.
- [ ] Instructors classify Library objects by selecting from the Sysadmin-managed Disciplines.
  - Mismatch: `schemas/base_schema/question_lineages.sql` stores Question-only nullable Subject/Topic metadata; a managed Discipline vocabulary/lifecycle, Instructor selection, shared Pool classification, and Subtopic contract are not implemented.
- [ ] Subject identifies an area within a Discipline, such as Genetics, Biochemistry, or Ecology.
  - Mismatch: `schemas/base_schema/question_lineages.sql` stores Question-only nullable Subject/Topic metadata; a managed Discipline vocabulary/lifecycle, Instructor selection, shared Pool classification, and Subtopic contract are not implemented.
- [ ] Sysadmins can edit Subjects.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Topic identifies a major area within the Subject.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Subtopic provides a narrower classification within the Topic.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Subject, Topic, and Subtopic names must satisfy length limits and formatting requirements.
  - Mismatch: `schemas/base_schema/question_lineages.sql` has bounded trimmed Subject/Topic checks only; consistent Subject/Topic/Subtopic validation, progressive allowances, and trim-before-validation are not established.
- [ ] Length allowances should generally increase from Subject to Topic to Subtopic, supporting more
  specific names as classification becomes narrower.
  - Mismatch: `schemas/base_schema/question_lineages.sql` has bounded trimmed Subject/Topic checks only; consistent Subject/Topic/Subtopic validation, progressive allowances, and trim-before-validation are not established.
- [ ] Strip leading and trailing whitespace from Subject, Topic, and Subtopic names and validate the
  resulting names consistently.
  - Mismatch: `schemas/base_schema/question_lineages.sql` has bounded trimmed Subject/Topic checks only; consistent Subject/Topic/Subtopic validation, progressive allowances, and trim-before-validation are not established.
- [ ] Discipline, Subject, Topic, and Subtopic should support consistent classification across the
  Question Library.
  - Mismatch: `schemas/base_schema/question_lineages.sql` stores Question-only nullable Subject/Topic metadata; a managed Discipline vocabulary/lifecycle, Instructor selection, shared Pool classification, and Subtopic contract are not implemented.
- [ ] Published Questions and Question Pools may also have Tags for useful classifications outside the
  Discipline, Subject, Topic, and Subtopic hierarchy.
  - Mismatch: `schemas/base_schema/question_lineages.sql` stores Question-only nullable Subject/Topic metadata; a managed Discipline vocabulary/lifecycle, Instructor selection, shared Pool classification, and Subtopic contract are not implemented.
- [ ] Tags are flexible and may overlap across Disciplines, Subjects, and Topics.
  - Mismatch: `schemas/base_schema/question_lineages.sql` stores Question-only nullable Subject/Topic metadata; a managed Discipline vocabulary/lifecycle, Instructor selection, shared Pool classification, and Subtopic contract are not implemented.
- [ ] Library metadata should support searching, filtering, sorting, and bulk editing.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` has Question Title/Description, Tags and nullable Subject/Topic, but no Subtopic hierarchy; `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` provide identity/member pins without the shared required Library metadata/support model. Audit the exact requirement; Question-only fields do not establish the expanded Pool/publication scope.
- [ ] Published Questions and Question Pools may have PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.
- [ ] Support content may be attached at the level where it applies rather than duplicated across
  individual Questions.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `question_revision` stores optional `general_feedback`, and accepted C910 proof covers that narrow feedback path only. Independent PLE-managed Hints/Worked Solutions, Pool-level support, disclosure controls, and revision/coexistence behavior required here are not fully implemented or proved.

#### Question Library object statistics

- [ ] Published Questions and Question Pools may retain privacy-safe aggregate statistics.
  - Verification pending: `schemas/base_schema/statistics.sql` `question_revision_statistics` supplies Question-only aggregate context; this requirement now also applies to Pool Revisions/use/selection or revised privacy/retention semantics. Audit the exact aggregate model and privacy/retention oracle; Question-only evidence is insufficient.
- [ ] Statistics are kept separately for each Published Question Revision and Question Pool Revision.
  - Verification pending: `schemas/base_schema/statistics.sql` `question_revision_statistics` supplies Question-only aggregate context; this requirement now also applies to Pool Revisions/use/selection or revised privacy/retention semantics. Audit the exact aggregate model and privacy/retention oracle; Question-only evidence is insufficient.
- [ ] Each Published Question Revision may retain aggregate counts of correct, incorrect, partial-credit,
  and unanswered results.
  - Mismatch: private aggregate capture exists, but no released Question Statistics surface establishes this product behavior.
- [x] Eligible Question Types may also retain aggregate answer-choice counts.
  - Evidence (source): `schemas/base_schema/statistics.sql` `selected_count` stores aggregate choice counts.
  - Owner: Data and history > Human-facing reference IDs > Student and FERPA data (first identical Human Guidance occurrence).
- [ ] Each Question Pool Revision may retain aggregate statistics for its use and Question selections.
  - Verification pending: `schemas/base_schema/statistics.sql` `question_revision_statistics` supplies Question-only aggregate context; this requirement now also applies to Pool Revisions/use/selection or revised privacy/retention semantics. Audit the exact aggregate model and privacy/retention oracle; Question-only evidence is insufficient.
- [ ] Published Question and Question Pool statistics may combine Revisions when clearly labeled and
  privacy thresholds are met.
  - Verification pending: `schemas/base_schema/statistics.sql` `question_revision_statistics` supplies Question-only aggregate context; this requirement now also applies to Pool Revisions/use/selection or revised privacy/retention semantics. Audit the exact aggregate model and privacy/retention oracle; Question-only evidence is insufficient.
- [ ] Aggregate statistics contain counts rather than Student Attempts or identifiable Student records.
  - Mismatch: private aggregate evidence exists, but no released Question Statistics surface establishes the required product behavior.
- [ ] Privacy-safe aggregate statistics remain after the underlying Student records are deleted.
  - Mismatch: retention transition needs runtime or connected-oracle proof.
- [ ] Student data retention removes the underlying Student evidence without removing approved aggregate
  statistics.
  - Mismatch: needs runtime or connected-oracle evidence for retention and aggregate preservation.
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
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` and `schemas/base_schema/question_pools.sql` `question_pool_revision` have no two-dimensional Bloom model; publication, AI assignment, correction without a Revision, and search/reporting proof remain absent for this requirement.
- [ ] The two Bloom dimensions are independent and together determine the object's Bloom Classification.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` and `schemas/base_schema/question_pools.sql` `question_pool_revision` have no two-dimensional Bloom model; publication, AI assignment, correction without a Revision, and search/reporting proof remain absent for this requirement.
- [ ] Bloom Classification describes the cognitive work required for full credit, not Question Difficulty.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` and `schemas/base_schema/question_pools.sql` `question_pool_revision` have no two-dimensional Bloom model; publication, AI assignment, correction without a Revision, and search/reporting proof remain absent for this requirement.
- [ ] A Question Pool's Bloom Classification describes the intended cognitive work of the Pool as a whole.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` and `schemas/base_schema/question_pools.sql` `question_pool_revision` have no two-dimensional Bloom model; publication, AI assignment, correction without a Revision, and search/reporting proof remain absent for this requirement.
- [ ] Bloom Classification is required before a Published Question or Question Pool enters the Question
  Library.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` and `schemas/base_schema/question_pools.sql` `question_pool_revision` have no two-dimensional Bloom model; publication, AI assignment, correction without a Revision, and search/reporting proof remain absent for this requirement.
- [ ] AI assigns the initial Bloom Classification as part of publication.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` and `schemas/base_schema/question_pools.sql` `question_pool_revision` have no two-dimensional Bloom model; publication, AI assignment, correction without a Revision, and search/reporting proof remain absent for this requirement.
- [ ] An **Instructor** can correct either Bloom dimension without creating a new Published Question or
  Question Pool Revision.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` and `schemas/base_schema/question_pools.sql` `question_pool_revision` have no two-dimensional Bloom model; publication, AI assignment, correction without a Revision, and search/reporting proof remain absent for this requirement.
- [ ] Question Library search and reporting should make both Bloom dimensions useful to **Instructors**.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` and `schemas/base_schema/question_pools.sql` `question_pool_revision` have no two-dimensional Bloom model; publication, AI assignment, correction without a Revision, and search/reporting proof remain absent for this requirement.
- [ ] Follow `docs/BLOOM_TAXONOMY_GUIDE.md` for Bloom classification and teaching interpretation.
  - Mismatch: `schemas/base_schema/question_lineages.sql` `published_question_metadata` and `schemas/base_schema/question_pools.sql` `question_pool_revision` have no two-dimensional Bloom model; publication, AI assignment, correction without a Revision, and search/reporting proof remain absent for this requirement.

#### Question Library stewardship specifications

- [ ] Question Library stewardship should use a GitHub-like model.
  - Verification pending: `schemas/base_schema/question_stewardship.sql` `validate_question_publication` supplies Published-Question stewardship context only. Re-audit this exact Question/Pool obligation, including private identity/watch projection and all named notification kinds; existing Question-only evidence does not establish Pool scope.
- [ ] Published Questions and Question Pools can be starred and watched.
  - Verification pending: `schemas/base_schema/question_stewardship.sql` `validate_question_publication` supplies Published-Question stewardship context only. Re-audit this exact Question/Pool obligation, including private identity/watch projection and all named notification kinds; existing Question-only evidence does not establish Pool scope.
- [x] Star means favorite and visible endorsement.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `set_current_question_star` records an active Instructor's Star only for a Published Question; `src/components/question_star_control.tsx` `QuestionStarControl` provides the visible Star and count surface.
  - Evidence (test): `tests/e2e/e2e_question_star_name_privacy.sh` `Question Star name privacy E2E` passed on 2026-09-15 with an active vetted Instructor's actual HTTP Star action and exact closed Star projection.
- [ ] Vetted **Instructors** can see the star count and which vetted **Instructors** starred a Published
  Question or Question Pool.
  - Verification pending: `schemas/base_schema/question_stewardship.sql` `validate_question_publication` supplies Published-Question stewardship context only. Re-audit this exact Question/Pool obligation, including private identity/watch projection and all named notification kinds; existing Question-only evidence does not establish Pool scope.
- [ ] Watch means subscription.
  - Mismatch: Question watches are not implemented.
- [ ] Watching a Published Question or Question Pool drives in-app notifications for new Revisions,
  forks, improvement threads, and impact notices.
  - Verification pending: `schemas/base_schema/question_stewardship.sql` `validate_question_publication` supplies Published-Question stewardship context only. Re-audit this exact Question/Pool obligation, including private identity/watch projection and all named notification kinds; existing Question-only evidence does not establish Pool scope.
  - Mismatch: source-bound Watch events exist for revisions and forks, but improvement threads and impact notices have no product-defined model or private delivery behavior.
  - Question: For improvement threads, who may create/read/reply/edit/resolve them, which identity/attachments/linkage/notification/retention rules apply; and for impact notices, who may create them, under what condition, with what text/category/severity/manual-or-derived/linkage/audience/update/cancel rules?
- [ ] An **Instructor's** watch list remains private.
  - Mismatch: Question watches are not implemented.
- [ ] **Students** and anonymous users do not receive **Instructor** identity lists or watch information.
  - Mismatch: Question watch access controls are not implemented.
