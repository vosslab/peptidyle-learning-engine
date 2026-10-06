# TODO

This list routes genuinely unfinished work. Intended behavior comes from
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) and explicit human decisions. Supporting contracts include
[CONTRACTS.md](CONTRACTS.md),
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md), and
[DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md); release evidence and sequencing
are in [ROADMAP.md](ROADMAP.md).

## Question-spec implementation follow-up

The Question-spec review remains documentation work. Record code changes here for later execution.
Neil chose to retain the already-written removal of "Used in my Courses"; offline checks passed.
The finding-by-finding reconciliation is in the
[decision review](active_plans/reports/QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md#implementation-follow-up-reconciliation).
The items below specify later alignment work; this pass changes documentation only.

- [ ] Align implementation names with PLE product vocabulary across SQL, Rust, TypeScript, JSON,
      API routes, clients, tests, and generated contracts. Review the combined Library route
      `/api/questions/search` and its `membership` / `LibraryQuestionMembership` filter names
      against Library Object and Questions in no Pool. Rename where needed to make the intended
      concept clear. Rename `BlueprintRevisionTuple` and related identifiers to match Blueprint
      Course Revision Tuple, using each language's normal casing. Update producers and consumers
      together. PLE is pre-production; change the design directly. Current names are evidence,
      not compatibility requirements. Sources:
      [question_search.rs](../crates/question_model/src/question_search.rs) and
      [contracts.rs](../crates/question_model/src/blueprint_operations/contracts.rs).

- [ ] Remove manually written Question/Pool impact notices from schema, SQL operations and grants,
      domain/API types, clients, and their tests. Neil chose the existing fork-and-fix workflow;
      this is removal of an unwanted feature, not a notice-permissions redesign. Preserve Watch
      notifications for Revisions, forks, and Pool membership changes. Source evidence:
      [library_discussion.sql](../schemas/base_schema/20_tables/library_discussion.sql) and
      [library_discussion_operations.sql](../schemas/base_schema/50_functions/library_discussion_operations.sql).
      The [authority audit](active_plans/reports/QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md#stored-notice-history)
      traces the August HG wording, September implementation, and October partial removal.

- [ ] When the set of Question Revision Tuples in a Pool changes, saving advances
      the Pool's Edit Number (R04).
      Remove the additional attestation checkbox, required request flag, and dedicated attester/time
      storage from Pool creation and member editing. Update SQL, Rust/API types, browser forms,
      clients, fixtures, and tests together. Preserve owner/Sysadmin checks, membership constraints,
      and the ordinary Edit Number. Evidence: [question_pools.sql](../schemas/base_schema/50_functions/question_pools.sql),
      [question_pool_create_dialog.tsx](../src/components/question_pool_create_dialog.tsx), and
      [test_question_pool_creation_client.mjs](../tests/test_question_pool_creation_client.mjs).
      Verify that valid members save without an additional certification input.
- [ ] Align Pool-use validation with unreleased Assessment Save (F03). In
      [assessment_pool_forks.sql](../schemas/base_schema/50_functions/assessment_pool_forks.sql),
      member Save currently raises after removal when the selection count exceeds membership;
      [assessment_pool_selection.sql](../schemas/base_schema/50_functions/assessment_pool_selection.sql)
      also rejects a larger count immediately. Allow that temporary mismatch to save, show it on
      the affected Assessment, and block release until resolved. Align UI gates and rejection
      tests with this behavior. Retain Pool membership rules and established post-issue limits.
      Verify the example of two valid Pool members and an unreleased Assessment requesting three.
- [ ] Align Question metadata edits with reporting Pool problems (R03). Preserve a valid Question
      classification correction, show which Pool requirements are not met, and block affected release.
      The inspected metadata write does not reject edits merely because of Pool membership;
      [question_pools.sql](../schemas/base_schema/50_functions/question_pools.sql) checks classification
      on admission, while [assessment_release_validation.sql](../schemas/base_schema/50_functions/assessment_release_validation.sql)
      counts members without checking their current classification. Complete the mismatch display
      and release checks against current valid members. Verify that correction saves, unreleased
      Assessment release is blocked, and already-released Assessments continue under HG's rule.
- [ ] Align the ordinary Question availability state with
      [PUBLISHED_QUESTION_SPEC.md](QUESTION_SPECS/PUBLISHED_QUESTION_SPEC.md#archive): archived
      Questions are read-only, outside normal discovery, preserved with existing references,
      and available to restore or fork.
- [ ] Align Question statistics with the agreed measures: times received by Students,
      graded-response count, average stored credit, full-credit percentage, and zero-credit
      percentage. Preserve per-Revision/per-Pool storage and existing privacy requirements.
      Assess the current statistics and display against
      [QUESTION_LIBRARY_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_SPEC.md#usage-statistics);
      existing implementation categories do not establish additional product requirements.
      Remove cross-Revision rollups from intended Library displays; report Question statistics
      per Revision. Count delivery separately from graded responses: a delivered Question in an
      unsubmitted Attempt contributes to times received, while a graded response contributes its
      stored credit.
      Current [statistics.sql](../schemas/base_schema/50_functions/statistics.sql) increments
      `issued_count` during submission collection, exposes Question-wide rollups, and derives Pool
      outcomes from current members across Revisions. Align collection, SQL reads, API/display, and
      tests with actual delivery and graded-response measures for the referenced Revision or Pool.
      A Pool's own outcomes describe deliveries from that Pool, rather than all uses of its current
      members. Preserve privacy-safe aggregation and ordinary duplicate-count protection. Verify
      delivery without submission, a submitted fractional response, and separate Revision/Pool
      totals. Existing extra counters may remain internal engineering details; they do not define
      additional displayed measures or a derived difficulty rating.
- [ ] Reconcile existing Question-language gates with HG: required language has no established
      product authority. Review the Published Question schema, Native JSON decoder/validator,
      authoring forms, and publication paths so current storage requirements do not become
      required Instructor input by accident. Preserve supplied language metadata. See
      [QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_METADATA_SPEC.md#current-implementation-evidence-language)
      and [NATIVE_JSON_SPEC.md](QUESTION_SPECS/NATIVE_JSON_SPEC.md#current-implementation-evidence-language).
- [ ] Provide Draft autosave with a visible saved status. Align creation, import, and saving with
      the absence of content and metadata requirements.
      Preserve empty, incomplete, and broken working content for every Backend. Preview and test
      errors leave that content available to save and edit; publication checks complete source
      and required metadata. Inspect editor, API, and storage checks against
      [DRAFT_QUESTION_SPEC.md](QUESTION_SPECS/DRAFT_QUESTION_SPEC.md). Verify saving and reopening
      unfinished Native JSON and PG/PGML Drafts, including the visible saved status, then successful
      publication after completion. Automated abandoned-Draft cleanup remains deferred.
- [ ] Align Question storage and editing with complete Revision records: keep metadata on the
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
- [ ] Fold Bloom fields and saves into ordinary Question/Pool metadata editing and concurrency.
      Existing dedicated classification tables, counters, routes, and browser types need alignment;
      update generated schema documentation with that implementation. Preserve owner/Sysadmin
      editing and pending NULL values. Permit correcting either dimension while the other remains
      NULL or keeps its existing value. The current `validate_bloom_pair` in
      [question_bloom.sql](../schemas/base_schema/50_functions/question_bloom.sql) rejects NULL;
      align the validator, schema, request/response types, forms, and tests with ordinary nullable
      fields. Verify a one-dimension correction with the other absent, unchanged Revision, ordinary
      concurrency, and owner/Sysadmin permissions. Enable the Assessment's Bloom sorting action
      when some entries lack Bloom; use a consistent ordinary placement for unclassified entries.
      That placement is an implementation detail, not another product interview.
      See
      [QUESTION_BLOOM_CLASSIFICATION_SPEC.md](QUESTION_SPECS/QUESTION_BLOOM_CLASSIFICATION_SPEC.md).
- [ ] Question Pool model alignment: implement [QUESTION_POOL_SPEC.md](QUESTION_SPECS/QUESTION_POOL_SPEC.md).
      Treat this as a direct pre-production schema correction. The source audit confirms drift
      across SQL, Rust, TypeScript, tests, and fixtures; see
      [FORK_MODEL_CODE_AUDIT_2026_10_05.md](active_plans/reports/FORK_MODEL_CODE_AUDIT_2026_10_05.md).
      Adding a Pool stores its existing ID and an Assessment-entry selection count. Reconcile code,
      API, schema, UI, tests, and terminology that assign Pool ownership to an Assessment or fork
      implicitly during Assessment/Blueprint/Course assembly. Explicit forking creates a regular
      Pool with a new ID, Instructor owner, copied current Question Revision Tuples and metadata,
      Edit Number 1, and parent pointer. Current
      Pool edits affect future selections across references; existing Attempts keep their selected
      Questions. Preserve Question ID + Revision and Pool ID + Edit Number in Student Work, with
      the Edit Number used only as a concurrency/change counter. Verify that saving updates the Pool with no undo,
      and that sorting changes only the editor display. Before release, save incomplete Assessment editing
      state, show when an Assessment requests more Questions than a Pool can provide, and block
      release until resolved; apply
      established post-issue restrictions afterward. Update tests for these settled semantics.
- [ ] Remove the special Assessment-owned Pool association and attachment rules:
      `assessment_question_pool_fork`, its unique Pool constraint, required foreign key and
      source-parent trigger, and Blueprint restrictions on sharing an existing Pool. Replace
      `BlueprintPoolInputChoice` copy-on-add behavior and the Assessment-owned Pool API/view with
      ordinary Pool references and explicit fork creation. Cover Course adoption and Blueprint
      updates, regenerate API types/schema docs, and update tests and fixtures that currently
      require automatic forks. Verify that one original or forked Pool can serve multiple
      Assessments while retaining its Instructor owner and existing Attempt selections.
- [ ] Store Question and Blueprint parent references with the ordinary object records. Fold
      `question_fork_source`, `draft_question_fork_source`, and `blueprint_course_fork` into their
      respective ordinary records, retaining the exact source Revision where applicable and
      carrying Draft source attribution through publication. Keep normal Draft/publication and
      Blueprint lifecycles, new identities, and child Revision 1; retain parent history at the
      parent. Verify fork-of-fork records the immediate source, not a copied history.
- [ ] Review retry bookkeeping currently held in `draft_question_fork_source` and
      `blueprint_course_fork_receipt` while simplifying the fork tables. Preserve repeated-request
      handling through ordinary creation behavior; request records are not product object types.
      The audit separates this concern from the confirmed Assessment Pool ownership error.
- [ ] Preserve all current Pool metadata during an explicit fork, including assigned Bloom.
      The inspected SQL constructor copies members and ordinary metadata columns but omits the
      separate Bloom row. Align this with the ordinary metadata cleanup and verify independent
      subsequent edits of parent and child.
- [ ] Allow corrections to Backend Question Type classification as ordinary metadata edits and
      report affected Pool mismatches under the release rules. Native JSON Type follows its source interaction; changing
      that interaction remains a source Revision. Inspect existing publication and metadata
      restrictions against [QUESTION_TYPE_SPEC.md](QUESTION_SPECS/QUESTION_TYPE_SPEC.md).
      WeBWorK Type is assigned manually for now; automatic PG/PGML detection is deferred.
- [ ] Apply the Assessment partial-credit setting to stored Backend credit fractions for all
      Attempts, including submitted Attempts. Enable partial credit for new Assessments.
      Store the earned fraction with the setting either
      on or off; off awards full credit for one and zero for smaller fractions. Recalculate all
      affected scores and the highest Attempt when the setting changes, without Backend regrading.
      Verify both toggle directions preserve stored fractions and give consistent Student scores.
      See [QUESTION_BACKEND_SPEC.md](QUESTION_SPECS/QUESTION_BACKEND_SPEC.md#stored-credit-and-awarded-points).
- [ ] Implement proportional Native JSON Matching credit: correct pairs divided by all prompts,
      equal weight, wrong or blank pairs zero, no additional deduction. Native JSON currently returns only
      zero or one and rejects incomplete Matching responses. Verify partial answers survive
      saving and submission, shuffled display retains the keyed identities, and fractional credit
      reaches Assessment scores and Student displays. Keep grading in the Question Backend;
      other Question Types require their own settled rules. See [QUESTION_TYPE_SPEC.md](QUESTION_SPECS/QUESTION_TYPE_SPEC.md).
- [ ] Add regular-expression answer matching to Native JSON FIB and reuse it for every MULTI-FIB
      blank. Current `text_matches` supports only exact, case-insensitive, and normalized matching.
      Align source validation, Draft response testing, and grading; preserve accepted-answer lists.
- [ ] Grade Native JSON MULTI-FIB as independent FIBs, with equal stored credit per correct blank.
      Verify wrong and unanswered blanks earn zero and remain in the denominator, and saved
      scores reflect the resulting credit.
- [ ] Implement the settled Native JSON Multiple Answer linear choice-count scoring rule in
      [MULTIPLE_ANSWER_SCORING_SPEC.md](QUESTION_SPECS/MULTIPLE_ANSWER_SCORING_SPEC.md).
      Backends grade their own Questions. Apply the Assessment Instructor's partial-credit setting
      when calculating scores. Verify score bounds, correct-selection and incorrect-selection
      behavior, saved scores, and display. New Assessments start with partial credit enabled.
- [ ] Implement Native JSON ORDER partial credit as the equal-weight average of correct-position
      and correctly ordered-pair fractions. Store the earned fraction with either setting. Follow
      [ORDER_SCORING_SPEC.md](QUESTION_SPECS/ORDER_SCORING_SPEC.md).
      Verify stable item identity, the worked example, permutation and swap tables, reversed
      orders, and fractional credit through saved scores and Student displays. Keep the existing
      complete-permutation response validation. New Assessments start with partial credit enabled.
- [ ] Align Published Question content and metadata editing with the owner-or-Sysadmin rule.
      Every Instructor may read, add to an Assessment, or fork any Published Question. The
      existing bulk-metadata SQL checks Instructor status and available targets without a Question
      owner check. Check Sysadmin content editing as well as metadata access. Preserve automated
      metadata assignment as a future direction; its AI work is
      deferred. See [QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](QUESTION_SPECS/QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md).
- [ ] Restrict Instructor Bloom corrections to the owning Instructor for Questions and Pools.
      Sysadmins may also correct Bloom on Questions and Pools.
      Audit the API, database checks, and displayed edit controls against the corrected rule;
      Library read access alone must not grant editing. Verify that another Instructor can still
      fork the content. See [QUESTION_BLOOM_CLASSIFICATION_SPEC.md](QUESTION_SPECS/QUESTION_BLOOM_CLASSIFICATION_SPEC.md).
- [ ] Reconcile existing Sysadmin access checks with full administrative authority, including
      Course and Student records. Authorize administrative operations through the Sysadmin role
      and apply the confirmation and audit rule below. Keep unproven tools deferred. See
      [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md).
      Update the old Sysadmin bullet wording in `devel/human_guidance_checklist.py`'s
      runtime-evidence requirement list when reconciling the implementation audit tooling.
- [ ] Add confirmation before a Sysadmin accesses FERPA-sensitive Student data, stating that
      access is needed for administrative work. Record that access for audit. Keep full Sysadmin
      authority. Verify that protected data appears after confirmation, cancelling leaves it
      undisclosed, and confirmed access is recorded. Inspect current UI, server access checks,
      and audit recording, then align the affected code and tests. This is intended behavior;
      implementation has not been verified in this documentation pass.
- [ ] Extend browser-facing Question authoring to use the shared Draft and publication services
      for PG/PGML as well as Native JSON. PG/PGML Draft creation and publication already exist in
      the content-loading tools; `crates/server/src/authoring.rs` currently accepts only Native
      JSON. Complete the same workflow: import or write a Draft, preview it, test it, refine it,
      add the metadata, then publish. The existing
      WeBWorK renderer and evaluator should serve Draft testing; the published-Question inspection
      preview does not prove that this authoring workflow works. Verify it end-to-end in later
      implementation work; see [DRAFT_QUESTION_SPEC.md](QUESTION_SPECS/DRAFT_QUESTION_SPEC.md).
- [ ] Carry the Blueprint Course Theme into a Course created from it; the Instructor can then
      change the Course Theme independently. Audit the existing create/adopt path and add the
      missing Blueprint Theme support. See [BLUEPRINT_COURSE_IMPORT_API_SPEC.md](BLUEPRINT_COURSE_IMPORT_API_SPEC.md).
- [ ] Carry shared Library Object fields through the combined result and browser rows. The
      [Library Object audit](active_plans/audits/library_object_documentation_audit_2026_10_05.md)
      records missing Question owner, Type, Backend, and Tags in the browser row and different
      shapes for common fields. Align the Rust/API/decoder/row/display boundary with
      [LIBRARY_OBJECT_SPEC.md](QUESTION_SPECS/LIBRARY_OBJECT_SPEC.md); retain kind-specific Revision
      and Pool fields. Verify common fields survive both result paths. Separate summary types
      are an implementation choice, not by themselves a defect.
- [ ] Apply shared Type search semantics to both Library Object kinds. In
      [question_library_operations.sql](../schemas/base_schema/50_functions/question_library_operations.sql),
      structured Type filtering includes Pools but Type text matching is Question-only. Align
      query handling and tests with each object's own Type. Keep Pool Author exclusion because
      Pools have no Author. This does not settle the tentative default result filter or mixed
      batch picker operations; Instructor bulk editing remains deferred.
- [ ] Verify and complete the supported Question import and Blueprint assembly path using
      generated PLE IDs and API readback. Existing content loaders already use shared Draft and
      publication services; direct-store/SQL fixtures do not establish the whole workflow. Track
      missing caller coverage against [QUESTION_IMPORT_SPEC.md](QUESTION_SPECS/QUESTION_IMPORT_SPEC.md)
      and [BLUEPRINT_COURSE_IMPORT_API_SPEC.md](BLUEPRINT_COURSE_IMPORT_API_SPEC.md). Preserve content,
      attribution, exact Question references, and reusable Pool references. Converter handoff
      remains deferred; use the external Rust library when that boundary is implemented.
- [ ] Correct stale saved-search comments and test names in
      [question_search.rs](../crates/question_model/src/question_search.rs) to describe ordinary
      filter normalization. Preserve useful parsing/round-trip checks. The targeted current
      search found terminology remnants, not a saved-search database table; remove any actual
      permanent prompt/result storage only if found when tracing callers. Search state remains
      temporary, as specified by HG.
- [ ] Finish clean-database verification of the filter removal: install the revised base schema
      and run the existing Question Library filtering and pagination test. The prior run was
      interrupted before completion; it is not passing evidence.
- [ ] Restart the Live Demo, verify Library search and Question pickers, and capture the affected
      screens. The Live Demo was stopped for database verification and remains stopped.

## Future product capabilities

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
