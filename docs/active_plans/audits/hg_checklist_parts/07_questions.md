## Questions

- [x] Questions are subject agnostic. Properly tagged Questions from all subjects belong in the same Question Library.
  - Evidence (source): `crates/server/src/question_library/paging.rs` `QuestionSearchFilter` supplies the shared Library query filter without a subject partition.
- [ ] Questions are strictly and deterministically automated; grading does not require an **Instructor**.
  - Mismatch: needs runtime grading evidence for every supported backend.
- [x] Questions have one canonical title. Compact interfaces may truncate that title.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question_metadata` stores one lineage-level title.
- [x] Every Question stored by PLE has its own internal Question record.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question` owns the internal Question record.

### Draft Questions

- [x] Draft Questions are private working content.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `ple_private.draft_question` stores draft state in the private schema.
- [x] Draft Questions use current state rather than immutable Revisions.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `draft_question_edit_number` is current-state concurrency data, separate from `question_revision`.
- [x] Saving a Draft Question replaces its previous working state.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `save_authoring_draft` replaces the current draft aggregate values.
- [ ] Instructors may delete Draft Questions they no longer need.
  - Mismatch: draft creation and save operations exist, but no owned draft deletion operation was found.
- N/A PLE may clean up abandoned Draft Questions after an appropriate warning and recovery period.
  - Reason: Automated abandoned-Draft cleanup is an explicitly optional future capability; HG sets no clock or durations.

### Question formats and types

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

### Native PLE JSON Questions

- [x] The native PLE JSON Question format is private, unversioned, and unpublished.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` accepts the unversioned internal source shape.
- [x] Stored native JSON Questions may be upgraded together when the internal format changes.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` is the single internal reader for stored PLE JSON.
- [x] The native PLE JSON Question format is a strictly validated internal source shape without an external API.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` validates the internal source document.
- [ ] Native JSON Questions are static, not algorithmic nor random, and receive no random seed.
  - Mismatch: `crates/server/src/assignment_delivery.rs` `issue_new_presentations` requires and persists a `QuestionSeed` for every native PLE issue, and `crates/adapters/ple/src/lib/question_json_source.rs` `PleQuestionBackend::issue_question_json` accepts it. The source is static, but the delivered native Question still receives a seed. `randomizeChoices` is separately compiled to server-only `NativeChoiceOrder::NonceRandomized` in `crates/adapters/ple/src/question_json/source_document.rs` `compile_choices`; `crates/question_model/src/presentation/choice_order.rs` `nonce_randomized_choices` uses the durable presentation nonce only to permute stable authored choice IDs. That presentation behavior is not algorithmic Question generation, but it does not cure the seed mismatch.
- [x] Native PLE JSON supports MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonResponse` defines all eight native types.
- [x] External URLs used by native JSON Questions are explicitly recorded and reviewable.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` records the author-declared `externalResources` inventory as source metadata only, without fetching or browser permission; `validate_external_resources` bounds and de-duplicates recorded URLs.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `validate_external_resource_url` accepts only bounded, printable, absolute HTTPS URLs without user information.
  - Decision: A one-time parser proof accepted all five resource kinds and legacy omission, while rejecting invalid and duplicate URLs; it is temporary evidence and will be removed rather than retained as a permanent implementation-inventory test.
- [x] Recorded external URLs include links, images, scripts, stylesheets, and other resources.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonExternalResourceKind` is the closed Link, Image, Script, Stylesheet, and Other category set for every `externalResources` entry.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonExternalResource` binds each recorded URL to exactly one reviewed category under `deny_unknown_fields` parsing.

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

### Question Backends

- [ ] WeBWorK, iMathAS, and H5P are PLE-managed Question Backends.
  - Mismatch: C870 removed every current H5P source, import, adapter, and runtime seam; H5P is not a delivered backend.
  - Question: Which H5P content type(s) and exact pinned library versions are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [x] The initial primary Question Backends are PLE-native JSON and WeBWorK.
  - Evidence (source): `schemas/base_schema/assessment_attempt_presentation.sql` `backend IN ('ple', 'webwork')` is the delivered presentation boundary.
- [ ] iMathAS and H5P are supported secondary Question Backends.
  - Mismatch: iMathAS has a launch boundary, while C870 leaves no current H5P source, import, or delivered backend seam.
  - Question: Which H5P content type(s) and exact pinned library versions are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
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
  - Question: Which H5P content type(s) and exact pinned library versions are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [x] PLE-native Questions use the PLE Question Backend.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `question_source_binding_fields_are_valid` maps `ple` to `pleQuestionJson`.
- [ ] WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
  - Mismatch: the isolated opaque adapter proves renderer documents, ordered pairs, score, partial credit, and stateless state. Connected live-ownership proof remains required; PLE is not required to capture historic renderer feedback.
- [ ] Question Backend feedback is transient unless the backend provides a robust way for PLE to preserve it.
  - Evidence (runtime): the C910 fresh-PG17 procedure persisted only author-managed general feedback while retaining the same opaque `webworkPgml` source binding through two immutable Published Revisions. It did not capture renderer feedback.
  - Mismatch: authorized Student HTTP delivery remains unverified because the server build is blocked by the current AWS Smithy dependency incompatibility.
- [ ] PLE does not extract or reconstruct transient feedback from Question Backend source or output.
  - Evidence (runtime): the C910 procedure changed only author-managed general feedback and retained the PGML binding path and checksum unchanged; it did not extract feedback from source or output.
  - Mismatch: a connected authorized-delivery boundary proof remains unavailable while the server build is blocked by the current AWS Smithy dependency incompatibility.
- [ ] Questions may have PLE-managed general feedback that remains separate from backend-generated interaction feedback.
  - Evidence (runtime): `schemas/base_schema/question_authoring_operations.sql` stores `general_feedback` on immutable Question Revisions separately from the opaque source binding. The accepted fresh-PG17 procedure created an explicit PGML Draft/binding, saved feedback, published Revision 1, then saved feedback only and published Revision 2; old and new feedback read back immutably with the same format, path, and checksum.
  - Mismatch: the authorized Student HTTP projection/release behavior remains unverified because the server build is blocked by the current AWS Smithy dependency incompatibility.
- [ ] H5P owns its runtime, interactions, state, and scoring.
  - Mismatch: C870 leaves no current H5P source, import, adapter, or runtime seam, so H5P cannot yet own delivered runtime behavior.
  - Question: Which H5P content type(s) and exact pinned library versions are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [ ] iMathAS owns its rendering and evaluation.
  - Mismatch: needs runtime rendering and evaluation proof for iMathAS.
- [ ] Question Backends may support more complex interactions without requiring PLE to implement those interactions.
  - Mismatch: incomplete secondary backends leave the general capability unverified.
- [ ] A Question Backend returns an immutable credit fraction for each complete response it evaluates.
  - Mismatch: needs test or runtime evidence for backend evaluation and immutable outcome creation.
- [ ] PLE stores the immutable credit fraction as the grading outcome.
  - Mismatch: needs test or runtime evidence linking backend credit to stored outcome.
- [ ] When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
  - Mismatch: needs runtime grading evidence for the no-deferred-state contract.
- [ ] Assessment scores are calculated from stored credit fractions and current Question point values.
  - Mismatch: needs test or runtime scoring evidence.
- [ ] Changing Question point values recalculates scores without another Question Backend interaction.
  - Mismatch: needs test or runtime rescoring evidence.
- [ ] Preserve the distinction between WeBWorK PG and PGML source. A Question should be identified as PGML only when its source is fully PGML-compliant; otherwise identify it as PG.
  - Evidence (runtime): the accepted canonical-source inventory records 42 parameterized BiologyProblems.org sources with explicit `pgml` format and matching `.pgml` paths; C910 also proved an explicit `webworkPgml` Draft binding persists across immutable feedback-only publication.
  - Mismatch: the remaining bundled static families have not completed canonical import, publication, and catalog migration, so the product-wide classification is unverified.
- [ ] BiologyProblems.org imports should preserve whether the canonical algorithmic source is PG or PGML rather than treating both formats generically as PG/PGML.
  - Evidence (source): `content/genetics/manifest.yaml` now registers 42 accepted canonical parameterized sources with explicit PGML paths and format metadata.
  - Mismatch: the current static-bank import/catalog migration remains incomplete, so this end-to-end import behavior is unverified.
- [ ] When parameterized WeBWorK PG or PGML source exists, prefer it to importing static variants.
  - Mismatch: no selection policy enforcement or test was found.
- [ ] Preserve backend-native algorithmic variation rather than expanding one algorithmic Question into static variants.
  - Mismatch: `content/genetics` still contains generated static WeBWorK expansions; C824--C841 own the forward replacement.
- [ ] One algorithmic Question remains one Published Question regardless of how many variants its Question Backend can generate.
  - Mismatch: no completed per-family publication and catalog transition proves this lineage boundary.
- [ ] Use a Question Pool with algorithmic Questions only when the Instructor wants selection among distinct Questions, not to represent variants of one algorithmic Question.
  - Mismatch: C885 supplies backend-neutral Pool membership, but no completed Instructor workflow proves the distinct-Question purpose and preserves independent backend variation.
- [ ] BiologyProblems.org WeBWorK problems should be imported from their canonical algorithmic PG or PGML source rather than from generated static variants.
  - Evidence (runtime): C839 accepted 42 canonical PGML sources (41 official biologyproblems-website sources plus HLA) with provenance, format/path, representative render/lint, and deterministic grading evidence.
  - Mismatch: redundant static source files were removed, but ordinary publication and catalog reconciliation remain unverified; C840--C841 own that work.
- [ ] Multiple static BiologyProblems.org questions generated from one algorithmic source represent one Published Question, not separate Published Questions or a Question Pool.
  - Evidence (runtime): C839's 42-source acceptance establishes candidate canonical sources, not a Published-Question lineage.
  - Mismatch: source removal does not prove a per-family Published-Question lineage or catalog migration; C840--C841 remain open.

### Question Pools

- [ ] A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
  - Mismatch: current pools are Assignment entries, not independent published Question Library objects.
- [ ] Pool contents should represent reasonably interchangeable assessments of the intended learning.
  - Mismatch: no interchangeability validation was found.
- [ ] Question Pools may contain Questions from any Question Backend.
  - Mismatch: C885 supplies backend-neutral Pool membership, but no completed Instructor Pool workflow proves this behavior.
- [x] Each member of a Question Pool is a **Published Question**.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool_revision_member` stores each exact Published Question revision reference.
- [ ] Question Pools are always published and have no draft or unpublished state.
  - Mismatch: current Question Pools are editable Assignment content rather than published library lineages.
- [ ] A Question Pool is an independently reusable Question Library object.
  - Mismatch: current pools are Assignment entries rather than Library objects.
- [ ] A Question Pool has its own public `AAAA-ZBBB` Crockford Base32 ID and immutable revisions.
  - Mismatch: no published Question Pool lineage or public Pool ID exists.
- [ ] Importing a Question Pool into a new Assessment automatically forks the Question Pool.
  - Mismatch: no independent Pool import-and-fork operation was found.
- [ ] The fork belongs to the new Assessment and can be changed without changing the source Question Pool.
  - Mismatch: no independent Question Pool fork model was found.
- [ ] Forking a Question Pool preserves its Published Questions by their public `AAAA-ZBBB` IDs.
  - Mismatch: no Question Pool fork model exists; current Question IDs use a different display grouping.
- [ ] Question Pools work the same way regardless of the Question Backend.
  - Mismatch: incomplete secondary backend delivery leaves this unverified.
- [ ] **Instructors** choose the contents of a Question Pool and how many Questions are selected.
  - Mismatch: canonical Instructor Question Pool authoring and selection-count workflow integration remains pending.
- [x] PLE selects from the Question Pool; the selected Question Backend controls the Question interaction.
  - Evidence (source): `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` performs server-owned selection.
- [x] Question Pool selection and backend-native randomization are separate forms of variation.
  - Evidence (source): `crates/domain/src/question_pool_selection.rs` `QuestionPoolSelectionEntropy` is separate from Question backend state.
- [x] Returning to an Attempt preserves the Question Pool selections already made.
  - Evidence (source): `schemas/base_schema/assessment_attempt_access.sql` `read_reusable_question_pool_selection` reads durable selections.
  - Evidence (test): `crates/question_model/src/student_work.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks exact retained selections.
- [ ] Starting a new Attempt makes fresh selections from its Question Pools.
  - Mismatch: `crates/learning-data-access/src/postgres/assignment_delivery_start.rs` `current_attempt_start_from_rows` calls `reusable_pool_selection` when the persisted `question_pool_reuse_rule` is `reuse_selection`; `schemas/base_schema/attempt_access.sql` `read_reusable_question_pool_selection` returns the latest prior selection for the same Student and Assignment. `schemas/base_schema/attempts.sql` `question_pool_reuse_rule` permits that mode, so a new Attempt can reuse rather than freshly select its pool membership.
- [x] Student Work preserves the exact Question Pool Revision and Published Question Revision delivered.
  - Evidence (source): `crates/question_model/src/student_work.rs` `QuestionPoolSelection` retains issued Question revision references.
  - Evidence (test): `crates/question_model/src/student_work.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the issued revision link.
- [x] Grading and historical evidence follow the exact Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `read_student_assessment_attempt_history_response_sources` retains `question_id` and `revision_number`.
  - Evidence (test): `crates/question_model/src/student_work.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the exact issued linkage.

### Question Library

- [x] Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
  - Evidence (source): `src/pages/library_route_page.tsx` `LibraryRoutePage` is the production Instructor Library surface.
- [x] The Question Library is one global collection of published Question content.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question` is not course-scoped.
- [x] **Published Questions** are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `question_library_entries` requires an active Instructor Account and exposes available Question summaries.
- [ ] Published Question Pools are available to all vetted **Instructors**.
  - Mismatch: published Question Pool library objects do not exist.
- [ ] **Students** access Question content through their Coursework rather than through the Question Library.
  - Mismatch: the Student landing component is source evidence only; no browser or behavior test verifies that Students cannot reach the Question Library.
- [x] Published content remains discoverable when used by a private **Course Instance**.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionSearchResult` is global and separately reports course use.
- [ ] With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.
  - Mismatch: no product test or design enforcement establishes archive as non-primary at this scale.
- [ ] Question Library workflows should support bulk operations because an Instructor may manage thousands of Questions.
  - Mismatch: bounded browsing does not implement the required bulk operations.
- [ ] Instructors should be able to select many Questions and update shared metadata such as tags, subject, topic, or other search fields together.
  - Mismatch: no bulk Question metadata update operation was found.
- [ ] Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.
  - Mismatch: search and filters exist, but bulk editing is absent.

#### Published Question identity

- [ ] Published Questions receive a public `AAAA-ZBBB` Crockford Base32 ID.
  - Mismatch: `QuestionId` renders `AAA-BBBB`, not the HG-required `AAAA-ZBBB` grouping.
- [ ] Published Questions and published Question Pools have public Crockford Base32 IDs.
  - Mismatch: published Question Pools and their public IDs do not exist.
- [ ] Public IDs use the form `AAAA-ZBBB`.
  - Mismatch: current `QuestionId` uses `AAA-BBBB`, not the HG-required `AAAA-ZBBB` grouping.
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

#### Published Question revisions, edits, and forks

- [x] **Published Questions** maintain immutable revision history.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `question_revision_is_immutable` trigger protects revision rows.
- [ ] Assessments and Student Work remain pinned to exact immutable Published Question Revisions.
  - Mismatch: exact revision columns are source evidence only; no connected test verifies an Assessment and Student Work stay pinned across a later publication.
- [x] Publishing a new Question Revision does not silently change existing Assessments or Student Work.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` publication appends `next_revision_number` rather than rewriting prior rows.
- [x] The Question owner may publish corrections, wording changes, accessibility improvements, answer changes, grading changes, and other updates as a new Revision.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `publish_question_revision` appends an owner-authored revision.
- [x] Changing Question source, answer content, grading rules, feedback, or Question assets creates a new Question Revision.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `publish_question_revision` persists a new source binding keyed to a new revision.
- [x] Changing the Question title, description, tags, subject, topic, or other search metadata does not create a new Question Revision.
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

#### Question stewardship

- [ ] Question stewardship should use a GitHub-like model.
  - Mismatch: star/watch and improvement-thread workflows are not all delivered.
- [ ] **Published Questions** can be starred and watched, similar to GitHub.
  - Mismatch: no Question star or watch persistence model was found.
- [x] Star means favorite and visible endorsement.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `set_current_question_star` records an active Instructor's Star only for a Published Question; `src/components/question_star_control.tsx` `QuestionStarControl` provides the visible Star and count surface.
  - Evidence (test): `tests/e2e/e2e_question_star_name_privacy.sh` `Question Star name privacy E2E` passed on 2026-09-15 with an active vetted Instructor's actual HTTP Star action and exact closed Star projection.
- [x] Vetted **Instructors** can see the star count and which vetted **Instructors** starred a Question.
  - Evidence (test): `tests/e2e/e2e_question_star_name_privacy.sh` `Question Star name privacy E2E` passed on 2026-09-15 against isolated PostgreSQL 17, MinIO, and `server_core`: the active vetted Instructor received only `{starCount, viewerHasStarred, starredInstructors:[{displayName}]}`; anonymous, Student, inactive-Instructor, and HMAC-valid non-Published requests were concealed with `404`.
  - Evidence (runtime): one-time Chromium proof for `src/components/question_star_control.tsx` `QuestionStarredInstructorList` passed on 2026-09-15. It rendered the exact server-shaped display-name list with an accessible name and no link, button, or avatar; the fixture was removed after review because it is implementation proof, not a stable product contract.
- [ ] Watch means subscription.
  - Mismatch: Question watches are not implemented.
- [ ] Watching drives in-app notifications for revisions, forks, improvement threads, and impact notices.
  - Mismatch: source-bound Watch events exist for revisions and forks, but improvement threads and impact notices have no product-defined model or private delivery behavior.
  - Question: For improvement threads, who may create/read/reply/edit/resolve them, which identity/attachments/linkage/notification/retention rules apply; and for impact notices, who may create them, under what condition, with what text/category/severity/manual-or-derived/linkage/audience/update/cancel rules?
- [ ] An **Instructor's** watch list remains private.
  - Mismatch: Question watches are not implemented.
- [ ] **Students** and anonymous users do not receive **Instructor** identity lists or watch information.
  - Mismatch: Question watch access controls are not implemented.

#### Question statistics

- [ ] Privacy-safe aggregate Question statistics remain after the underlying Student records are deleted.
  - Mismatch: retention transition needs runtime or connected-oracle proof.
  - Owner: `docs/active_plans/audits/hg_checklist_parts/06_data.md`
- [ ] Question statistics are kept separately for each Published Question Revision.
  - Mismatch: private aggregate capture exists, but no released Question Statistics surface establishes this product behavior.
- [ ] Each Published Question Revision may retain aggregate counts of correct, incorrect, partial-credit, and unanswered results.
  - Mismatch: private aggregate capture exists, but no released Question Statistics surface establishes this product behavior.
- [x] Eligible Question Types may also retain aggregate answer-choice counts.
  - Evidence (source): `schemas/base_schema/statistics.sql` `selected_count` stores aggregate choice counts.
  - Owner: `docs/active_plans/audits/hg_checklist_parts/06_data.md`
- [ ] Question-level statistics may combine Revisions when clearly labeled and privacy thresholds are met.
  - Mismatch: `QuestionStatistics` is currently `Unavailable`; no labeled combined-revision view exists.
- [ ] Question statistics contain aggregate counts rather than Student Attempts or identifiable Student records.
  - Mismatch: private aggregate evidence exists, but no released Question Statistics surface establishes the required product behavior.
- [ ] Student data retention removes the underlying Student evidence without removing approved aggregate Question statistics.
  - Mismatch: needs runtime or connected-oracle evidence for retention and aggregate preservation.
- [ ] Removing Student names alone does not make statistics anonymous.
  - Mismatch: no released Question Statistics policy establishes this behavior.
- [ ] Shared Question statistics should be shown only when individual Students cannot reasonably be identified from the aggregate.
  - Mismatch: `QuestionStatistics` is currently `Unavailable`; no shared-view privacy threshold exists.
- [ ] Course-specific Question analysis remains FERPA-sensitive when individual Students could be inferred.
  - Mismatch: aggregate analysis structures exist, but no complete FERPA-sensitive product workflow was verified.

#### Question behavior

- [ ] Answer-choice randomization belongs to the Question.
  - Mismatch: native answer-choice randomization ownership has not been verified.
- [ ] PLE-native Questions control their own answer-choice randomization.
  - Mismatch: no native answer-choice randomization implementation was found.
- [x] Question writers may add optional Question Feedback when it helps.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_feedback_fields.tsx` `PleQuestionJsonFeedbackFields` edits optional feedback.
- [x] Optional Question Feedback is shown when the Question Backend provides it.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_student_feedback` releases backend-provided feedback.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `feedback_projection_allowlists_each_released_field` verifies released Question Feedback fields.
- [x] Question Feedback does not use Assessment correct-answer disclosure settings.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `StudentFeedbackReleaseDecision` has a separate Question Feedback decision.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `withheld_question_answer_is_absent_while_authorized_feedback_still_releases` verifies separate feedback and answer disclosure.
- [ ] Student workflows remain complete whether or not Students read Question Feedback.
  - Mismatch: release projection is optional, but no end-to-end Student workflow test covers completion with feedback withheld and read.
