# TODO

This list routes genuinely unfinished work. Intended behavior comes from
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) and explicit human decisions. Supporting contracts include
[CONTRACTS.md](CONTRACTS.md),
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md), and
[DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md); release evidence and sequencing
are in [ROADMAP.md](ROADMAP.md).

## Question-spec implementation follow-up

The approved 29-milestone Question-spec implementation is complete locally. All 35 scoped
behaviors below have decisive evidence and passed M29 integration acceptance. Per-item evidence and
independent reviews are in the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
The inventory, dependencies, and acceptance criteria are in the [implementation plan](active_plans/active/question_spec_implementation_plan.md)
and [execution ledger](active_plans/reports/question_spec_implementation_ledger.md). A milestone is
accepted only after its listed dependencies and evidence gates pass. Existing deferred decisions
remain in the [decision log](active_plans/decisions/question_spec_implementation_uncertainties.md).
Neil chose to retain the already-written removal of "Used in my Courses"; offline checks passed. The
finding-by-finding reconciliation is in the
[decision review](active_plans/reports/QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md#implementation-follow-up-reconciliation).

- [x] Align implementation names with PLE product vocabulary across SQL, Rust, TypeScript, JSON,
      API routes, clients, tests, and generated contracts. Blueprint Course Revision Tuple names
      are implemented; see the [M02 verification report](active_plans/reports/question_spec_m02_tuple_names.md).
      The combined Library route `/api/library-objects/search`, `questions`, and
      `PublishedQuestionFilter` now use Library Object and Questions in no Pool names across the
      producers and consumers. See the [M24 results report](active_plans/reports/QUESTION_SPEC_M24_LIBRARY_RESULTS.md).
      PLE is pre-production; change the design directly. Sources:
      [question_search.rs](../crates/question_model/src/question_search.rs) and
      [contracts.rs](../crates/question_model/src/blueprint_operations/contracts.rs).

- [x] Remove manually written Question/Pool impact notices from schema, SQL operations and grants,
      domain/API types, clients, and their tests. Preserve settled Watch notifications for
      Revisions, Question forks, Pool Edit Number changes, and Pool forks; Change Proposals remain
      separate. Watch events carry Question Revision Number or Pool Edit Number across SQL, Rust,
      API, and TypeScript. Source/review and all three connected Watch selectors pass in the
      canonical database baseline. See the
      [M14 removal report](active_plans/reports/QUESTION_SPEC_M14_NOTICES.md) and current
      [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).

- [x] When the set of Question Revision Tuples in a Pool changes, saving advances
      the Pool's Edit Number (R04).
      Remove the additional attestation checkbox, required request flag, and dedicated attester/time
      storage from Pool creation and member editing. Update SQL, Rust/API types, browser forms,
      clients, fixtures, and tests together. Preserve owner/Sysadmin checks, membership constraints,
      and the ordinary Edit Number. Pool member display sorting is spreadsheet-style and changes
      only displayed rows; remove the unsupported persisted Assessment Pool order field and all
      `QuestionPoolOrder` user choices. Canonical tuple sorting remains an internal sampling detail.
      Evidence: [question_pools.sql](../schemas/base_schema/50_functions/question_pools.sql),
      [question_pool_create_dialog.tsx](../src/components/question_pool_create_dialog.tsx), and
      [test_question_pool_creation_client.mjs](../tests/test_question_pool_creation_client.mjs).
      Order-correction implementation and evidence: [M09 order correction report](active_plans/reports/QUESTION_SPEC_M09_ORDER_CORRECTION.md).
      Verify that valid members save without an additional certification input.
      The ordinary Owner/Sysadmin Pool editor now has Add/Remove/Save and local table sorting. Its
      isolated Playwright check verifies that sorting an edited draft leaves its exact Save tuple
      set and acknowledged Edit Number unchanged. Connected PostgreSQL selectors 01-07/09 now pass
      direct Pool Save, shared references, fork/privacy/retired-Pool behavior, and Owner/Sysadmin
      authority; see `output_question_spec/assessment_saved_response_current_20261007.log`. The
      M05 nullable Pool Bloom and M06 non-owner read/use/fork criteria now pass in
      `output_question_spec/final_proofs/metadata_nonowner_proof_origin_20261007.log`; fresh
      database and final integration acceptance passed under M29.
- [x] Align Pool-use validation with unreleased Assessment Save (F03). In
      [assessments.sql](../schemas/base_schema/50_functions/assessments.sql), remove the current
      save-time rejection when an Assessment's requested `selection_count` exceeds its ordinary
      Pool's current membership. An unreleased Assessment with two valid Pool members and a
      requested count of three must save; report the specific insufficiency on that Assessment
      and block release through [assessment_release_validation.sql](../schemas/base_schema/50_functions/assessment_release_validation.sql).
      Keep Pool membership rules and established post-issue limits. The current M12 PostgreSQL
      proof passes the two-member/request-three Save and blocked-release case; see
      `output_question_spec/m12_release_current_20261007.log`.
- [x] Align Question metadata edits with reporting Pool problems (R03). Preserve a valid Question
      classification correction, show which Pool requirements are not met, and block affected release.
      The inspected metadata write does not reject edits merely because of Pool membership;
      [question_pools.sql](../schemas/base_schema/50_functions/question_pools.sql) checks classification
      on admission, while [assessment_release_validation.sql](../schemas/base_schema/50_functions/assessment_release_validation.sql)
      counts members without checking their current classification. Complete the mismatch display
      and release checks against current valid members. Current M12 assertions prove the correction
      saves, reports the invalid Pool member, blocks release, and allows release after correction.
      See `m12_release_current_20261007.log`; the already-released Assessment rule remains part of
      the M03/M11 dependent acceptance.
- [x] Align the ordinary Question availability state with
      [PUBLISHED_QUESTION_SPEC.md](QUESTION_SPECS/PUBLISHED_QUESTION_SPEC.md#archive): archived
      Questions are read-only, outside normal discovery, preserved with existing references,
      and available to restore or fork. The permitted production-browser authoring scenario now
      verifies read-only state, discovery exclusion, retained source tuple/metadata on fork,
      restoration, and return to discovery; see the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- [x] Complete the remaining Question Pool acceptance gates. M09-M11 source work is complete:
      Pool storage uses exact Question Revision Tuples; Assessments and Blueprint/Course operations
      preserve ordinary Pool IDs with Assessment-local counts; only explicit ordinary Pool forks
      copy current metadata, including both Bloom dimensions; and Attempts retain their exact
      selected Question Revision Tuples. See the [M09 Pool storage report](active_plans/reports/QUESTION_SPEC_M09_POOL_STORAGE.md),
      [M10 Pool references report](active_plans/reports/QUESTION_SPEC_M10_POOL_REFERENCES.md), and
      [M11 Pool operations report](active_plans/reports/QUESTION_SPEC_M11_POOL_OPERATIONS.md).
      Connected Pool Save/editor, shared-reference/API readback, Blueprint persistence/adoption/update,
      explicit fork independence, and issued-tuple fairness checks pass. M12 request-count,
      classification mismatch, and release-blocking runtime proofs also pass. M04/M05/M06 specific
      criteria also pass; the connected Native metadata, Pool Bloom, and non-owner permission proof
      is in `output_question_spec/final_proofs/metadata_nonowner_proof_origin_20261007.log`.
      Fresh database and final integration acceptance passed under M29; see the
      [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- [x] Align Question statistics with the agreed measures: times received by Students,
      graded-response count, average stored credit, full-credit percentage, and zero-credit
      percentage. Preserve per-Revision/per-Pool storage and existing privacy requirements.
      Assess the current statistics and display against
      [QUESTION_LIBRARY_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_SPEC.md#usage-statistics);
      existing implementation categories do not establish additional product requirements.
      Remove cross-Revision rollups from intended Library displays; report Question statistics
      per Revision. Count delivery separately from graded responses: a delivered Question in an
      unsubmitted Attempt contributes to times received, while a graded response contributes its
      stored credit.
      Current connected delivery/origin/retry SQL proof and five-student Revision/Pool display proof
      pass in `output_question_spec/statistics_current_20261007.log` and
      `output_question_spec/m23_available_final_20261007.log`. The final UI proof stores 0.5 credit
      while awarding zero points with partial credit disabled; separate scoring/display proof covers
      all five Native formats. Remaining status follows M11 dependency criteria in the
      [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
      Pool statistics accumulate from Questions delivered through that Pool, including across
      changes to its set of Question Revision Tuples. Preserve privacy-safe aggregation and
      ordinary duplicate-count protection. Verify
      delivery without submission, a submitted fractional response, and separate Revision/Pool
      totals. Existing extra counters may remain internal engineering details; they do not define
      additional displayed measures or a derived difficulty rating.
- [x] Reconcile existing Question-language gates with HG: required language has no established
      product authority. Review the Published Question schema, Native JSON decoder/validator,
      authoring forms, and publication paths so current storage requirements do not become
      required Instructor input by accident. Preserve supplied language metadata. See
      [QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_METADATA_SPEC.md#current-implementation-evidence-language)
      and [NATIVE_JSON_SPEC.md](QUESTION_SPECS/NATIVE_JSON_SPEC.md#current-implementation-evidence-language).
- [x] Provide Draft autosave with a visible saved status. Align creation, import, and saving with
      the absence of content and metadata requirements.
      Preserve empty, incomplete, and broken working content for every Backend. Preview and test
      errors leave that content available to save and edit; publication checks complete source
      and required metadata. Inspect editor, API, and storage checks against
      [DRAFT_QUESTION_SPEC.md](QUESTION_SPECS/DRAFT_QUESTION_SPEC.md). Verify saving and reopening
      unfinished Native JSON and PG/PGML Drafts, including the visible saved status, then successful
      publication after completion. Automated expired-Draft cleanup remains deferred; eventual
      cleanup requires an appropriate warning and recovery period. No expiration period is set.
      This work remains outside the accepted implementation plan.
- [x] Align Question storage and editing with complete Revision records: keep metadata on the
      record and permit ordinary in-place metadata corrections while source and grading changes
      create a new complete Revision. Carry attributes forward at publication. Audit schema,
      publication, reads, search, and save permissions against
      [QUESTION_REVISION_SPEC.md](QUESTION_SPECS/QUESTION_REVISION_SPEC.md).
      Keep Question metadata on the Question record, using the field definitions in
      [QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_METADATA_SPEC.md).
      Audit Backend source for duplicated Question metadata and align authoring,
      decoding, publication, and read paths with the ordinary record as the authority;
      Native JSON contains the Question content needed to display and grade it;
      update source examples, import/export callers, fixtures, and tests. Verify that a metadata
      correction appears consistently in detail and search without changing source or Revision.
      Native JSON holds the Backend's rendering, response, and grading material. Q31 records
      the decision in the [question log](active_plans/decisions/question_specs_open_questions.md).
      Connected M04 evidence passes Native detail/search with unchanged source/Revision Tuple and
      supplied-language publication in `output_question_spec/final_proofs/metadata_nonowner_proof_origin_20261007.log`.
- [x] Verify ordinary nullable Bloom metadata edits and Assessment sorting. Current Question and
      Pool writes use the ordinary metadata functions in
      [published_question_metadata_operations.sql](../schemas/base_schema/50_functions/published_question_metadata_operations.sql)
      and [question_pool_search_metadata.sql](../schemas/base_schema/50_functions/question_pool_search_metadata.sql).
      The shared field contract is in
      [QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_METADATA_SPEC.md#shared-field-table);
      Bloom-specific behavior is in
      [QUESTION_BLOOM_CLASSIFICATION_SPEC.md](QUESTION_SPECS/QUESTION_BLOOM_CLASSIFICATION_SPEC.md).
      Preserve owner/Sysadmin editing and pending NULL values. The Question selector already proves
      a one-dimension correction with source/Revision preservation, ordinary concurrency, and
      owner/Sysadmin permissions. Pools have no Revision; evidence for ordinary nullable Pool Bloom
      correction passes connected edit/readback; Assessment sorting passes its complete test.
      `metadata_nonowner_proof_origin_20261007.log` records the Pool check. Pools have no Revision.
      This is settled, not another product interview; fresh-database integration passed under M29.
- [x] Implement direct ordinary Pool references for Assessments and Assessment-local selection
      counts, removing the special Assessment-owned Pool association and its fork-only API/editor
      paths. The connected selector proves two Assessments save and read back the same direct Pool ID
      at counts 1 and 2 while the Pool owner remains unchanged (`03_course_pool_forks.sql` 390-471);
      see the [M10 Pool references report](active_plans/reports/QUESTION_SPEC_M10_POOL_REFERENCES.md).
- [x] Store Question and Blueprint parent references with the ordinary object records. Fold
      `question_fork_source`, `draft_question_fork_source`, and `blueprint_course_fork` into their
      respective ordinary records, retaining the exact source Revision where applicable and
      carrying Draft source attribution through publication. Keep normal Draft/publication and
      Blueprint lifecycles, new identities, and child Revision 1; retain parent history at the
      parent. Verify fork-of-fork records the immediate source, not a copied history.
- [x] Review retry bookkeeping currently held in `draft_question_fork_source` and
      `blueprint_course_fork_receipt` while simplifying the fork tables. Preserve repeated-request
      handling through ordinary creation behavior; request records are not product object types.
      The audit separates this concern from the confirmed Assessment Pool ownership error.
- [x] Preserve all current Pool metadata during an explicit fork, including both Bloom dimensions.
      M11 implementation and review evidence are in the
      [M11 Pool operations report](active_plans/reports/QUESTION_SPEC_M11_POOL_OPERATIONS.md);
      connected PostgreSQL checks also prove Edit Number 1, exact tuple copy, both Bloom dimensions,
      and independent parent/fork membership (`04_blueprint_pool_forks.sql` 259-325).
- [x] Allow corrections to Backend Question Type classification as ordinary metadata edits and
      report affected Pool mismatches under the release rules. Native JSON Type follows its source interaction; changing
      that interaction remains a source Revision. Inspect existing publication and metadata
      restrictions against [QUESTION_TYPE_SPEC.md](QUESTION_SPECS/QUESTION_TYPE_SPEC.md).
      WeBWorK Type is assigned manually for now; automatic PG/PGML detection is deferred outside this plan.
      M12 mismatch reporting/release checks and M06 non-owner/owner permission evidence pass; fresh-database integration passed under M29.
- [x] Apply the Assessment partial-credit setting to stored Backend credit fractions for all
      Attempts, including submitted Attempts. Enable partial credit for new Assessments.
      Store the earned fraction with the setting either
      on or off; off awards full credit for one and zero for smaller fractions. Recalculate all
      affected scores and the highest Attempt when the setting changes, without Backend regrading.
      Verify both toggle directions preserve stored fractions and give consistent Student scores.
      See [QUESTION_BACKEND_SPEC.md](QUESTION_SPECS/QUESTION_BACKEND_SPEC.md#stored-credit-and-awarded-points).
- [x] Verify connected Native JSON Matching behavior: proportional credit is correct pairs divided
      by all prompts, with equal weight, wrong or blank pairs worth zero, and no additional
      deduction. The source now grades proportionally and accepts incomplete Matching responses.
      Verify partial answers survive saving and submission, shuffled display retains keyed
      identities, and fractional credit reaches Assessment scores and Student displays. Keep
      grading in the Question Backend; other Question Types require their own settled rules. See
      [QUESTION_TYPE_SPEC.md](QUESTION_SPECS/QUESTION_TYPE_SPEC.md).
- [x] Complete connected acceptance for regular-expression answer matching in Native JSON FIB and
      MULTI-FIB. Current source validation, authoring, and grading implement regex matching; source
      tests cover valid and invalid patterns. Verify saved responses and scores through the
      connected Question and Assessment workflows. Preserve accepted-answer lists.
- [x] Grade Native JSON MULTI-FIB as independent FIBs, with equal stored credit per correct blank.
      Verify wrong and unanswered blanks earn zero and remain in the denominator, and saved
      scores reflect the resulting credit.
- [x] Implement the settled Native JSON Multiple Answer linear choice-count scoring rule in
      [MULTIPLE_ANSWER_SCORING_SPEC.md](QUESTION_SPECS/MULTIPLE_ANSWER_SCORING_SPEC.md).
      Backends grade their own Questions. Apply the Assessment Instructor's partial-credit setting
      when calculating scores. Verify score bounds, correct-selection and incorrect-selection
      behavior, saved scores, and display. New Assessments start with partial credit enabled.
- [x] Implement Native JSON ORDER partial credit as the equal-weight average of correct-position
      and correctly ordered-pair fractions. Store the earned fraction with either setting. Follow
      [ORDER_SCORING_SPEC.md](QUESTION_SPECS/ORDER_SCORING_SPEC.md).
      Verify stable item identity, the worked example, permutation and swap tables, reversed
      orders, and fractional credit through saved scores and Student displays. Keep the existing
      complete-permutation response validation. New Assessments start with partial credit enabled.
- [x] Align Published Question content and metadata editing with the owner-or-Sysadmin rule.
      Every Instructor may read, add to an Assessment, or fork any Published Question. The
      remaining focused evidence is that a non-owner Instructor can read, use, and fork while
      server permissions and UI controls restrict edits to the owner or Sysadmin. Check Sysadmin
      content editing as well as metadata access. Preserve automated
      metadata assignment as a future direction; its AI work is
      deferred outside this plan. Connected non-owner read/use/fork and owner-only edit checks pass in
      `output_question_spec/final_proofs/metadata_nonowner_proof_origin_20261007.log`; fresh-database integration passed under M29. See [QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](QUESTION_SPECS/QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md).
- [x] Restrict Instructor Bloom corrections to the owning Instructor for Questions and Pools.
      Sysadmins may also correct Bloom on Questions and Pools.
      Audit the API, database checks, and displayed edit controls against the corrected rule;
      Library read access alone must not grant editing. Connected checks pass non-owner read/use/fork,
      rejected direct metadata writes, and ordinary Pool Bloom edit/readback; fresh-database integration
      passed under M29. See [QUESTION_BLOOM_CLASSIFICATION_SPEC.md](QUESTION_SPECS/QUESTION_BLOOM_CLASSIFICATION_SPEC.md).
- [x] Reconcile existing Sysadmin access checks with full administrative authority, including
      Course and Student records. Authorize administrative operations through the Sysadmin role
      and apply the confirmation and audit rule below. Keep unproven tools deferred. See
      [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md).
      The checklist tool now uses HG's current access, confirmation, and audit wording; its
      diff and consistency checks pass. Final integration review passed; see the implementation ledger.
- [x] Add confirmation before a Sysadmin accesses FERPA-sensitive Student data, stating that
      access is needed for administrative work. Record that access for audit. Keep full Sysadmin
      authority. Verify that protected data appears after confirmation, cancelling leaves it
      undisclosed, and confirmed access is recorded. Inspect current UI, server access checks,
      and audit recording. The canonical production-browser scenario and Support journey passed in
      `output_question_spec/production_browser_permitted_20261007.log`, including direct-request
      role checks, confirmation, cancellation, and recorded access. The final screenshot corpus
      also captures the confirmation form and checks cancellation without a Student-data request.
- [x] Complete connected acceptance for shared Draft authoring across Native JSON, PG, and PGML.
      Current source includes WebWork Draft preview, response testing, and publication paths in
      addition to Native JSON authoring. Verify import or write, preview, testing, refinement,
      metadata, and publication end to end; see
      [DRAFT_QUESTION_SPEC.md](QUESTION_SPECS/DRAFT_QUESTION_SPEC.md).
- [x] Carry the Blueprint Course Theme into a Course created from it; the Instructor can then
      change the Course Theme independently. Audit the existing create/adopt path and add the
      missing Blueprint Theme support. See [BLUEPRINT_COURSE_IMPORT_API_SPEC.md](BLUEPRINT_COURSE_IMPORT_API_SPEC.md).
- [x] Carry shared Library Object fields through the combined result and browser rows. The
      [Library Object audit](active_plans/audits/library_object_documentation_audit_2026_10_05.md)
      records missing Question owner, Type, Backend, and Tags in the browser row and different
      shapes for common fields. Align the Rust/API/decoder/row/display boundary with
      [LIBRARY_OBJECT_SPEC.md](QUESTION_SPECS/LIBRARY_OBJECT_SPEC.md); retain kind-specific Revision
      and Pool fields. The passing mixed-search matrix and Library proof cover common result fields,
      both result kinds, picker rows, and detail reads. Separate summary types are an implementation
      choice, not by themselves a defect; final integration review passed under M29.
- [x] Apply shared Type search semantics to both Library Object kinds. In
      [question_library_operations.sql](../schemas/base_schema/50_functions/question_library_operations.sql),
      structured Type filtering includes Pools but Type text matching is Question-only. Query
      handling and the canonical mixed-search matrix now cover each object's own Type. Keep Pool
      Author exclusion because Pools have no Author. This does not settle the tentative default
      result filter or mixed batch picker operations; Instructor bulk editing remains deferred outside this plan.
      Final integration review passed under M29.
- [x] Verify and complete the supported Question import and Blueprint assembly path using
      generated PLE IDs and API readback. Existing content loaders already use shared Draft and
      publication services; direct-store/SQL fixtures do not establish the whole workflow. Track
      missing caller coverage against [QUESTION_IMPORT_SPEC.md](QUESTION_SPECS/QUESTION_IMPORT_SPEC.md)
      and [BLUEPRINT_COURSE_IMPORT_API_SPEC.md](BLUEPRINT_COURSE_IMPORT_API_SPEC.md). Preserve content,
      attribution, exact Question references, and reusable Pool references. Converter handoff
      remains deferred outside this plan; use the external Rust library when that boundary is implemented.
- [x] Correct stale saved-search comments and test names in
      [question_search.rs](../crates/question_model/src/question_search.rs) to describe ordinary
      filter normalization. The targeted source trace found no remaining saved-search callers or
      persistent prompt/result storage; useful parsing/round-trip checks remain. Fresh-schema filter
      and pagination checks and temporary search discard/reopen pass. See the
      [M25 report](active_plans/reports/QUESTION_SPEC_M25_SEARCH_STORAGE.md); final integration
      review passed under M29.
- [x] Verify the revised filter schema with the existing Question Library filtering and pagination
      selector. `question_library_search_filters_and_pages_in_postgresql` and its mixed-search matrix
      passed in `output_question_spec/database_baseline_final_retry_20261007.log`; the clean full
      acceptance retry also passed in `output_question_spec/all_test_clean_retry_20261007.log`.
- [x] Complete final visual and integration review for Library search and Question pickers. The
      canonical production-browser suite and fresh full screenshot corpus pass.
      `output_question_spec/canonical_screenshots_all_controls_20261007.log` records 257 captures,
      the generated atlas, and clean shutdown of the owned Demo. Corrected capture-contract SPEC/QUALITY reviews and representative independent visual review pass;
      distinct fresh final integration QUALITY passed.

## Future product capabilities

- [ ] Integrate imports that accept Native JSON directly from qti-package-maker-rs.
      Bind verified image bytes to the existing QuestionImageAssetTuple (logical asset ID and
      SHA-256 checksum) and make the corresponding asset available through existing Draft storage.
      Preserve that logical identity independently of physical object-store identity, and save
      Question metadata through the ordinary Draft metadata path. Reuse existing import and asset
      storage; prefer assets alongside text-only JSON over a ZIP package. This is separate future
      integration work; it does not reopen the accepted 29-milestone Question-spec implementation.
      Preserve and display authored HTML with inline CSS in Native JSON display content through
      authoring, preview, and live prompt, choice, and response rendering. For ordinary HTML image
      references relative to supplied import files, locate, verify, hash, and store the bytes, then
      bind them to the existing QuestionImageAssetTuple; the import path is not persisted identity,
      and the converter need not know PLE asset IDs. Add focused visual behavior checks for prompt and
      choice images when implemented. Current text escaping is implementation drift. Current
      publication binds one HOTSPOT surface image and does not enumerate ordinary prompt and choice
      image references. The HOTSPOT converter-side pre-binding shape is a shared implementation
      question; keep using the existing asset model, with PLE computing or verifying the tuple
      checksum. Transport details and final stored HTML `src` syntax remain implementation work;
      sanitization details are deferred. Existing
      author-JavaScript isolation and RDKit rules remain in force.
      See [Native JSON converter handoff](QUESTION_SPECS/NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md).
- [ ] Configure an external email provider and verify inbox delivery for the
      implemented SMTP email-code sign-in path. The optional Gmail API adapter
      remains future work under
      [GMAIL_EMAIL_DELIVERY_BACKEND.md](GMAIL_EMAIL_DELIVERY_BACKEND.md).
- [ ] Replace each BiologyProblems.org WeBWorK static expansion with its one
      canonical algorithmic PG/PGML source and one Published Question lineage. Verify
      provenance, parameter behavior, and representative rendering/grading; retire
      redundant generated-variant Pool members when present while retaining intentionally pooled
      distinct algorithmic Questions; preserve historical pins while retiring replaced
      static Questions, redundant Pools, and source copies.
- [ ] Build public Blueprint Course search as one bounded projection, Store,
      Server, authorization, and browser workflow capability.
- My Questions and Starred have Instructor routes, pages, and server-backed read paths;
  Watched opens the private watch-notification inbox. Track any remaining runtime or capture
  acceptance with its owning evidence plan instead of commissioning these routes as new work.
- [ ] Build pilot grade export as a direct authorized CSV or TSV download of
      point-based Assessment scores. Do not add LMS synchronization, separate
      Question weights, Grade Categories, weighted categories, Course Grade
      Schemes, or Course percentage calculations.
- [ ] Build Blueprint update review, source-fork update discovery and selective
      application, Blueprint Course Change Proposals, and canonical Blueprint
      JSON import/export as complete, authorized workflows. Existing Assessment
      changes require review; newly added Blueprint Assessments copy to daughter
      Courses automatically as Unreleased Assessments.
- [ ] Remove the generic Question Seed input from static PLE Question JSON
      issuance; static declarative sources do not vary and do not execute code.
- [ ] Enforce the Course Instance six-month maximum Active lifetime from
      creation: warn the Instructors, reject later Assessment deadlines, make
      the Course Inactive at the limit, preserve bulk roster import for adding
      Students, and prohibit bulk Student removal while preserving individual
      enrollment corrections.
- [ ] Implement the Course retention behavior already defined by Human
      Guidance: latest-Assessment-deadline FERPA clock, Instructor notice, FERPA archive
      from normal interfaces, recoverability during the retention period,
      and permanent deletion. FERPA retention durations are tunable operational
      configuration; exact Store/worker shapes remain implementation choices,
      and no retention Revision family is required.

## Production release

- [ ] Obtain the explicit human production-release decision after the Roadmap's
      completed implementation gates. This decision freezes the canonical base
      schema and starts the forward-only migration lifecycle.

## Evidence discipline

Temporary investigation material supports decisions without becoming a parallel
documentation layer. Keep accepted decisions in the owning contract, code, test,
or operational guide; retire duplicate or superseded working evidence. Permanent
tests protect durable behavior, authorization, evidence integrity, or lifecycle
contracts.

## Theme follow-up

- [ ] Decide whether to expose or remove the unused `html[data-contrast="increased"]` styling.
