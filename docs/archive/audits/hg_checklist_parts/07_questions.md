## Question specifications
- [x] Questions are subject agnostic. Properly classified Published Questions from all subjects belong in
  the same Question Library.
  - Evidence (source): `crates/server/src/question_library/paging.rs` `QuestionSearchFilter` supplies the shared Library query filter without a subject partition.

- [x] Questions are strictly and deterministically automated; grading does not require an **Instructor**.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `record_direct_automated_grading_result` stores backend credit with no Instructor argument, and `reject_grading_evidence_change` keeps that result immutable.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` grades the compiled answer key and takes no Instructor.
  - Evidence (source): `crates/adapters/webwork/src/lib/grade.rs` `grade` sends the seeded Student response to the renderer and takes no Instructor.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `questions_are_strictly_and_deterministically_automated_grading_does_not_require_an_instructor` graded blue twice at credit 1 and red twice at credit 0.
  - Evidence (test): `crates/adapters/webwork/src/http_renderer/tests.rs` `grade_forwards_ordered_pairs_once_with_trusted_fields` mapped renderer scores 0, 0.5, and 1 to those credits with `isInstructor=0` and problem seed 7.

- [x] Questions have one canonical title. Compact interfaces may truncate that title.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question_metadata` stores one lineage-level title.

- [x] Every Question stored by PLE has its own internal Question record.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question` owns the internal Question record.

- [x] Answer-choice randomization belongs to the Question.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_compile.rs` `compile_choices` maps the Question document randomizeChoices flag to NonceRandomized or Fixed for single-choice and multiple-answer responses.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `AssessmentActivityRules` stores authored or shuffled Question order in assessment_question_order_rule and has no answer-choice field.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` states that answer-choice order is configured on each Question.
  - Evidence (test): `crates/adapters/ple/src/question_json/tests.rs` `choice_randomization_is_choice_owned_and_defaults_when_omitted` compiled a single-choice document with randomizeChoices true to NonceRandomized and rejected that field on fill-in.
  - Evidence (test): `tests/test_ple_question_json_editor_model.mjs` `setChoiceRandomization` set randomizeChoices true and kept choice ids choice_a and choice_b.
  - Evidence (test): `tests/test_ple_question_json_multiple_answer_editor.mjs` `setMultipleAnswerChoiceRandomization` set randomizeChoices true and kept correct ids kinase and enzyme.

- [x] PLE-native Questions control their own answer-choice randomization.
  - Evidence (source): `crates/question_model/src/presentation/builder_items.rs` `pending_items` permutes multiple-choice items only when native_choice_order is NonceRandomized.
  - Evidence (source): `crates/question_model/src/presentation/choice_order.rs` `nonce_randomized_choices` ranks choices by the issued nonce and each stable choice id.
  - Evidence (test): `crates/question_model/src/presentation/tests.rs` `nonce_randomized_native_choices_reproduce_the_issued_binding_order` issued carboxyl, amine, then hydroxyl for nonce 0x31 and kept that order after the authored vector was reversed.

### Draft Question specifications
- [x] Draft Questions are private working content.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `ple_private.draft_question` stores draft state in the private schema.

- [x] Draft Questions are not part of the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns accepted Published Question Revisions and does not read Draft Questions.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` searches those same Published Question Revisions.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `list_authoring_drafts` returns the Instructor's Draft Questions outside the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns published Question Pools and does not read Draft Questions.
  - Owner: 07_questions.md / Draft Question specifications (first occurrence; identical requirement and status).

- [x] Draft Questions use current state rather than immutable Revisions.
  - Evidence (source): `schemas/base_schema/20_tables/question_authoring.sql` `draft_question` stores one current Draft by Edit Number and has no Revision column.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `save_authoring_draft` updates that current Draft in place and does not insert a Question Revision.

- [x] Saving a Draft Question replaces its previous working state.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `save_authoring_draft` replaces the current draft aggregate values.

- [x] **Instructors** may delete Draft Questions they no longer need.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `delete_draft_question` resolves only the current Instructor-owned Draft, locks and compares its Edit Number, then deletes that private aggregate without considering the separate Published Question lineage.
  - Evidence (source): `crates/learning-data-access/src/postgres/authoring.rs` `delete_authoring_draft` carries the SQL compare-and-swap through the authenticated Store.
  - Evidence (source): `crates/server/src/authoring.rs` `delete_draft` requires the parsed `If-Match` Edit Number and maps a concurrent change to 412; `src/pages/question_drafts_page.tsx` `QuestionDraftsPage` supplies explicit Keep/Delete confirmation.
  - Evidence (runtime): `crates/server/src/authoring.rs` `delete_draft` passed accepted isolated PostgreSQL/MinIO actual-server and focused browser proof: cancel, confirm, and list reload; valid-current-ETag collaborator, unrelated Instructor, Student, Sysadmin, and anonymous 404 denials while owner source/Edit Number remained unchanged; 428 missing, 400 malformed, and 412 stale preconditions; preserved parsed Published Question lineage and Revision JSON after a published-origin Draft deletion; and 404 repeat DELETE/PUT. Artifact: `/private/tmp/ple-draft-delete-artifacts.km9ybM`.

- N/A PLE may clean up abandoned Draft Questions after an appropriate warning and recovery period.
  - Reason: Automated abandoned-Draft cleanup is an explicitly optional future capability; HG sets no clock or durations.

- [x] A Draft Question must pass Question Publication Validation before becoming a Published Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` is the ordinary Draft publication path. It refuses a missing Discipline, Subject, license, or reviewed authorship before any insert, writes acceptance, authorship, license, source binding, and owner, then inserts the publication event. A fork-source license check runs only when that Draft has a fork source.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `validate_question_publication` rolls the publication transaction back when acceptance, contiguous authorship, license, exact source, or owner is missing.
  - Evidence (source): `crates/server/src/authoring.rs` `publish_draft` refuses a Draft whose source has no Question License before it builds the publication command.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `A Draft Question must pass Question Publication Validation before becoming a Published Question` posted a complete ordinary publication to `/publish` and withheld the request when Discipline, Subject, or reviewed authorship was missing.

- [x] Question Publication Validation requires Discipline, Subject, and all other required Question
  Library metadata before publication.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a missing Discipline or Subject and copies Title and Description from the Draft.
  - Evidence (source): `schemas/base_schema/20_tables/question_authoring.sql` `draft_question_metadata` requires Title and Description before publication copies them.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` requires Title, Description, one Discipline, and a Subject that belongs to that Discipline.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `publication sends shared Question Library metadata and refuses a missing Discipline or Subject` rejected a blank Title and Description and withheld publication when Discipline or Subject was omitted.

### Question formats and type specifications
- [x] PLE flat-question JSON is the canonical machine format for simple static Questions.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` validates the PLE JSON source form.

- [x] QTI is for import, export, and archival interchange rather than the internal source model.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `workspace_import` treats `qti` as an import format, not a source binding.

- [x] A QTI ZIP, retained QTI archive, and extracted QTI image are interchange roles, not Question Image
  Assets.
  - Evidence (source): `crates/adapters/qti/src/model.rs` `QtiPackageArchive` and `QtiPackageExtractedImage` are import-only; `question_image_asset_id()` is derived at Question bind.

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
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_attempts.sql` `validate_issued_question_reproduction` passed in `/private/tmp/ple-native-seed-proof.sh --isolated --native-seed-http` against PostgreSQL: shuffled-position-2 native seed/hash were null, real WeBWorK retained numeric seed/64-character hash privately, public start/read/save/resume/restored payloads omitted both fields, resume retained the same issued Questions and saved native response, and invalid native seed insertion failed. Artifact: `/private/tmp/ple-native-seed-artifacts.KfY7Op`.

- [x] Native PLE JSON supports MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonResponse` defines all eight native types.
  - Evidence (runtime): `tests/playwright/screenshot_corpus/scenarios_student_types.ts` `captureTypes` supplied current authorized Student delivery of each released native type at laptop and phone widths with exact published Revision checks. The 16 native captures establish presentation only; response interaction, save/reload, and grading remain separately scoped per type. Receipt: `/private/tmp/ple-resumed-types-20260916.md`.

- [x] External URLs used by native JSON Questions are explicitly recorded and reviewable.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` records the author-declared `externalResources` inventory as source metadata only, without fetching or browser permission; `validate_external_resources` bounds and de-duplicates recorded URLs.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `validate_external_resource_url` accepts only bounded, printable, absolute HTTPS URLs without user information.
  - Decision: A one-time parser proof accepted all five then-supported resource kinds and legacy omission, while rejecting invalid and duplicate URLs; remote script resources have since been removed from the closed kind set.

- [x] Recorded external URLs include links, images, stylesheets, and other resources.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonExternalResourceKind` is the closed Link, Image, Stylesheet, and Other category set for every `externalResources` entry; external script syntax is not supported.
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

- [x] Question response layouts may adapt to available screen space while preserving the same content,
  response meaning, and grading behavior. Narrow layouts may repeat choices when that improves use.
  - Evidence (source): `src/components/question_response_control_styles.ts` `matching-layout` places the shared choice bank beside the prompts when the row is wide and stacks that same bank when the row is narrow.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` grades the saved Student response.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `Question response layouts may adapt to available screen space while preserving the same content, response meaning, and grading behavior` kept both prompts and both choices once, stacked the bank at a narrow width, and saved the same matches as the wide width.

- [x] MATCH Questions should make each prompt's assigned choice easy to recognize and keep the choice
  bank reachable while Students assign, change, and clear matches using keyboard, pointer, or touch.
  - Evidence (source): `src/components/question_response_controls/matching.tsx` `MatchingResponse` shows the assigned choice wording in the prompt slot and keeps every choice in the bank.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `MATCH Questions should make each prompt's assigned choice easy to recognize and keep the choice bank reachable while Students assign, change, and clear matches using keyboard, pointer, or touch` assigned, changed, and cleared a match with pointer, keyboard, and touch while the bank stayed visible and the slot showed the choice wording.

#### Native PLE JSON Questions and JavaScript
- [x] Native JSON Questions may contain author-supplied JavaScript, including chemistry content using RDKit.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document/author_script.rs` `PleQuestionJsonAuthorScript` records the author source and the closed rdkit library without running JavaScript.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_codec.ts` `decodeAuthorScript` accepts that source and the rdkit library name.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `grading_and_correctness_decisions_remain_server_owned_and_independent_of_author_supplied_javascript` stored RDKit.get_mol("CCO") with library rdkit, issued that source, graded blue at credit 1, and graded red at credit 0.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `author script metadata stays closed and preserved without becoming an execution path` preserved an author script and the rdkit library and refused an unknown library.

- [x] Author-supplied JavaScript may provide client-side rendering or interaction without access to a random seed.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `author_content_document_response` installs the author source as a client script and does not receive a question seed.
  - Evidence (source): `src/components/author_content_frame.tsx` `AuthorContentFrame` loads that document for the issued Question position.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_may_provide_client_side_rendering_or_interaction_without_access_to_a_random_seed` built the client document from author source, kept the script bytes and author-content root, and left question-seed-9f3c out of the document and headers.

- [x] Author-supplied JavaScript runs in an isolated browser environment.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `author_content_document_response` serves the author document with a script sandbox, `default-src 'none'`, and no `allow-same-origin`.
  - Evidence (source): `src/components/author_content_frame.tsx` `AuthorContentFrame` loads that document in an iframe whose sandbox is `allow-scripts` and whose referrer policy is `no-referrer`.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_runs_in_an_isolated_browser_environment` built the client document, required `sandbox allow-scripts` without `allow-same-origin`, and kept the author script bytes in that document.

- [x] Author-supplied JavaScript is treated as untrusted content.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `author_content_document_response` carries the author source as base64 into a script text node.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_codec.ts` `decodeAuthorScript` accepts only source and libraries.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_is_treated_as_untrusted_content` built a document whose source was a script breakout and kept that text out of HTML parsing.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `author script metadata stays closed and preserved without becoming an execution path` preserved an author script and refused an execute field.

- [x] Author-supplied JavaScript is isolated from PLE application state, credentials, and privileged browser context.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `author_content_document_response` serves the author document with `sandbox allow-scripts` and `connect-src 'none'`, and sets no cookie or authorization header.
  - Evidence (source): `src/components/author_content_frame.tsx` `AuthorContentFrame` loads that document in an iframe whose sandbox is `allow-scripts`, whose permissions policy is empty, and whose source is only the attempt and position URL.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_is_isolated_from_ple_application_state_credentials_and_privileged_browser_context` built the client document, required `sandbox allow-scripts` and `connect-src 'none'`, and left cookie, authorization, storage APIs, and the session credential out of the document.
  - Evidence (test): `tests/test_assessment_attempt_navigation.mjs` `Author-supplied JavaScript is isolated from PLE application state, credentials, and privileged browser context` rendered the shipped frame with sandbox `allow-scripts`, an empty permissions policy, no-referrer, and an attempt/position document URL that omitted the session credential.

- [x] Author-supplied JavaScript is limited to client-side rendering and interaction.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `document_csp` limits the no-library author document to `sandbox allow-scripts` with `connect-src 'none'`, `form-action 'none'`, `frame-src 'none'`, and `worker-src 'none'`.
  - Evidence (source): `src/components/author_content_frame.tsx` `AuthorContentFrame` grants that document only `sandbox="allow-scripts"` and an empty permissions policy.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_is_limited_to_client_side_rendering_and_interaction` built the client document, required the script sandbox and the none-source directives, and left fetch, XHR, WebSocket, and application paths out of the document.
  - Evidence (test): `tests/test_assessment_attempt_navigation.mjs` `Author-supplied JavaScript is limited to client-side rendering and interaction` rendered the shipped frame with sandbox exactly `allow-scripts` and without form, popup, navigation, download, or modal tokens.

- [x] Author-supplied JavaScript operates independently of PLE application APIs and privileged state.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `author_content_document_response` builds the author document from the author source and parent origin, with no API client or session.
  - Evidence (source): `src/components/author_content_frame.tsx` `AuthorContentFrame` reads only the document URL from the application client.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_operates_independently_of_ple_application_apis_and_privileged_state` built the document from author source alone, kept `connect-src 'none'`, and left application paths, the session credential, and storage APIs out of the document.
  - Evidence (test): `tests/test_assessment_attempt_navigation.mjs` `Author-supplied JavaScript operates independently of PLE application APIs and privileged state` rendered the shipped frame from a client that exposes only the document URL and left the session credential out of the frame.

- [x] Native interactive Question Types such as HOTSPOT use PLE-owned interaction code.
  - Evidence (source): `src/components/question_response_controls/question_response_control.tsx` `QuestionResponseControl` dispatches a delivered `hotspot` format to `HotspotResponse`; `src/components/question_response_controls/hotspot.tsx` `HotspotResponse` owns the image overlay, labeled native region controls, response serialization, and Save handoff.
  - Evidence (runtime): `tests/playwright/screenshot_corpus/hotspot_workflow.ts` `exerciseHotspot` passed unchanged for Avery's pointer input and Jack's keyboard Space input: each selected the PLE-owned region, saved, reloaded the exact issued Question ID and Revision with the selection intact, submitted the whole Attempt, and received `Marked correct.` from server grading. Receipt: `/private/tmp/ple-hotspot-connected-interaction-20260916.md`.

- [x] HOTSPOT content uses supported still images and SVG.
  - Evidence (source): `crates/server/src/draft_question_images.rs` `upload` accepts image/svg+xml and stores the WebP from prepare_question_image.
  - Evidence (test): `crates/objects/src/image_validation/svg_question_image.rs` `hotspot_content_uses_supported_still_images_and_svg` rewrote a red SVG into a 12 by 8 WebP, left a local file and a remote image unpainted, refused SVG text so labels cannot disappear, and refused DOCTYPE, entity, gzip, and a pixel flood.

- [x] Grading and correctness decisions remain server-owned and independent of author-supplied JavaScript.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` grades the compiled private answer key and does not pass author content into grading.
  - Evidence (source): `crates/grading/src/ple_question_json.rs` `evaluate` scores the response from the server answer key.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `grading_and_correctness_decisions_remain_server_owned_and_independent_of_author_supplied_javascript` issued an author script that returns red, kept that source and the rdkit library on the presentation, graded blue at credit 1, and graded red at credit 0.

- [x] Supported author JavaScript libraries are explicitly recorded and served by PLE.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document/recorded_javascript.rs` and `src/features/ple_question_json_authoring/recorded_javascript_dependencies.ts` retain the closed `rdkit` library registry; external script resources and CDN allowlists are absent.
  - Evidence (test): `crates/adapters/ple/src/question_json/tests.rs` `supported_author_javascript_libraries_are_explicitly_recorded_and_reviewable` parses an author Question that declares rdkit.

- N/A Approved external dependencies may initially load from recorded CDN sources.
  - Reason: Author Questions do not support remote script resources; the recorded rdkit dependency is served from PLE.

- [x] Supported external dependencies should eventually become PLE-owned and served locally.
  - Evidence (source): `crates/server/src/author_content_dependency_assets.rs` `author_content_dependency_asset_router` serves the current RDKit JavaScript and WASM from compiled-in local bytes at fixed PLE routes.
  - Evidence (source): `crates/server/src/author_content_dependency_registry_generated.rs` `RDKIT_FILES` records those two local files and names no CDN URL.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `document_csp` points an rdkit author document at that local WASM route and no other network host.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `supported_external_dependencies_should_eventually_become_ple_owned_and_served_locally` built the rdkit document, required the local JavaScript and WASM routes, rejected CDN hosts, and received both files from the PLE router with no redirect.
  - Evidence (test): `crates/server/src/author_content_dependency_assets.rs` `current_rdkit_assets_are_byte_exact_and_have_only_their_public_contract` served those routes and matched the compiled-in JavaScript and WASM bytes.

### Question Backend specifications
#### Supported Question Backends
- [x] WeBWorK is a PLE-managed Question Backend.
  - Evidence (source): `crates/adapters/webwork/src/lib.rs` `WebworkAdapter` is the current PLE-managed WeBWorK integration boundary.

- [x] The initial primary Question Backends are PLE-native JSON and WeBWorK.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_presentation.sql` `backend IN ('ple', 'webwork')` is the delivered presentation boundary.

- N/A iMathAS and H5P are desired secondary Question Backends governed by Deferred product behavior.
  - Reason: Human Guidance explicitly defers both Backends, so they are desired product behavior rather than current implementation requirements. Current production Backends are PLE and WeBWorK.

- [x] PLE-native Questions use the PLE Question Backend.
  - Evidence (source): `schemas/base_schema/15_table_check_functions.sql` `question_source_binding_fields_are_valid` accepts native pleQuestionJson only with the ple Question Backend.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionBackend` names Ple as that first-party Question Backend.

- [x] WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
  - Evidence (source): `crates/adapters/webwork/src/http_renderer/client.rs` `render` keeps the renderer HTML document, and `grade` returns only the renderer score.
  - Evidence (test): `crates/adapters/webwork/src/http_renderer/tests.rs` `webwork_owns_pg_pgml_rendering_controls_evaluators_partial_credit_and_feedback` forwarded PGML source unchanged, kept the renderer form control and feedback HTML, and graded the renderer score 0.5 as partial credit. No Live Demo stack was started. No PostgreSQL proof was run.

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

- [x] Question Backends own rendering, interaction, response, grading, feedback, and backend-specific state.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `issue_question_json` renders the native prompt, choice control, and backend state, and `grade_question_json` grades the captured response.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `native_question_backend_owns_rendering_interaction_response_grading_feedback_and_state` issued the favorite-color prompt and choice control, graded blue at credit 1 and red at credit 0, projected the native incorrect feedback, and kept PLE backend state. `crates/adapters/webwork/src/http_renderer/tests.rs` `webwork_owns_pg_pgml_rendering_controls_evaluators_partial_credit_and_feedback` kept the renderer document, form control, answer field, partial-credit score, and feedback HTML. iMathAS is not a current production backend. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] PLE owns authorization, Question ID, revisions, persistence, lifecycle, and stored outcomes.
  - Evidence (source): `crates/server/src/assessment_delivery.rs` `student_with_sessions` admits only a Student session before finalization preparation.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `prepare_assessment_attempt_finalization` selects the published question id, revision number, and database-derived finalization kind.
  - Evidence (source): `crates/objects/src/question_source.rs` `resolve` reads source bytes only for that exact Question Revision tuple.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `commit_assessment_attempt_finalization` stores the returned credit fraction only when the prepared finalization kind is still current.
  - Evidence (test): `crates/server/src/assessment_delivery/direct_finalization.rs` `ple_owns_authorization_question_id_revisions_persistence_lifecycle_and_stored_outcomes` concealed an Instructor before preparation, then posted the shipped submission route for a Student. The handler graded revision 4 of the prepared Question ID, committed Deadline with normalized credit 1, and opened zero renderer connections. No Live Demo stack was started. No PostgreSQL proof was run.

- [ ] PLE uses the same basic interface for every Question Backend, each backend handles its own internal details.
  - Mismatch: `crates/question_model/src/question_library.rs` `QuestionBackend` is only an enum discriminator. Issuance and finalization branch separately on backend in `crates/server/src/assignment_delivery.rs` `issue_new_presentations` and `crates/server/src/assignment_delivery/direct_finalization.rs` `evaluate_one`; no common adapter interface covers every backend.

- [x] Each Question Backend adapter retains its backend-specific interaction knowledge.
  - Evidence (source): `crates/adapters/webwork/src/lib/grade.rs` `pg_source` sends the Question's PG source, PG path, and backend-owned response payload to the renderer.
  - Evidence (source): `crates/adapters/webwork/src/lib.rs` `WebworkAdapter` keeps that WeBWorK interaction behind the adapter facade. Deferred iMathAS and H5P behavior is outside this production adapter.
  - Evidence (test): `crates/adapters/webwork/src/lib/tests.rs` `opaque_lifecycle_issues_and_grades_one_backend_owned_payload` issued one document and graded the opaque payload only after the shipped adapter passed the PG source, PG path, and payload through. `opaque_grading_refuses_native_ple_responses_without_a_renderer_call` refused a native PLE response before any renderer call. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Question Backends may support more complex interactions without requiring PLE to implement those interactions.
  - Evidence (source): `crates/adapters/webwork/src/lib/issue.rs` `QuestionResponseFormat::BackendOwned` issues an empty PLE prompt and a backend-owned response, so the PG form stays with the renderer.
  - Evidence (test): `crates/adapters/webwork/src/lib/tests.rs` `opaque_lifecycle_issues_and_grades_one_backend_owned_payload` checked that empty prompt and backend-owned response, then graded the opaque answer payload through the shipped adapter. Deferred iMathAS and H5P behavior stays outside this production backend. No Live Demo stack was started. No PostgreSQL proof was run.

#### Question Backend grading and feedback
- [x] Question Backend feedback is transient unless the backend provides a robust way for PLE to preserve it.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` projects recorded native PLE feedback from the exact retained response and source, while the WeBWorK branch does not reconstruct or persist transient renderer feedback.
  - Evidence (runtime): the C910 isolated actual-HTTP proof exercised `crates/server/src/assessment_delivery/history.rs` `student_history`, stopping the renderer after issuance and then submitting and reading exact WeBWorK Revision history without a backend-feedback field.

- [x] PLE does not extract or reconstruct transient feedback from Question Backend source or output.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` invokes recorded teaching-content projection only for the native PLE source variant; the WeBWorK source remains opaque.
  - Evidence (runtime): the C910 actual-HTTP proof exercised `crates/server/src/assessment_delivery/history.rs` `student_history`; the history read succeeded after the renderer stopped and exposed no choice, correct, or incorrect feedback reconstructed from the PGML source or rendered output.

- [x] PLE-managed Hints, Question Feedback, and Worked Solutions remain separate from backend-generated
  interaction feedback.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns Revision Question Feedback, Hints, and Worked Solutions before backend teaching projection.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_teaching_feedback` assigns backend choice, correct, and incorrect feedback without writing Question Feedback, Hints, or Worked Solutions.
  - Evidence (test): `crates/server/src/assessment_delivery/history.rs` `ple_managed_support_stays_separate_from_backend_interaction_feedback` kept the PLE Hint, Question Feedback, and Worked Solution after backend choice, correct, and incorrect notes were applied. Those backend notes stayed out of the three PLE-managed fields. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] A Question Backend returns an immutable credit fraction for each complete response it evaluates.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` returns the compiled evaluation for one complete response.
  - Evidence (source): `crates/question_model/src/student_work/grading.rs` `QuestionEvaluation` keeps that credit fraction as the evaluation fact.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `backend_returned_credit_is_stored_as_the_immutable_grading_outcome` graded the correct color response at credit 1 and the same incorrect color response at credit 0 twice.
  - Owner: 07_questions.md / Question Backend grading and feedback (first occurrence; identical requirement and status).

- [x] PLE stores the immutable credit fraction as the grading outcome.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `commit_assessment_attempt_finalization` records each evaluated normalized credit through `record_direct_automated_grading_result`.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `grading_result_is_immutable` rejects a later change to the stored credit.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `backend_returned_credit_is_stored_as_the_immutable_grading_outcome` stored the backend credit 0 for the incorrect color response, refused to rewrite it, and refused a second grading result.

- [x] When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` returns the compiled evaluation immediately.
  - Evidence (source): `crates/question_model/src/student_work/grading.rs` `QuestionEvaluation` records only correctness and a normalized credit fraction.
  - Evidence (source): `crates/adapters/webwork/src/http_renderer/grade.rs` `QuestionGradingOutcome::Evaluated` is the only grading result for a complete renderer score.
  - Evidence (source): `crates/adapters/webwork/src/http_renderer/client.rs` `WeBWorK uses stateless grading` refuses a lifecycle state on a grading request.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `questions_are_strictly_and_deterministically_automated_grading_does_not_require_an_instructor` graded complete responses to credits 1, 1, 0, and 0.
  - Evidence (test): `crates/adapters/webwork/src/http_renderer/tests.rs` `grade_forwards_ordered_pairs_once_with_trusted_fields` returned evaluated credits 0, 0.5, and 1.
  - Evidence (test): `crates/adapters/webwork/src/lib/tests.rs` `opaque_lifecycle_issues_and_grades_one_backend_owned_payload` graded one complete payload as an evaluated outcome and kept lifecycle state empty.
  - Evidence (test): `crates/adapters/webwork/src/lib/tests.rs` `issuance_refuses_renderer_lifecycle_state_for_stateless_webwork` refused renderer-issued lifecycle state for the stateless WeBWorK integration.

- [x] Assessment scores are calculated from stored credit fractions and current Question point values.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `score_recorded_credit` multiplies the retained credit fraction by the current Question point value.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `prepare_assessment_attempt_finalization` applies that calculation when an Attempt is already submitted.
  - Evidence (test): `crates/learning-data-access/tests/grading_rescore_postgres.rs` `current_point_values_recalculate_stored_credit_without_another_backend_grade` stored backend credit 1 as 2 of 2 and backend credit 0 as 0 of 2, then scored those same credits as 5 of 5 and 0 of 5 after the point value changed from 2 to 5.

- [x] Changing Question point values recalculates scores without another Question Backend interaction.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `save_assessment` writes the current Question point value.
  - Evidence (source): `crates/server/src/assessment_delivery/submission.rs` `AlreadySubmitted` returns the prepared score before Question Backend evaluation.
  - Evidence (test): `crates/learning-data-access/tests/grading_rescore_postgres.rs` `current_point_values_recalculate_stored_credit_without_another_backend_grade` changed both point values through an Instructor save, and the next prepare returned already_submitted with no Question Attempt, response, backend, or source.

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
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted fresh PostgreSQL/MinIO actual-server and private bundled-main HTTP-proxy browser proof: 42 canonical Genetics Questions installed with zero implicit Pools, then the Instructor visibly selected distinct DNA structure and nucleotide components Revision-1 PGML Questions, attested interchangeability, created a reusable Pool, and imported a distinct Assessment-owned fork with `selection_count=1`. Real WeBWorK rendering, radio-response save/resume, exact fork Pool/Question Revision, issued ID, seed/hash preservation, whole-Attempt submit, and fresh new-Attempt selection/issued IDs passed; a new Attempt may select the same Question and need not have different seeds. Artifact: `/private/tmp/ple-algorithmic-pool-artifacts.K2Kk6Z`. Release used a 3600-second time limit and Correct answer Never; answer disclosure, full Live Demo/authentication/TLS, and all-backend acceptance are outside this receipt. Browser error arrays were empty after route teardown completed.

- [x] BiologyProblems.org WeBWorK problems should be imported from their canonical algorithmic PG or PGML source rather than from generated static variants.
  - Evidence (source): `crates/project-tools/src/curriculum_content.rs` `validate_selected_parameterized_manifest` validates canonical source pins before publication.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication importing all 42 C839-accepted canonical PGML sources (41 BiologyProblems.org sources plus HLA), preserving source pins and producing ordinary available WeBWorK Question lineages. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.

- [x] Multiple static BiologyProblems.org questions generated from one algorithmic source represent one Published Question, not separate Published Questions or a Question Pool.
  - Evidence (source): `crates/project-tools/src/curriculum_content/publication.rs` `validate_loaded_content` rejects non-Fixed entries in the canonical Blueprint.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication creating one Revision-1 Question lineage per canonical source, 42 direct Fixed entries, and zero Pools; exact replay was unchanged and a same-short-name conflict made no mutation. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.

### Published Question specifications
- [x] A Published Question is an immutable-revision Question available for reuse through the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` publishes one Revision from the current Draft.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision_is_immutable` rejects a later change to that Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns that available Published Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` returns that same Revision.

- [x] Published Questions are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `question_library_entries` requires an active Instructor Account and exposes available Question summaries.

#### Published Question identity specifications
- [x] Published Questions receive a public `XXXX-ZXXX` Crockford Base32 ID.
  - Evidence (source): `crates/server/src/question_publication.rs` `RandomQuestionIdIssuer` mints the exact public form, and `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` stores only that hyphenated form.

- [x] Question IDs have the canonical form `XXXX-ZXXX`.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` accepts only four Crockford characters, a hyphen, a checksum character, and three Crockford characters.
  - Evidence (source): `crates/question_model/src/question_library.rs` `from_random_identifier` builds that same `XXXX-ZXXX` form.

- [x] The hyphen is part of the canonical ID and makes Question IDs immediately recognizable.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QUESTION_ID_HYPHEN_INDEX` requires the hyphen at the fifth character of the canonical Question ID.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` rejects a Question ID that omits that hyphen.

- [x] Human-entered Question IDs may omit the hyphen.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` accepts an eight-character explicit human-entry value and inserts the canonical hyphen before validation.

- [x] Normalize accepted human input to the canonical hyphenated form before validation and lookup.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` normalizes explicit human entry, inserts the hyphen, and invokes `validateCanonicalQuestionIdSyntax`.

- [x] PLE always stores, transmits, displays, and copies the canonical hyphenated form.
  - Evidence (test): `crates/question_model/src/question_library.rs` `question_and_pool_ids_generate_and_transmit_only_the_canonical_hyphenated_form` mints `ABCD-XEFG` from seven Crockford characters and JSON-transmits that hyphenated value for both Question and Pool IDs, rejecting an unhyphenated value, a lowercase value, and a bad checksum.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` accepts storage only when the hyphenated checksum form matches.
  - Evidence (test): `tests/test_frontend_contract.mjs` `PLE always stores, transmits, displays, and copies the canonical hyphenated form.` renders `ABCD-XEFG` in the Question ID code and refuses an unhyphenated display value before any copy control appears.
  - Evidence (test): `tests/playwright/record_list_contracts.mjs` `checkSemanticContent` clicks Copy Question ID `7K3M-79QP` and records that exact hyphenated clipboard value. No Live Demo stack was started. No PostgreSQL proof was run.

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

- [x] Published Questions may include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `save_authoring_draft_general_feedback` stores optional `hint`, `worked_solution`, and `general_feedback` on the Draft, and leaves Hint and Worked Solution unchanged when they are omitted.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` copies Draft `hint`, `worked_solution`, and `general_feedback` onto the Published Question Revision.
  - Evidence (source): `crates/server/src/authoring.rs` `save_general_feedback` writes Hint and Worked Solution only when the request contains both keys.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `published questions include optional PLE-managed hint question feedback and worked solution` loaded the three texts and saved Hint, Question Feedback, and Worked Solution together. No Live Demo stack was started. No PostgreSQL proof was run.
  - Evidence (test): `crates/server/src/authoring.rs` `published_questions_include_optional_hint_feedback_and_worked_solution` kept omitted Hint and Worked Solution unchanged, replaced both when present, and rejected a body that sent only one of them.
  - Owner: 07_questions.md / Published Question metadata (first occurrence; identical requirement and status).

- [x] Published Questions also use the shared Question Library metadata required for publication.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a Published Question without a Discipline or a Subject and stores Discipline, Subject, Topic, Subtopic, and Tags.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` requires one Discipline and one Subject and stores optional Topic, Subtopic, and Tags.
  - Evidence (source): `crates/server/src/authoring.rs` `publish_draft` copies the Draft Question Tags and the requested Discipline, Subject, Topic, and Subtopic into publication.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `publication sends shared Question Library metadata and refuses a missing Discipline or Subject` sent that classification and withheld a request that omitted Discipline or Subject.

#### Published Question revisions, edits, and forks
- [x] **Published Questions** maintain immutable revision history.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision_is_immutable` trigger protects revision rows.

- [x] Assessments and Student Work remain pinned to exact immutable Published Question Revisions.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `assessment_entry_question` stores the Assessment's exact Published Question Revision.
  - Evidence (source): `schemas/base_schema/20_tables/assessment_attempt.sql` `issued_question` stores the Student Work Published Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` appends the next Revision and does not write Assessment entries or issued Questions.

- [x] Publishing a new Question Revision does not silently change existing Assessments or Student Work.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` appends the next Revision and does not write Assessment entries or issued Questions.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `assessment_entry_question` keeps the Assessment on the Revision recorded before publication.
  - Evidence (source): `schemas/base_schema/20_tables/assessment_attempt.sql` `issued_question` keeps Student Work on the Revision recorded before publication.

- [x] The Question owner may publish corrections, wording changes, accessibility improvements, answer changes, grading changes, and other updates as a new Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` appends an owner-authored revision.

- [x] Changing Question source, answer content, grading rules, Hints, Question Feedback, Worked Solutions,
  or Question Image Assets creates a new Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` appends the next Revision unless `source_object_checksum`, Question Feedback `general_feedback`, Hint `hint`, Worked Solution `worked_solution`, and HOTSPOT image flag `v_question_image_unchanged` all match the parent. A different draft-owned Question Image Asset does not keep the current Revision.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `correct_choice` stores answer content in the Question source. `PleQuestionJsonNumericResponseTolerance` stores a grading rule. `PleQuestionJsonHotspotSurface` stores the Question Image Asset id and checksum in that same source.
  - Evidence (test): `crates/adapters/ple/src/question_json/tests.rs` `changing_source_answer_grading_or_question_image_changes_the_revision_source_checksum` changed the prompt, the correct choice, the numeric tolerance, and the HOTSPOT image tuple. Each change produced a different canonical source checksum. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Changes to the Question title, description, Discipline, Subject, Topic, Subtopic, Tags, or other
  search metadata update the Published Question metadata while preserving the current Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/published_question_metadata_operations.sql` `bulk_replace_published_question_metadata` stores each Question Title and Description and the shared Discipline, Subject, Topic, Subtopic, and Tags on published_question_metadata without inserting a Question Revision.
  - Evidence (source): `crates/learning-data-access/src/postgres/question_bulk_metadata.rs` `bulk_replace_published_question_metadata` sends that per-Question text with the shared metadata patch.
  - Evidence (source): `src/api/http_client/question_bulk_metadata.ts` `updateQuestionBulkMetadata` is the browser command for that metadata update.
  - Evidence (test): `tests/test_library_classification_search.mjs` `search metadata updates Title, Description, and classification without a Question Revision` sent two Question titles and descriptions with one shared classification and Tags and refused a blank Title before the request.

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

- [x] Watching a Published Question drives in-app notifications for new Revisions, forks, improvement
  threads, and impact notices.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_watch_revision_event` records a Published Question Revision, and `snapshot_library_watch_event_recipients` delivers that event to the Question's current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_watch_fork_event` records a Published Question fork, and `enqueue_library_watch_thread_event` records an improvement thread for that watched Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_impact_event` records an impact notice for that watched Question.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `eventLabel` names New Revision, New public fork, Improvement thread activity, and Impact notice activity in the Watch inbox.
  - Evidence (test): `tests/test_library_watch_notification_client.mjs` `Library Watch inbox returns every discriminated event shape privately` decoded a Published Question Revision, fork, improvement thread, and impact notice through the shipped Watch client. `crates/server/src/library_watch_notification.rs` `every_watch_activity_serializes_to_the_stable_wire_shape` serialized those four Question activities. No Live Demo stack was started. No PostgreSQL proof was run.

#### Published Question behavior specifications
- [x] Published Questions may include optional PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `save_authoring_draft_general_feedback` stores optional `hint`, `worked_solution`, and `general_feedback` on the Draft, and leaves Hint and Worked Solution unchanged when they are omitted.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` copies Draft `hint`, `worked_solution`, and `general_feedback` onto the Published Question Revision.
  - Evidence (source): `crates/server/src/authoring.rs` `save_general_feedback` writes Hint and Worked Solution only when the request contains both keys.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `published questions include optional PLE-managed hint question feedback and worked solution` loaded the three texts and saved Hint, Question Feedback, and Worked Solution together. No Live Demo stack was started. No PostgreSQL proof was run.
  - Evidence (test): `crates/server/src/authoring.rs` `published_questions_include_optional_hint_feedback_and_worked_solution` kept omitted Hint and Worked Solution unchanged, replaced both when present, and rejected a body that sent only one of them.
  - Owner: 07_questions.md / Published Question metadata (first occurrence; identical requirement and status).

- [x] PLE-managed Hints, Question Feedback, and Worked Solutions are separate from Question Backend-generated content.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `hint` stores the PLE-managed Hint on the Question Revision. `worked_solution` stores the PLE-managed Worked Solution. `general_feedback` stores Question Feedback. Those columns are not the Question source binding.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `disclosed_revision_support` projects the Revision texts and does not take Question Backend-generated content.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_teaching_feedback` assigns a backend Question Answer and Question Answer Explanation without writing Question Feedback, Hints, or Worked Solutions.
  - Evidence (test): `crates/server/src/assessment_delivery/history.rs` `ple_managed_support_stays_separate_from_backend_generated_content` kept the PLE Hint, Question Feedback, and Worked Solution while the backend answer and explanation were disclosed in their own fields. The backend content stayed out of the three PLE-managed fields. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] WeBWorK Questions may use PLE-managed Hints, Question Feedback, and Worked Solutions even when similar material also exists in the WeBWorK source.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `hint` stores the optional PLE-managed Hint on the Question Revision, separate from the WeBWorK source. `worked_solution` stores the optional PLE-managed Worked Solution the same way. `general_feedback` stores Question Feedback.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history_response_sources` returns `hint` and `worked_solution` from the issued Question Revision for every backend.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `disclosed_revision_support` projects those Revision texts for every backend, including WeBWorK, and does not read the WeBWorK source. A native Question Hint is used only when the Revision Hint is absent.
  - Evidence (test): `crates/server/src/assessment_delivery/history.rs` `webwork_questions_keep_ple_managed_hints_and_worked_solutions` disclosed the PLE Hint and Worked Solution while a WeBWorK BEGIN_HINT and BEGIN_SOLUTION string stayed out of those blocks. Withheld timings hid both. A native Hint appeared only when the Revision Hint was absent. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Question Feedback is shown when its disclosure rules allow it.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `released_general_feedback` shows the Revision's Question Feedback when that text is present and omits it when absent.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns that feedback without using Assessment correct-answer disclosure.
  - Evidence (source): `src/pages/assessment_attempt_summary_page.tsx` `ReleasedBlocks` shows the General feedback block when the history carries it.
  - Evidence (test): `crates/server/src/assessment_delivery/history.rs` `question_feedback_is_shown_when_its_disclosure_rules_allow_it` kept "Keep the units." visible while a withheld answer decision hid the correct answer and still showed provided backend choice feedback. Absent Question Feedback stayed absent. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Hints and Worked Solutions use their own disclosure settings.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_disclosed_support` shows Hint blocks only when the Hints timing allows them and Worked Solution blocks only when the Worked Solutions timing allows them.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_disclosed_support` applies that projection from `project_released_content` on Attempt history. `disclosed_revision_support` uses the Revision Hint when it is stored and a native Question Hint only when the Revision Hint is absent. A Worked Solution stays absent until the Revision stores one.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `feedback_hints` stores the Hints timing separately from Question Answer disclosure. `feedback_worked_solutions` stores the Worked Solutions timing. Both default to Never.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `disclosureSummary` names the Hints timing and the Worked Solutions timing in the Student-facing sentence.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `hints_and_worked_solutions_use_their_own_disclosure_settings` released Hints during the Attempt while Worked Solutions stayed hidden, then released Worked Solutions after submit while Hints stayed hidden. The Quiz cohort gate kept the released Worked Solution and hid the Question Answer. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Student workflows remain complete when a Question has none of this optional support content.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AssessmentAttemptPage` presents the open Attempt without requiring Hints, Question Feedback, or Worked Solutions.
  - Evidence (source): `src/pages/assessment_attempt_summary_page.tsx` `ReleasedBlocks` shows a support block only when that block is present.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `general_feedback` may be absent; a NULL value means that optional fact is not stored.

### Question Pool specifications
- [x] A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` persists an ordered nonempty set of exact Published Question Revision members, and `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` selects from that Pool for Student delivery.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `start` passed accepted actual-server proof that selected an exact Pool member for Student Attempt 1, preserved it on resume, and selected again for Attempt 2. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.

- [x] Pool contents should represent reasonably interchangeable assessments of the intended learning.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` requires the creating Instructor's true `interchangeability_attested` value; it does not substitute an automatic pedagogical evaluator.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted actual-main proof that required the Instructor's attestation before creating the ordered reusable Pool and before its later Assessment-owned reorder. Artifacts: `/private/tmp/ple-course-empty-artifacts.bzwXEa` and `/private/tmp/ple-course-empty-artifacts.lgyOMK`.
  - Evidence (runtime): `crates/server/src/question_pool_creation.rs` `create_question_pool` passed accepted actual-server proof that false or missing attestation returned 422 and left no Pool behind. Artifact: `/private/tmp/ple-course-empty-artifacts.hvS4KT`.

- [x] Question Pools may contain Questions from any Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores Published Questions from each production Question Backend in one Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_backend_is_supported_for_production` accepts the ple and webwork backends.

- [x] Question Pools are created from a Published Question and enter the Question Library immediately.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` requires each member to be an available Published Question and stores the Pool in that call.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that new Pool to the Instructor.

- [x] A Question Pool is an independently reusable Question Library object.
  - Evidence (source): `crates/server/src/question_pool_library.rs` `current_pool` reads a Pool independently of any Assessment.
  - Evidence (runtime): `crates/server/src/question_pool_library.rs` `current_pool` passed accepted actual-main Instructor proof: Pool `SBQR-N5RE` was created from the Question Library and its ordered member pins were read through `/api/question-pools/SBQR-N5RE`; separate actual-server proof then imported another reusable Pool into an Assessment.

- [x] Question Pools are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` and `read_current_published_question_pool` authorize active Instructors and project only public Pool/Revision/member facts.
  - Evidence (runtime): `crates/server/src/question_pool_library.rs` `list_pools` passed accepted actual-server proof that a second vetted Instructor listed and read root Pool `1N6T-MZRD` and child Pool `J1BX-8V8F` with exact public member pins and no Course facts. A nonmember Assessment-fork PUT returned 404 without mutation; Student and anonymous Pool list/read calls returned no-store 404. Artifact: `/private/tmp/ple-course-empty-artifacts.hvS4KT`.

- [x] A Question Pool has its own public `XXXX-ZXXX` Crockford Base32 ID.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_id` is the Pool's own public ID column.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` requires that ID to keep the `XXXX-ZXXX` checksum form.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `reserve_public_id` records the ID for `question_pool` and rejects a second reservation of the same ID.

- [x] A Question Pool is a current ordered list of exact Published Question Revisions plus its
  metadata. Saving the list re-attests interchangeability and advances the Pool's Edit Number;
  no Revision is created. Removing ten Questions and saving once is one Edit.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_member` stores the current ordered Published Question Revision pins, and `question_pool` stores the Pool metadata with an Edit Number and no Revision column.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `save_question_pool_members` re-attests interchangeability and advances `question_pool_edit_number` by one for one member-list save.

- [x] Importing a Question Pool into a new Assessment automatically forks the Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_pool_forks.sql` `import_assessment_question_pool_fork` atomically creates a fresh child Pool Revision and Assessment Entry from an exact reusable source Revision without accepting raw member pins.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `import_fork` passed accepted actual-server proof that imported source Pool `P8H3-QYX9` into a direct Assessment and returned distinct fork `VFH9-CQKS`, Revision 1, at Assessment Edit 2.

- [x] The fork belongs to the new Assessment and can be changed without changing the source Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `assessment_question_pool_fork` owns each child Pool through exactly one Assessment Entry, and `schemas/base_schema/50_functions/question_pools.sql` retains exact source-Revision provenance.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `append_fork_revision` passed accepted actual-server proof that appended the fork's Revision 2 with the two exact member pins reversed, then reread the reusable source unchanged at Revision 1 with its original order. Artifact: `/private/tmp/ple-course-empty-artifacts.BbKFFd`.

- [x] Forking a Question Pool preserves its list of Published Questions by their public `XXXX-ZXXX` IDs.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `construct_question_pool_fork` copies each source member `published_question_id` onto the new Pool.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` requires that copied public ID to keep the `XXXX-ZXXX` checksum form.

- [x] Question Pools work the same way regardless of the Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `prepare_current_assessment_attempt_start` returns every member of one Pool with its source backend and does not split that Pool by backend.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` stores one Pool selection and both issued Questions, and keeps the source-backend reproduction seed.

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
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` stores a new Question Pool selection on a new Attempt and returns the open Attempt without writing another selection.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `prepare_current_assessment_attempt_start` returns that Pool's current members for the new selection.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `prepare_current_assessment_attempt_start_decision` returns no resumable Attempt once the open Attempt has passed its stored expiration.

- [x] Student Work pins the Published Question ID, its Revision Number, the Question Pool ID, and the
  Pool's Edit Number for every Question served from a Pool; the pinned Published Question
  Revision is all that later interpretation and grading need.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` stores the Pool ID, its Edit Number, and each selected Published Question ID and Revision Number on the Attempt.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_access.sql` `read_student_assessment_attempt_pool_selection` returns those stored pins.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `prepare_assessment_attempt_finalization` returns the issued Published Question ID, Revision Number, and that Revision's source binding.

- [x] Grading and historical evidence follow the exact Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history_response_sources` retains `question_id` and `revision_number`.
  - Evidence (test): `crates/question_model/src/student_work/model_tests.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the exact issued linkage.

- [x] Each member of a Question Pool is a **Published Question**.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `question_pool_member` stores each exact Question Revision Tuple.

- [x] Question Pools contain only **Published Questions**; Question Pools cannot be members of Question Pools.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_member` stores each member as a Published Question ID and Revision Number.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `validate_question_pool_member_insert` requires that member to be a current production Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `reserve_public_id` keeps one public ID from being both a Question Pool and a Published Question.

- [x] Watching a Question Pool drives in-app notifications for membership edits, forks, improvement
  threads, and impact notices.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_pool_watch_members_changed_event` records a Pool membership edit for current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_pool_watch_fork_event` records a Pool fork for current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_thread_event` records an improvement thread for a watched Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_impact_event` records an impact notice for a watched Pool.
  - Evidence (source): `crates/learning-data-access/src/postgres/library_watch_notification.rs` `MembersChanged` reads that membership edit into the private Watch inbox and `require_pool_membership_target` refuses it for a Published Question.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `membersChanged` labels the inbox row Membership edit.
  - Evidence (test): `tests/test_library_watch_notification_client.mjs` `Library Watch inbox returns every discriminated event shape privately` decoded a Question Pool membersChanged notification with Edit Number 5 through the shipped Watch client, and rejected a membership edit aimed at a Published Question. watch_activity_accepts_each_complete_variant accepted members_changed for a Pool and rejected it for a Question. every_watch_activity_serializes_to_the_stable_wire_shape serialized membersChanged. No Live Demo stack was started. No PostgreSQL proof was run.

#### Question Pool metadata
- [x] Question Pools have metadata specific to the individual Question Pool.
  - Evidence (runtime): `schemas/base_schema/50_functions/question_pools.sql` `question_pool` owns independent metadata. Accepted SQL/rollback/concurrency and final SQL, Rust/API, and browser reviews combine with root-supplied rebuilt `8147` HTTP/browser proof at `/private/tmp/ple-pool-metadata-connected-report.md`: independent metadata survives list/current reads and real Library UI creation/retry. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.

- [x] Question Pool metadata includes Title and Description.
  - Evidence (runtime): Required independent Title/Description in `schemas/base_schema/50_functions/question_pools.sql` have accepted SQL and source review. Rebuilt `8147` proof at `/private/tmp/ple-pool-metadata-connected-report.md` rejects missing fields, retains exact list/current text, and preserves both fields after denied mixed-member UI creation. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.

- [x] The first Published Question establishes the Question Pool's Discipline and Subject.
  - Evidence (runtime): Accepted actual-role SQL creation proof and final source reviews establish first-member classification. Rebuilt `8147` HTTP/browser proof at `/private/tmp/ple-pool-metadata-connected-report.md` retains exact ordered pins and first-member Discipline/Subject, rejects mixed Subject with `422` and unchanged public list, then creates after ordinary picker reselection. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.

- [x] Every additional Published Question added to the Pool has the same Discipline and Subject as the Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `save_question_pool_members` refuses a new member whose Discipline or Subject differs from the Pool and stores a new member that matches.

- [x] Published Questions retain their own Topic, Subtopic, Tags, and other Library Object metadata
  when included in a Question Pool.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` keeps Topic, Subtopic, Tags, Title, and Description on the Published Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `save_question_pool_members` updates the Pool member list and does not write that Question metadata.

- [x] Question Pools may have their own authorship, attribution, license, and source information where
  appropriate.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_authorship` stores optional Pool-owned authors.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_license` stores an optional Pool-owned SPDX license.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_source` stores optional Pool-owned attribution and source information.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `save_question_pool_provenance` stores those facts, and a later empty save clears them without writing member Question authorship, license, citation, the Pool edit number, or source_question_pool_id.

- [x] Question Pool metadata describes the Pool rather than duplicating metadata from its member
  Published Questions.
  - Evidence (runtime): Accepted SQL/source proof establishes independent Title/Description, empty creation Tags, optional narrower hierarchy, classification retention after Question reclassification, and historical fork preservation. Rebuilt `8147` HTTP/browser proof at `/private/tmp/ple-pool-metadata-connected-report.md` confirms separately authored Pool text through creation, retry, list, and current reads. No historical Pool HTTP route is claimed. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.

- [x] Question Pools may include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `hint` stores optional Pool Hint, Question Feedback, and Worked Solution text. NULL means absent. No PostgreSQL proof was run. No Live Demo stack was started.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_support.sql` `save_question_pool_ple_managed_support` replaces those three Pool texts and leaves the member-list Edit Number unchanged.
  - Evidence (source): `src/components/question_pool_support_editor.tsx` `QuestionPoolSupportEditor` edits the three Pool texts. Headless Chromium filled Hint, Question Feedback, and Worked Solution, saved them, and showed the fields at 1280 and 390 pixels. No Live Demo stack was started.
  - Evidence (test): `crates/server/src/question_pool_support.rs` `pools_keep_optional_ple_managed_support_without_copying_member_questions` saved the three texts, cleared blank text, and kept Edit Number 4.

- [x] Question Pools also use the shared Question Library metadata required for publication.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` stores one Discipline, one Subject, optional Topic and Subtopic, and Tags on each Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` copies the first member Discipline and Subject onto the Question Pool and stores its Tags.
  - Evidence (source): `src/api/decoders/question_pool_library.ts` `decodeQuestionPoolMetadata` requires that Discipline and Subject and accepts optional Topic, Subtopic, and Tags.
  - Evidence (test): `tests/test_question_pool_metadata.mjs` `Question Pools use the shared Question Library metadata required for publication` decoded that shared metadata and refused a Pool that omitted Discipline or Subject.

### Question Library specifications
- [x] Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
  - Evidence (source): `src/pages/library_route_page.tsx` `LibraryRoutePage` is the production Instructor Library surface.

- [x] The Question Library is one global collection of Published Questions and Question Pools.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` discovers Published Questions and Published Question Pools on one Question Library page.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `question_library_entries` returns Published Questions for an Instructor or Sysadmin and left-joins Course membership only to mark course use.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns Question Pools from the global Question Pool table for an Instructor or Sysadmin without a Course filter.
  - Evidence (test): `tests/test_library_classification_search.mjs` `The Question Library is one global collection of Published Questions and Question Pools` requested `/api/questions/search` and `/api/question-pools` with no Course id.

- [x] Draft Questions are not part of the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns accepted Published Question Revisions and does not read Draft Questions.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` searches those same Published Question Revisions.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `list_authoring_drafts` returns the Instructor's Draft Questions outside the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns published Question Pools and does not read Draft Questions.
  - Owner: 07_questions.md / Draft Question specifications (first occurrence; identical requirement and status).

- [x] **Published Questions** and Question Pools are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `create_instructor_account` creates an Instructor Account only after completed Instructor identity vetting.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_instructor` recognizes that active Instructor role.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `question_library_entries` returns available Published Questions to an active Instructor or Sysadmin and does not filter by author or Course.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns Question Pools to an active Instructor or Sysadmin and does not filter by author or Course.
  - Evidence (source): `crates/server/src/question_library.rs` `library_reader_session_hash` admits an Instructor session to Question search and conceals a Student session.
  - Evidence (source): `crates/server/src/question_pool_library.rs` `library_reader` admits an Instructor session to Question Pool discovery and conceals a Student session.
  - Evidence (test): `crates/server/src/question_library/tests.rs` `question_search_resolves_native_source_only_for_the_returned_page` searches as an Instructor and requires the author and Course filters to stay off.

- [x] **Students** access Question content through their Coursework rather than through the Question Library.
  - Evidence (source): `src/route_contract.ts` `ROUTE_CONTRACT` reserves both Question Library routes for Instructors, and `src/route_access_boundary.tsx` `withRouteAccessBoundary` fail-closes every protected route before its page component mounts.
  - Evidence (runtime): `src/route_access_boundary.tsx` `withRouteAccessBoundary` passed accepted actual-main Student proof that denied three Library routes without any Question Library API request, while the Student Ribbon allowed Coursework navigation to a Released Assessment. Earlier accepted native Student Attempt proof delivered Question content through that Assessment. Artifacts: `/private/tmp/ple-course-empty-artifacts.9s89JA` and `/private/tmp/ple-course-empty-artifacts.ZquNiI`.

- [x] Question Library content remains discoverable when used by a private **Course Instance**.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionSearchResult` is global and separately reports course use.

- [x] With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` is the Question Library collection surface and does not mount an archive control. `src/pages/question_detail_page.tsx` `QuestionArchiveControl` offers one Archive Published Question action for one available Published Question, and the danger zone appears only after that action is opened.
  - Evidence (test): `tests/test_question_library_archive_workflow.mjs` `renderLibrarySearch` rendered the shipped LibraryPage search surface with Search Question Library and Create Question Pool and without Archive. `renderQuestionArchive` rendered shipped QuestionArchiveControl as one Archive Published Question button, hid Danger Zone, and called getQuestionLineage without archiveQuestion. The render did not load 13,000 Questions. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Question Library workflows should support bulk operations because an **Instructor** may manage thousands of Questions.
  - Evidence (source): `crates/server/src/question_bulk_metadata.rs` `question_bulk_metadata_router` registers POST /api/questions/bulk-metadata. `bulk_replace_metadata` accepts one Instructor command for the selected Published Questions, and `decode_input` refuses a selection longer than 1000 before storage. `src/pages/library_page.tsx` `QuestionBulkMetadataEditor` mounts that command on the Question Library.
  - Evidence (test): `crates/server/src/question_bulk_metadata.rs` `question_library_workflows_support_bulk_operations_for_many_questions` posted two Published Questions through the shipped route, refused 1001 items before storage while the cap stayed 1000, and concealed a Student session without a store call. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] **Instructors** should be able to select many Library objects and update shared metadata such as
  Discipline, Subject, Topic, Subtopic, Tags, or other search fields together.
  - Evidence (source): `crates/server/src/question_bulk_metadata.rs` `question_bulk_metadata_router` updates Tags, Discipline, Subject, Topic, and Subtopic for many Published Questions. `crates/server/src/question_pool_bulk_metadata.rs` `question_pool_search_metadata_router` updates Topic, Subtopic, and Tags for many Question Pools, and `decode_patch` refuses Discipline and Subject. `schemas/base_schema/50_functions/question_pools.sql` `validate_question_pool_lineage_update` keeps the Discipline and Subject established by the first member and allows those Pool search fields only while Edit Number stays unchanged. `src/pages/library_pool_discovery.tsx` `updatePoolSelection` selects many Pools, and `src/components/question_pool_search_metadata_editor.tsx` `QuestionPoolSearchMetadataEditor` submits the Pool command.
  - Evidence (test): `crates/server/src/question_pool_bulk_metadata.rs` `instructors_select_many_library_objects_and_update_shared_search_metadata` posted two Question Pools, kept Edit Number 4, refused disciplineUuid before storage, concealed a Student session without a store call, and refused 1001 Pools while the cap stayed 1000. `crates/server/src/question_bulk_metadata.rs` `question_library_workflows_support_bulk_operations_for_many_questions` posted two Published Questions through the Question command. No Live Demo stack was started. No PostgreSQL proof was run.

- [ ] Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.
  - Mismatch: search, filters, and an accepted mock-transport browser metadata workflow exist, but connected HTTP and 13k practical-cleanup evidence remains pending.

#### Question Library metadata
- [x] **Library Objects** use shared metadata for organization, search, filtering, and discovery.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` filters and searches Published Questions by Discipline, Subject, Topic, Subtopic, and Tags.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` filters and searches Question Pools by that same shared metadata.
  - Evidence (source): `src/api/question_library_repository.ts` `questionSearchRequest` sends that metadata for Published Question discovery.
  - Evidence (source): `src/api/question_pool_library_filter.ts` `questionPoolLibraryFilter` sends that metadata for Question Pool discovery.
  - Evidence (test): `tests/test_library_classification_search.mjs` `Library Objects use shared metadata for organization, search, filtering, and discovery` sent the shared metadata through both discovery requests and refused a Subject without its Discipline.

- [x] Required Question Library metadata must be complete before a Library Object enters the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a new Published Question when Discipline or Subject is absent and stores its Title, Description, language, Tags, and classification.
  - Evidence (source): `schemas/base_schema/20_tables/question_authoring.sql` `draft_question_metadata` requires that Title, Description, and language before publication copies them.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `question_library_entries` returns a Published Question only together with that Question Library metadata.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` requires a Title, Description, Discipline, and Subject.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` requires that Title and Description and copies the first member Discipline and Subject.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `publication sends shared Question Library metadata and refuses a missing Discipline or Subject` withheld publication when Discipline or Subject was omitted.

- [x] Library metadata should describe the Library Object rather than its location in a Course, Assessment, or textbook.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` stores the Published Question title, description, language, tags, and classification, and has no Course, Assessment, or textbook column.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` stores the Question Pool title, description, tags, and classification, and has no Course, Assessment, or textbook column.
  - Evidence (source): `src/api/decoders/question_bulk_metadata.ts` `decodeQuestionBulkMetadataCurrent` accepts only the Published Question identity, edit number, tags, and classification.
  - Evidence (source): `src/api/decoders/question_pool_library.ts` `decodeQuestionPoolMetadata` accepts only the Question Pool title, description, classification, and tags.
  - Evidence (test): `tests/test_library_classification_search.mjs` `Library metadata describes the Library Object rather than a Course Assessment or textbook` decoded that object metadata and refused a Course, Assessment, and textbook location.

- [x] Library Objects use the shared **Discipline**, **Subject**, **Topic**, **Subtopic**, and **Tag**
  vocabulary.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` stores content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, and tags on each Published Question.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` stores content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, and tags on each Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that Question Pool vocabulary and filters by Topic, Subtopic, and Tags.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` filters Published Questions by that same vocabulary.
  - Evidence (test): `tests/test_library_classification_search.mjs` `Library Objects share the Discipline Subject Topic Subtopic and Tag vocabulary` sent one Discipline, Subject, Topic, Subtopic, and the tag review through questionSearchRequest.

- [x] Every Library Object has exactly one **Discipline** and one **Subject**.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` stores one Discipline and one Subject for each Published Question.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` stores one Discipline and one Subject for each Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a Question without a Discipline or a Subject and stores one of each.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns that Question's Discipline and Subject.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` copies that one Discipline and Subject onto the Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that Pool's Discipline and Subject.
  - Owner: Content classification (first occurrence).

- [x] **Topic** and **Subtopic** are optional for Library Objects.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` publishes a Question with Topic and Subtopic absent, and publishes another with both present.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns those Question classifications.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores a Question Pool with Topic and Subtopic absent.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that Pool classification.
  - Owner: Content classification (first occurrence).

- [x] Library Objects may have any number of **Tags**, including none.
  - Evidence (source): `schemas/base_schema/15_table_check_functions.sql` `question_metadata_tags_are_valid` accepts an empty tag list and any number of distinct trimmed tags.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` stores a Question tag list, including none, and stores several tags.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns those Question tag lists.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores a Question Pool with no tags and stores a different caller-chosen tag list.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns those Pool tag lists.

- [x] Question Publication Validation requires Discipline and Subject before publication.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a new lineage when Discipline or Subject is absent and stores both on the Published Question.

- [x] Library Object classification follows Discipline -> Subject -> Topic -> Subtopic.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a Subtopic without a Topic, refuses a Topic from another Subject, refuses a Subtopic from another Topic, and stores one valid chain.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` rejects a Topic or Subtopic outside that chain.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns that Question's Discipline, Subject, Topic, and Subtopic.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `content_topic_id` belongs to the Pool's Subject, and a Subtopic belongs to that Topic.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores the member Question's Discipline and Subject with Topic and Subtopic absent.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that Pool classification.

- [x] Questions and Question Pools retain their Library Object classification when used in an Assessment.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `save_assessment` pins a Published Question on an Assessment and does not rewrite its Library Object classification.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns that Question's Discipline, Subject, Topic, and Subtopic after the pin.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_pool_forks.sql` `import_assessment_question_pool_fork` places a Question Pool on an Assessment by forking it.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `construct_question_pool_fork` copies the source Pool's Discipline, Subject, Topic, and Subtopic.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns the source Pool and the Assessment fork with that same classification.

- [x] Library classification supports searching, filtering, sorting, and bulk editing.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` searches and filters by Discipline, Subject, Topic, and Subtopic, and sorts the matching Questions by title or publication time.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` filters Question Pools by classification and returns them in Pool ID order.
  - Evidence (source): `schemas/base_schema/50_functions/published_question_metadata_operations.sql` `bulk_replace_published_question_metadata` replaces Subject, Topic, and Subtopic for a selected set of Published Questions.

- [x] Published Questions and Question Pools may have PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
  - Evidence (test): `crates/server/src/authoring.rs` `published_questions_include_optional_hint_feedback_and_worked_solution` kept omitted Published Question Hint and Worked Solution unchanged and replaced both when present.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `general_feedback` stores optional Pool Question Feedback beside hint and worked_solution. No PostgreSQL proof was run. No Live Demo stack was started.
  - Evidence (test): `crates/server/src/question_pool_support.rs` `pools_keep_optional_ple_managed_support_without_copying_member_questions` saved the three Pool texts and kept Edit Number 4.

- [x] Support content may be attached at the level where it applies rather than duplicated across individual Questions.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_support.sql` `save_question_pool_ple_managed_support` updates ple_data.question_pool and does not write question_revision. No PostgreSQL proof was run. No Live Demo stack was started.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `construct_question_pool_fork` copies hint, general_feedback, and worked_solution onto the new Pool.
  - Evidence (test): `crates/server/src/question_pool_support.rs` `pools_keep_optional_ple_managed_support_without_copying_member_questions` refused a body that named publishedQuestionId and recorded no member Question id.

#### Question Library object usage statistics
- [x] Published Questions and Question Pools keep privacy-safe aggregate usage statistics.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `capture_issued_question_statistics_observation` records one observation for each Issued Question when the Attempt is submitted.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `increment_question_revision_statistics` adds that observation to the Published Question aggregate.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `increment_question_pool_issue_statistics` adds the Question Pool issue count and the member selection count.

- [x] Statistics are kept separately for each Published Question Revision and for each Question
  Pool.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` stores counts for one Published Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `read_question_library_revision_usage_statistics` returns each Revision on its own, including a Revision that has not been issued.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_pool_statistics` stores the Question Pool issue count apart from those Revision rows.

- [x] Aggregate statistics contain counts and sums, never Student Attempts or identifiable Student
  records.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` stores issued, blank, answered, outcome, and credit sums.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `read_question_library_usage_statistics` returns those counts and sums.

- [x] Instructors should be able to judge how often a Question is used and how hard it is.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `read_question_library_usage_statistics` returns issued, correct, and credit sums for an Instructor.
  - Evidence (source): `src/pages/question_statistics_panel.tsx` `QuestionStatisticsPanel` shows each rate beside its observation count.

- [x] Removing Student names alone does not make statistics anonymous.
  - Evidence (source): `docs/FERPA_DATA_POLICY.md` `Question Library object usage statistics` keeps Course, Student, Account, Attempt, and response facts in Student Work.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` stores counts and credit sums without a Student name or Student record.

- [x] Shared statistics should be shown only when individual Students cannot reasonably be identified
  from the aggregate.
  - Evidence (source): `crates/question_model/src/question_library_statistics.rs` `into_shared_statistics` withholds a positive count below `DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE` and omits a small Revision or Pool cell.
  - Evidence (source): `crates/server/src/question_library/usage_statistics.rs` `question_detail_statistics` and `bulk_question_statistics` call `into_shared_statistics`.
  - Evidence (source): `crates/server/src/question_pool_library.rs` `pool_evidence` calls `into_shared_statistics` for the Pool issue count.
  - Evidence (test): `crates/question_model/src/question_library_statistics.rs` `shared_statistics_withhold_counts_that_can_identify_one_student` withheld one observation and kept an empty count available.

- [x] Course-specific analysis remains FERPA-sensitive when individual Students could be inferred.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `read_question_library_usage_statistics` returns global counts and sums with no Course or Student breakdown.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `disclose_course_class_statistics` omits a course average unless the feedback policy releases it and the cohort meets `DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE`.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `student_history` returns the `project_history` projection.
  - Evidence (test): `crates/server/src/assessment_delivery/history.rs` `course_class_statistics_stay_omitted_when_a_student_could_be_inferred` omitted a cohort of 1 and a cohort one below the minimum, omitted a safe cohort while class statistics were Never, and disclosed that safe cohort after AfterSubmit.

- [x] Schema and increment rules live in [DESIGN_DECISIONS.md](../../../../docs/DESIGN_DECISIONS.md) and
  [FERPA_DATA_POLICY.md](/docs/FERPA_DATA_POLICY.md).
  - Evidence (source): `docs/DESIGN_DECISIONS.md` `Library usage statistics are retained counters` states the stored counts and the submission increment.
  - Evidence (source): `docs/FERPA_DATA_POLICY.md` `Question Library object usage statistics` states those schema and increment rules.

#### Question Library Bloom classification metadata
- [x] Published Question Revisions and Question Pools can have a Bloom Cognitive Process and Bloom
  Knowledge Dimension.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `question_revision_bloom` stores one Cognitive Process and one Knowledge Dimension for a Published Question Revision.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_bloom` stores one Cognitive Process and one Knowledge Dimension for a Question Pool.
  - Evidence (source): `crates/learning-data-access/src/question_library.rs` `PublishedQuestionLibraryEntry` returns that Question pair when a classification row exists.
  - Evidence (source): `crates/learning-data-access/src/question_pool_library.rs` `PublishedQuestionPool` returns that Pool pair when a classification row exists.

- [x] The two Bloom dimensions are independent and together determine the object's Bloom Classification.
  - Evidence (source): `schemas/base_schema/10_types.sql` `bloom_cognitive_process` is a separate closed vocabulary from the Knowledge Dimension.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `validate_bloom_pair` accepts any Cognitive Process with any Knowledge Dimension.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `correct_question_revision_bloom` changes one stored dimension and leaves the other stored dimension in place.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `question_revision_bloom` stores those two dimensions as the classification and has no derived matrix column.

- [x] Bloom Classification supports Question Library search and Assessment item sorting.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `sortByBloomClassification` orders mixed fixed and Pool Entries from their exact Bloom pairs, and `save` sends that order through the current Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_live_page.tsx` `reloadAssessment` loads the saved Assessment again after a concurrent save conflict.
  - Evidence (source): `src/components/library_bloom_discovery.tsx` `LibraryBloomDiscovery` puts the selected Cognitive Process and Knowledge Dimension on the Question Library search.
  - Evidence (test): `tests/playwright/test_bloom_classification_workflow.mjs` `mountBloomAssessmentWorkflow` drove the shipped Assessment Question editor through sort, save, a concurrent-save conflict, and reload of the saved order. `mountBloomLibraryBrowse` drove shipped Question Library browse and recorded a search for Remember and Factual Knowledge. The browser called the pages' client methods through a test double. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] A Question Pool's Bloom Classification describes the intended cognitive work of the Pool as a whole.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_bloom` stores the Pool pair by question_pool_id.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `attach_question_pool_bloom` stores that prepared Pool pair on the Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `correct_question_pool_bloom` updates the Pool pair without reading a member Question's Bloom classification.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns the Pool pair.

- [x] Bloom Classification is left blank when a Published Question or Question Pool enters the Question
  Library, to be updated by AI later.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` publishes a Question Revision and does not insert a Bloom row.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` creates a Question Pool and does not insert a Bloom row.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `load_question_library_revision` returns a null Bloom pair when that row is absent.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns a null Bloom pair when the Pool row is absent.

- N/A AI assigns the initial Bloom Classification using a daemon after publication.
  - Reason: Deferred product behavior overrides this implementation language. Initial Bloom Classification is deferred with the AI backend, AI-backed Bloom classification is deferred until a later release, and automated daemon backends are deferred until a final server location. Publication does not start a daemon.

- [x] An **Instructor** can correct either Bloom dimension without creating a new Published Question
  Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `correct_question_revision_bloom` updates one stored dimension for an active Instructor and does not insert a Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `correct_question_pool_bloom` updates one stored Pool dimension and does not insert a Question Revision.

- [ ] Question Library search and reporting should make both Bloom dimensions useful to **Instructors**.
  - Evidence (source): `crates/question_model/src/question_search.rs` retains two independent exact Bloom filters, unchanged sorts, and normalized-query-bound cursors. `crates/learning-data-access/src/postgres/question_library.rs` applies them to the whole Library relation and computes all six plus all four guide-order counts; `src/pages/library_search_parameters.ts`, `src/pages/library_page.tsx`, and `src/components/library_bloom_discovery.tsx` retain URL/saved-search values, zeros, and empty results.
  - Verification pending: connected multi-page, role, and browser proof remains required. It stays open independently of the connected mixed-entry Assessment-sort/save/reload/concurrent-save proof required by the preceding row.

- [ ] Follow `docs/BLOOM_TAXONOMY_GUIDE.md` for Bloom classification and teaching interpretation.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` accepts only the guide's six Cognitive Process and four Knowledge Dimension spellings.
  - Verification pending: fresh PostgreSQL actual-role proof closes storage and publication-required attachment; classifier/provider semantics, Instructor-facing teaching interpretation, and connected search/reporting remain open.

#### Question Library stewardship specifications
- [ ] Question Library stewardship should use a GitHub-like model.
  - Evidence (runtime): `docs/archive/audits/sql_human_guidance_audit.md` records current Question/Pool Star and Watch SQL/LDA proof plus four-event private Watch delivery.
  - Verification pending: connected Question/Pool workflows and browser presentation remain open.

- [x] Published Questions and Question Pools can be starred and watched.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `set_current_question_star` records an Instructor Star, and `set_current_question_watch` records that Instructor's Question Watch.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_stewardship.sql` `set_current_question_pool_star` records a Pool Star, and `set_current_question_pool_watch` records that Instructor's Pool Watch.
  - Evidence (source): `src/components/question_star_control.tsx` `QuestionStarControl` and `src/components/question_pool_star_control.tsx` `QuestionPoolStarControl` expose the Star actions.
  - Evidence (source): `src/components/question_watch_control.tsx` `QuestionWatchControl` and `src/components/question_pool_watch_control.tsx` `QuestionPoolWatchControl` expose the Watch actions.
  - Evidence (test): `tests/test_frontend_contract.mjs` `Published Questions and Question Pools can be starred and watched.` starred Question 7K3M-79QP and Pool 3S8B-24DZ, watched both through the shipped client, and rendered Star question and Star Pool. `tests/test_question_watch_client.mjs` `Watch means subscription.` rendered Watch and Unwatch for a Question and a Pool. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Star means favorite and visible endorsement.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `set_current_question_star` records an active Instructor's Star only for a Published Question; `src/components/question_star_control.tsx` `QuestionStarControl` provides the visible Star and count surface.
  - Evidence (test): `tests/e2e/e2e_question_star_name_privacy.sh` `Question Star name privacy E2E` passed on 2026-09-15 with an active vetted Instructor's actual HTTP Star action and exact closed Star projection.

- [x] Vetted **Instructors** can see the star count and which vetted **Instructors** starred a Published
  Question or Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `verified_instructor_display_name` returns the display name from the completed Instructor identity vetting decision.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `read_current_question_star` sets star_count to the active endorsers whose vetted display name is present, the same names the list returns.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_stewardship.sql` `read_current_question_pool_star` sets star_count to the active endorsers whose vetted display name is present, the same names the list returns.
  - Evidence (source): `crates/server/src/question_stewardship.rs` `read_star` returns that Question projection only after `instructor_session_hash`.
  - Evidence (source): `crates/server/src/question_pool_stewardship.rs` `read_star` returns that Pool projection only after `instructor_session_hash`.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionStarControl` shows the count and names when `mayMutateLibrary` admits the Instructor role.
  - Evidence (source): `src/pages/library_page.tsx` `mayMutateLibrary` is the Instructor role and is passed to Pool discovery as `mayWatchPools`.
  - Evidence (source): `src/pages/library_pool_discovery.tsx` `QuestionPoolStarControl` shows the Pool count and names on that Instructor surface.
  - Evidence (test): `tests/test_frontend_contract.mjs` `Vetted Instructors can see the star count and which vetted Instructors starred a Published Question or Question Pool.` rendered star count 2 and the names Ada Lopez and Grace Hopper for Question 7K3M-79QP and Pool 3S8B-24DZ through the shipped Star controls and HTTP client.

- [x] Watch means subscription.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `set_current_question_watch` inserts or deletes only the current Instructor's Question Watch row.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_stewardship.sql` `set_current_question_pool_watch` inserts or deletes only the current Instructor's Pool Watch row.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `snapshot_library_watch_event_recipients` copies those Question and Pool Watch rows as the notification recipients.
  - Evidence (source): `src/components/question_watch_control.tsx` `QuestionWatchControl` shows Watch or Unwatch for that Instructor's own boolean.
  - Evidence (source): `src/components/question_pool_watch_control.tsx` `QuestionPoolWatchControl` shows Watch or Unwatch for that Instructor's own boolean.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionWatchControl` is on the Instructor Question surface.
  - Evidence (source): `src/pages/library_pool_discovery.tsx` `QuestionPoolWatchControl` is on the Instructor Pool surface.
  - Evidence (test): `tests/test_question_watch_client.mjs` `Watch means subscription.` rendered Watch and Unwatch for Question 7K3M-79QP and Pool 3S8B-24DZ, subscribed and unsubscribed through the shipped Watch client, and rejected a Pool watcher count.

- [x] Watching a Published Question drives in-app notifications for new Revisions, forks, improvement
  threads, and impact notices.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_watch_revision_event` records a Published Question Revision, and `snapshot_library_watch_event_recipients` delivers that event to the Question's current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_watch_fork_event` records a Published Question fork, and `enqueue_library_watch_thread_event` records an improvement thread for that watched Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_impact_event` records an impact notice for that watched Question.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `eventLabel` names New Revision, New public fork, Improvement thread activity, and Impact notice activity in the Watch inbox.
  - Evidence (test): `tests/test_library_watch_notification_client.mjs` `Library Watch inbox returns every discriminated event shape privately` decoded a Published Question Revision, fork, improvement thread, and impact notice through the shipped Watch client. `crates/server/src/library_watch_notification.rs` `every_watch_activity_serializes_to_the_stable_wire_shape` serialized those four Question activities. No Live Demo stack was started. No PostgreSQL proof was run.
  - Owner: earlier Human Guidance occurrence of this bullet.

- [x] Watching a Question Pool drives in-app notifications for membership edits, forks, improvement
  threads, and impact notices.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_pool_watch_members_changed_event` records a Pool membership edit for current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_pool_watch_fork_event` records a Pool fork for current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_thread_event` records an improvement thread for a watched Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_impact_event` records an impact notice for a watched Pool.
  - Evidence (source): `crates/learning-data-access/src/postgres/library_watch_notification.rs` `MembersChanged` reads that membership edit into the private Watch inbox and `require_pool_membership_target` refuses it for a Published Question.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `membersChanged` labels the inbox row Membership edit.
  - Evidence (test): `tests/test_library_watch_notification_client.mjs` `Library Watch inbox returns every discriminated event shape privately` decoded a Question Pool membersChanged notification with Edit Number 5 through the shipped Watch client, and rejected a membership edit aimed at a Published Question. watch_activity_accepts_each_complete_variant accepted members_changed for a Pool and rejected it for a Question. every_watch_activity_serializes_to_the_stable_wire_shape serialized membersChanged. No Live Demo stack was started. No PostgreSQL proof was run.
  - Owner: earlier Human Guidance occurrence of this bullet.

- [x] An **Instructor's** watch list remains private.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `read_current_question_watch` returns only the current Instructor's watching boolean and does not enumerate other Instructors' watches.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_stewardship.sql` `read_current_question_pool_watch` returns only the current Instructor's watching boolean and does not enumerate other Instructors' watches.
  - Evidence (source): `crates/server/src/question_watch.rs` `WatchResponse` serializes only that Instructor's watching boolean.
  - Evidence (source): `crates/server/src/question_pool_stewardship.rs` `WatchResponse` serializes only that Instructor's watching boolean.
  - Evidence (source): `src/api/decoders/question_watch.ts` `decodeQuestionWatchProjection` accepts only the watching field.
  - Evidence (source): `src/api/decoders/question_pool_stewardship.ts` `decodeQuestionPoolWatchProjection` accepts only the watching field.
  - Evidence (source): `src/components/question_watch_control.tsx` `QuestionWatchControl` shows Watch or Unwatch for the caller and does not list other Instructors.
  - Evidence (source): `src/components/question_pool_watch_control.tsx` `QuestionPoolWatchControl` shows Watch or Unwatch for the caller and does not list other Instructors.
  - Evidence (test): `tests/test_question_watch_client.mjs` `An Instructor's watch list remains private.` rejected a Question response that added watchList, a Pool response that added watchers, and an inbox response that added watchList, then rendered only the caller's Watch and Unwatch controls for Question 7K3M-79QP and Pool 3S8B-24DZ. The test does not open another Instructor's session.

- [x] **Students** and anonymous users do not receive **Instructor** identity lists or watch information.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `read_current_question_star` refuses a Student or empty session before returning starred Instructor names. `read_current_question_watch` refuses those sessions before returning Watch information.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_stewardship.sql` `read_current_question_pool_star` refuses a Student or empty session before returning starred Instructor names. `read_current_question_pool_watch` refuses those sessions before returning Watch information.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `read_current_library_watch_notifications` refuses a Student or empty session before returning the Watch inbox.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `students_and_anonymous_users_do_not_receive_instructor_identity_lists_or_watch_information` refused Student USSV00009 and an empty session on Question SVR1-4XYZ and Pool SVP1-FABC, then let Instructor UVSV0000A open the Question Star list, Question Watch, and the Watch inbox.
  - Evidence (test): `tests/e2e/e2e_assessment_saved_response.sh` `students_and_anonymous_users_do_not_receive_instructor_identity_lists_or_watch_information` ran that oracle on PostgreSQL. No Live Demo stack was started.
