## M06 correction complete

Removed the inferred WeBWorK `hotspot` ban from the metadata Type guard while keeping Native Type bound to its source interaction. The WeBWorK editor now offers all eight defined Type tags, and the connected oracle covers saving `hotspot` without changing the Revision or source identity.

Fresh SPEC and QUALITY reviews passed with no in-scope blockers. The focused browser regression passed for Owner and Sysadmin, the metadata client check passed 2/2, and scoped Rust formatting passed.

The connected PostgreSQL test remains queued for the root batch. Its test target is currently blocked by unrelated `selection_rule` compile errors; workspace TypeScript and schema-style checks also report unrelated in-progress issues. No full-stack check ran, and no generated contracts or Git state were changed.

See the [M06 correction handoff](m06_type_tag_correction_cli.md), the [durable M06 report](../QUESTION_SPEC_M06_PERMISSIONS.md#L27), the [SQL guard](../../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql#L87), and the [Type selector](../../../../src/features/question_metadata/question_metadata_editor.tsx#L35). The separate Native Draft-source `hotspot` gate is documented for M15 review and remains unchanged.

