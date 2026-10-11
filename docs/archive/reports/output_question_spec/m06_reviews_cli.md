## M06 SPEC review status: BLOCK (historical; correction pending re-review)

The fresh **gpt-6-luna SPEC reviewer** found a source-flow mismatch. I did not start QUALITY because the required SPEC PASS was not reached.

The SQL loader returns Question Type alongside the metadata Edit Number ([question_library_operations.sql](../../../../schemas/base_schema/50_functions/question_library_operations.sql#L534), [same loader's result query](../../../../schemas/base_schema/50_functions/question_library_operations.sql#L575)). But the Rust caller's SELECT omits `question_type` ([question_library.rs](../../../../crates/learning-data-access/src/postgres/question_library.rs#L192)), and its decoder requires it ([question_library.rs](../../../../crates/learning-data-access/src/postgres/question_library.rs#L543)). The metadata editor calls this read before initializing its fields ([question_metadata_editor.tsx](../../../../src/features/question_metadata/question_metadata_editor.tsx#L89)). As written, the read fails before the editor can load Type and the matching Edit Number.

A fresh worker's read-only repair analysis identified the narrow correction: include `question_type` in that Rust SELECT. This is the current shared-metadata loader, separate from M24's search predicates. No schema or DTO change appears necessary.

Static review found the write boundary checks Instructor/Sysadmin role, ownership, availability, exact Revision, source Type, and metadata Edit Number before updating metadata ([published_question_metadata_operations.sql](../../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql#L51)). Direct table updates are limited to the private owner role ([question_lineages.sql](../../../../schemas/base_schema/70_grants/question_lineages.sql#L30)). I found no new grant for other Instructors to edit. The browser shows Native Type as derived text and offers a selector for WeBWorK ([question_metadata_editor.tsx](../../../../src/features/question_metadata/question_metadata_editor.tsx#L177)); publication rejects a Draft with NULL Type ([question_publication_operations.sql](../../../../schemas/base_schema/50_functions/question_publication_operations.sql#L164)).

## Handoff and limits

The statement that browser contract generation is pending is historical: the ignored generated
outputs `generated/api/QuestionMetadataReplacement.ts` and
`generated/api/PublishedQuestionSharedMetadata.ts` were observed to contain `questionType` in this
checkout. The durable Rust DTO definitions carry the corresponding field in
[`QuestionMetadataReplacement`](../../../../crates/question_model/src/question_metadata.rs#L13)
and [`PublishedQuestionSharedMetadata`](../../../../crates/question_model/src/question_library.rs#L202).
Generated-file freshness still needs the final contract-generation check. The connected PostgreSQL
assertion remains unrun ([M06 report](../QUESTION_SPEC_M06_PERMISSIONS.md#L41)); compiling the test
does not establish runtime behavior.

No files were edited. I ran no builds, tests, or database operations. After the source correction, use a fresh SPEC reviewer; start a different fresh QUALITY reviewer only after that review passes. M07, M24 search predicates, and M09 Pool fields remain separate handoffs.
