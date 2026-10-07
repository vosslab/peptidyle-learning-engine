# Final API and schema document alignment

Date: 2026-10-06

## Scope and authority

This report covers the M29 API and schema documentation subtask in
[question_spec_implementation_plan.md][plan], approved for this task.
[HUMAN_GUIDANCE.md][human-guidance] and that plan set product authority.
Current Rust routes, generated DTOs, and the base schema provide implementation
evidence. This task changed only [API_CONTRACTS.md][api-contracts], regenerated
[SCHEMA_TABLES.md][schema-tables], and this report.

## Corrections made

- Added the current Question Pool create and detail routes, their request and
  receipt fields, and the rule that member order is not stored.
- Added the owner/Sysadmin write boundary for Question and Pool metadata and
  Pool members. Replaced an obsolete M09/M12 gate note with a source/runtime
  distinction.
- Added required `questionType` to the complete Question metadata replacement.
  The API uses `PublishedQuestionRevisionTuple` for the exact Question Revision
  wire value; that stable transport type was retained.
- Added the current Draft metadata, image, preview, and test routes. The M15
  publication and autosave work remains active, so these entries describe the
  route and DTO source shape without claiming runtime acceptance.
- Clarified that Library metadata is stored on the Question record, separately
  from Native JSON content. Native JSON `questionType` must agree with its
  source Type.
- Added Sysadmin access to exact Question Revision reads and listed the
  confirmed Student-data request field and audit receipt fields.
- Documented `policies.partialCreditEnabled`, its default for new Assessments,
  immutable stored Backend credit fractions, and current point recalculation.

## Generated schema evidence

The canonical generator rebuilt `SCHEMA_TABLES.md` from the current schema
source. It records Question metadata separately in
`ple_data.question_revision_metadata`, with required `question_type` and two
independently nullable Bloom columns. `ple_data.question_pool` has its own
metadata and Bloom columns. `ple_data.question_pool_member` keys one exact
Question Revision per Question in the Pool. `ple_private.grading_result` stores
`normalized_credit` between zero and one, and the Assessment policy snapshot
stores `partial_credit_enabled`.

## Source and runtime boundary

The route and DTO statements above were checked against the current Rust
registrations and contract types. Schema facts were checked through the
generated schema document and base-schema source. No connected database, Live
Demo, or container was started for this documentation task; source evidence is
not reported as connected runtime acceptance.

## Handoff discrepancies resolved by current source review

- Pool creation now accepts each member as a `PublishedQuestionRevisionTuple`,
  matching the owned API contract. The current route DTO is defined in
  [question_pool_creation.rs][pool-creation-rust]. No tuple-shape discrepancy
  remains in this handoff.
- The 15-argument metadata save belongs to Drafts:
  `save_authoring_draft_general_feedback` in
  [question_authoring_operations.sql][draft-metadata-sql]. It is separate from
  the 14-argument Published Question replacement,
  `replace_published_question_metadata`, in
  [published_question_metadata_operations.sql][metadata-sql]. The PostgreSQL
  Published Question Store binds 13 values and supplies literal `NULL` for the
  language argument, for 14 SQL arguments ([question_metadata.rs][metadata-rust]).
  These are separate functions and their arities do not conflict.
- [QUESTION_BLOOM_CLASSIFICATION_SPEC.md][bloom-spec] now specifies ordinary
  Question and Pool metadata saves with independently nullable dimensions.
  The current request requires both fields but each may be `null`; no complete-
  pair validator or dedicated correction route remains. See the
  [M05 API documentation report][m05-api-report] for route and SQL source
  evidence. Connected runtime acceptance remains pending.
- Deferred product choices remain deferred, including initial AI Bloom
  assignment, bulk metadata editing, and Question Feedback timing.

## Verification

- `source source_me.sh && devel/generate_schema_tables_doc.py && schema_style/check_schema_style.py`
  - PASS. Generator wrote `docs/SCHEMA_TABLES.md` and
    `schemas/catalog_snapshot.json`; schema style reported `clean`.
- `source source_me.sh && pytest -q tests/test_markdown_links.py -k 'API_CONTRACTS or SCHEMA_TABLES'`
  - PASS: 2 passed, 520 deselected. The root `output_question_spec/`
  directory is ignored, so
  the link checker does not scan this report; its local targets were checked
  against the current paths.
- The earlier full Markdown link run failed 508 passed / 12 failed. Its
  affected documentation references have since been aligned to durable report
  copies or current source paths. This continuation did not rerun pytest.
- A separate manual local-target scan checked 21 current/assigned Markdown
  documents; all local targets existed. The scan does not replace the repository
  Markdown-link test.
- Focused ISO-8859-1 checks passed for all three owned files. Markdown section
  checks found no section over 25 bullets in the API map or this report.
- A fresh SPEC review confirmed the Pool-create tuple mismatch recorded above
  and found no other assigned-contract drift.
- `git diff --check -- docs/API_CONTRACTS.md docs/SCHEMA_TABLES.md` passed;
  the report has no trailing whitespace. All seven report links resolve.

[plan]: ../../active/question_spec_implementation_plan.md
[human-guidance]: ../../../HUMAN_GUIDANCE.md
[api-contracts]: ../../../API_CONTRACTS.md
[schema-tables]: ../../../SCHEMA_TABLES.md
[metadata-sql]: ../../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql
[metadata-rust]: ../../../../crates/learning-data-access/src/postgres/question_metadata.rs
[draft-metadata-sql]: ../../../../schemas/base_schema/50_functions/question_authoring_operations.sql
[pool-creation-rust]: ../../../../crates/server/src/question_pool_creation.rs
[m05-api-report]: ../QUESTION_SPEC_M05_API_DOCS.md
[bloom-spec]: ../../../QUESTION_SPECS/QUESTION_BLOOM_CLASSIFICATION_SPEC.md
