# Question Spec M06 Permissions Report

## Delivered

Question metadata replacement now includes Question Type under the same exact Revision Tuple and
metadata Edit Number used by Bloom and descriptive metadata. The SQL operation permits the Question
Owner or Sysadmin, rejects archived Questions, and keeps the stale-tuple/Edit-Number checks. Native
Type remains equal to `native_question_type` on the immutable source binding. WeBWorK Type is
editable classification metadata across all eight defined values, including `hotspot`. A tag
correction uses the ordinary metadata Edit Number, leaves the Question Revision and source binding
unchanged, and does not control WeBWorK rendering or grading.

Question library and current shared-metadata projections now read Type from
`question_revision_metadata`. The detail metadata editor initializes Type from its current metadata
snapshot. It displays Native Type as source-derived and provides every defined Type value in the
WeBWorK selector. Bloom replacement retains explicit `null` clearing for either nullable dimension
and follows the same owner/Sysadmin and metadata Edit Number rules.

Authoring Draft access and publication accept Instructor or Sysadmin sessions. A Sysadmin can create
and use their own authoring workspace, fork a Question into a Draft, and publish a successor for an
available Question while retaining its existing owner. Ordinary Instructors still require the
Question Owner permission for metadata and publication changes. Archived lineages remain read-only.

Fixtures were aligned with Type removal from `question_revision`, Type addition to metadata, and
Native source-binding Type. M09 owns the shared Pool SQL and its remaining fixture alignment.

## Verification

- The shared-metadata loader SELECT now includes `question_type`, matching the existing row
  decoder. A fresh SPEC review passed; see [m06_fix_spec.md](output_question_spec/m06_fix_spec.md).
- Fresh read-only SPEC and QUALITY reviews of the WeBWorK Type-tag correction passed with no
  in-scope blockers; see the [M06 Type correction handoff](output_question_spec/m06_type_tag_correction_cli.md).
- A fresh QUALITY review confirmed the source correction. Its initial QUALITY BLOCK was limited to
  stale report status; the narrow follow-up confirmed the corrected status and source-level QUALITY
  PASS. See [the metadata-read-fix report](output_question_spec/m06_metadata_read_fix.md),
  [the initial QUALITY review](output_question_spec/m06_fix_quality.md) (historical), and
  [the status follow-up](output_question_spec/m06_fix_status_quality_final.md).
- `source ./source_me.sh && cargo check -p question_model -p learning-data-access`: passed.
- `source ./source_me.sh && cargo check -p server_core`: passed after updating the Draft-image route
  to use the Instructor-or-Sysadmin authoring session resolver.
- `source ./source_me.sh && devel/generate_schema_tables_doc.py && schema_style/check_schema_style.py`:
  passed (`clean`).
- `source ./source_me.sh && node --import tsx tests/test_question_metadata_client.mjs && node --import tsx tests/test_library_classification_search.mjs`:
  passed (2 and 17 tests).
- Updated the current-snapshot Playwright regression to assert all eight WeBWorK Type options,
  select `hotspot`, and verify the saved request. The focused browser regression passed for Owner
  and Sysadmin projections; execution details are recorded in the
  [M06 Type correction handoff](output_question_spec/m06_type_tag_correction_cli.md).
- Scoped `rustfmt --check` on the modified Rust sources passed. Workspace-wide `cargo fmt --check`
  reported formatting changes across concurrent unrelated edits, which were left untouched.
- The connected PostgreSQL oracle refuses a Native Type change against its exact source binding and
  accepts a WeBWorK `hotspot` metadata correction. It checks the same Revision Tuple, metadata Edit
  Number increment, and stable Backend source identity. The acceptance test is ignored unless the
  disposable runtime is configured; execution is pending the coordinated root batch. The scoped
  Rust test target currently does not compile because concurrent `exchange` and `lineage_fork`
  tests still reference removed `selection_rule` fields; the focused correction module reported no
  additional compiler errors.
- `source ./source_me.sh && node --import tsx tests/test_question_metadata_client.mjs`: passed (2/2).
  `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json` remains blocked by a concurrent
  decoder/contract mismatch: `parentPublishedQuestionRevisionTuple` is not a member of
  `QuestionSummary` in `src/api/decoders/question_library.ts:117`.
- Scoped Rust formatting passed. The schema-style checker reports eight pre-existing findings in
  concurrent Draft-table work in `schemas/base_schema/20_tables/question_authoring.sql`; the
  correction changes no table definitions.

## Remaining acceptance and handoffs

Generated browser files already include the required `questionType` field on
`SaveQuestionMetadataRequest.metadata` and `PublishedQuestionSharedMetadata`; contract generation is
historical work, and final generator freshness is still pending with the root coordinator. The
focused metadata client check and Owner/Sysadmin browser regression passed; connected PostgreSQL
execution remains pending the coordinated root batch. The current connected fixture covers
owner/Sysadmin writes, another Instructor denial, stale CAS, archived read-only, Native source-Type
rejection, and WeBWorK Type correction. No full-stack build was run.

The current metadata editor exposes all eight WeBWorK Type tags. An integrated content-correction
Draft affordance remains part of M15/M16's authoring UI work; the server publication boundary now
supports Sysadmin correction while preserving the Question's existing owner.

Read-only review found no WeBWorK/`hotspot` exclusion in the publication validators. The M15 Draft
source creation function still rejects Native JSON plus `hotspot` at
`question_authoring_operations.sql:477`; this is a source-authoring gate, not the editable WeBWorK
metadata tag policy. It remains unchanged for M15 review. The Native HOTSPOT image binding checks
and M07 publication functions also remain unchanged.
