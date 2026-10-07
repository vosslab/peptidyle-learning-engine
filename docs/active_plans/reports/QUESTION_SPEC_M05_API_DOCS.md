# M05 API contract documentation correction

The API contract now describes the checked-in ordinary Question and Pool metadata routes and
their JSON fields. It no longer describes a dedicated Bloom correction route or classification
counter. This report records source evidence for the documentation change; it makes no runtime
acceptance claim.

Question metadata evidence:

- `crates/server/src/question_library.rs` registers `GET /api/questions/by-id/{question_id}/detail`
  and `POST /api/questions/bulk-metadata/current`.
- `crates/server/src/question_library/shared_metadata.rs` reads the posted `questionIds` and
  returns `items`; `crates/question_model/src/question_library.rs` defines each item with
  `questionId`, `metadataEditNumber`, and the editable metadata fields.
- `crates/server/src/question_metadata.rs` registers
  `POST /api/questions/by-id/{question_id}/metadata`.
- `crates/question_model/src/question_metadata.rs` defines its request as
  `publishedQuestionRevisionTuple`, `expectedMetadataEditNumber`, and `metadata`. The complete
  replacement has independently nullable `bloomCognitiveProcess` and
  `bloomKnowledgeDimension` fields.

Pool metadata evidence:

- `crates/server/src/question_pool_library.rs` registers
  `GET` and `PUT /api/question-pools/{question_pool_id}/metadata`.
- `crates/question_model/src/question_pool_library.rs` defines the GET view, PUT request, and
  complete replacement. Both Bloom dimensions are independently nullable.

The M18 follow-up in `docs/TODO.md` now records that Matching grading is proportional and accepts
incomplete responses in source. Connected saving, submission, shuffled identity, points, and
Student display checks remain open.

The implementation evidence is `crates/grading/src/ple_question_json_validate.rs`, where Matching
credit counts correct prompt-choice pairs and divides by the full prompt count, and
`crates/grading/src/ple_question_json.rs`, whose partial-response case expects `0.6` for three of
five pairs and `0.0` for an empty response. This is source evidence only.

Documentation checks performed: links added by this change resolve, and all three edited Markdown
files are ASCII. Other concurrent edits were present in `docs/API_CONTRACTS.md` and `docs/TODO.md`
and were preserved. No tests or runtime checks were run.

## Ordinary nullable Bloom metadata follow-up

The unchecked Bloom follow-up in `docs/TODO.md` now points to the current ordinary metadata writes.
The former `question_bloom.sql` link named removed SQL. Current source evidence:

- [published_question_metadata_operations.sql](../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql)
  accepts both Bloom dimensions in the ordinary Question metadata replacement, writes them to the
  current Question Revision metadata row, checks the Revision Tuple and metadata Edit Number, and
  advances the metadata Edit Number.
- [question_pool_search_metadata.sql](../../../schemas/base_schema/50_functions/question_pool_search_metadata.sql)
  accepts both Bloom dimensions in the ordinary Pool metadata replacement, writes them to the Pool
  record, and advances its metadata Edit Number.
- The storage fields are nullable in
  [published_question.sql](../../../schemas/base_schema/20_tables/published_question.sql) and
  [question_pool.sql](../../../schemas/base_schema/20_tables/question_pool.sql). Field rules are
  specified in [QUESTION_LIBRARY_METADATA_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_METADATA_SPEC.md#shared-field-table).

The unchecked follow-up remains for verification of one-dimension corrections, unchanged Question
Revision, ordinary concurrency, owner/Sysadmin permissions, and Assessment sorting with absent
Bloom. This is static source and documentation evidence only; no runtime acceptance is claimed.

A fresh read-only review flagged that the TODO described unclassified placement only as an
implementation detail. The approved [M05 implementation plan](../active/question_spec_implementation_plan.md#m05-make-bloom-ordinary-metadata)
specifies sorting unclassified entries last, so the TODO retains that settled placement for
verification. The review confirmed the other scoped links and evidence, and that no runtime
acceptance is claimed.
