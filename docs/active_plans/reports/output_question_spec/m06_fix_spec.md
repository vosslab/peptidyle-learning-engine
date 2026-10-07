# SPEC PASS

The original shared metadata read mismatch is fixed. The SQL result includes `question_type` at [question_library_operations.sql:529](../../../../schemas/base_schema/50_functions/question_library_operations.sql#L529) and selects it at [question_library_operations.sql:570](../../../../schemas/base_schema/50_functions/question_library_operations.sql#L570). The Rust caller now selects it at [question_library.rs:195](../../../../crates/learning-data-access/src/postgres/question_library.rs#L195), matching the decoder's required read at [question_library.rs:544](../../../../crates/learning-data-access/src/postgres/question_library.rs#L544).

The reviewed permission boundary matches the authority: metadata writes require the Question Owner or Sysadmin, check the exact Revision and Edit Number, and enforce Native Type against the source binding ([published_question_metadata_operations.sql:51](../../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql#L51), [published_question_metadata_operations.sql:65](../../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql#L65), [published_question_metadata_operations.sql:87](../../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql#L87), [published_question_metadata_operations.sql:120](../../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql#L120)). Human Guidance says Native Type comes from its built-in interaction while WeBWorK Type is assigned manually ([HUMAN_GUIDANCE.md](../../../HUMAN_GUIDANCE.md#L1124), [HUMAN_GUIDANCE.md](../../../HUMAN_GUIDANCE.md#L1127)). The shared read admits Instructors and Sysadmins ([question_library_operations.sql:540](../../../../schemas/base_schema/50_functions/question_library_operations.sql#L540)).

The PostgreSQL scenario contains assertions for Type and metadata Edit Number reads by both Sysadmin and another Instructor ([question_revision_metadata.rs:559](../../../../crates/learning-data-access/tests/blueprint_course_postgres/question_revision_metadata.rs#L559), [question_revision_metadata.rs:561](../../../../crates/learning-data-access/tests/blueprint_course_postgres/question_revision_metadata.rs#L561), [question_revision_metadata.rs:580](../../../../crates/learning-data-access/tests/blueprint_course_postgres/question_revision_metadata.rs#L580), [question_revision_metadata.rs:582](../../../../crates/learning-data-access/tests/blueprint_course_postgres/question_revision_metadata.rs#L582)). It also expects another Instructor's write to be denied ([question_revision_metadata.rs:375](../../../../crates/learning-data-access/tests/blueprint_course_postgres/question_revision_metadata.rs#L375)).

M24 search predicates remain separate: the shared metadata read calls its own loader, while search uses a separate Rust method and SQL predicates ([question_library.rs:198](../../../../crates/learning-data-access/src/postgres/question_library.rs#L198), [search.rs:29](../../../../crates/learning-data-access/src/postgres/question_library/search.rs#L29), [question_library_operations.sql:266](../../../../schemas/base_schema/50_functions/question_library_operations.sql#L266)).

The ignored generated outputs `generated/api/QuestionMetadataReplacement.ts` and
`generated/api/PublishedQuestionSharedMetadata.ts` were observed to contain `questionType` in this
checkout. The durable Rust DTO definitions also carry the corresponding `question_type` field in
[`QuestionMetadataReplacement`](../../../../crates/question_model/src/question_metadata.rs#L13)
and [`PublishedQuestionSharedMetadata`](../../../../crates/question_model/src/question_library.rs#L202).
The reports accurately record the observed generated presence while generator freshness remains
pending ([m06_metadata_read_fix.md](m06_metadata_read_fix.md#L11),
[QUESTION_SPEC_M06_PERMISSIONS.md](../QUESTION_SPEC_M06_PERMISSIONS.md#L49)).

## Pending acceptance

- Connected PostgreSQL execution remains pending. The scenario is marked ignored unless its disposable acceptance runtime is configured ([question_revision_metadata.rs:276](../../../../crates/learning-data-access/tests/blueprint_course_postgres/question_revision_metadata.rs#L276)); its assertions are source evidence, not runtime results.
- Generated-contract freshness and focused TypeScript/client checks remain pending ([QUESTION_SPEC_M06_PERMISSIONS.md](../QUESTION_SPEC_M06_PERMISSIONS.md#L47), [QUESTION_SPEC_M06_PERMISSIONS.md](../QUESTION_SPEC_M06_PERMISSIONS.md#L50)).
- The current PostgreSQL scenario does not exercise a WeBWorK Type correction end to end ([QUESTION_SPEC_M06_PERMISSIONS.md](../QUESTION_SPEC_M06_PERMISSIONS.md#L53)).

No files were edited; I ran no tests and accessed no database. Repository guidance specifies `source source_me.sh && python3` for Python commands ([AGENTS.md](../../../../AGENTS.md#L37)).
