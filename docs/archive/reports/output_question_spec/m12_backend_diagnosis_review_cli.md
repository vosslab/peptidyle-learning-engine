## Finding

**The latest claim is false as stated:** deleting only metadata does not fabricate a backend mismatch. The helper reads `member_backend` from `question_revision.backend`, while `metadata_question_id` comes from a separate LEFT JOIN. The backend rule compares only the member backend, Pool backend, and support result; missing metadata does not gate or null that comparison. A real backend mismatch remains reportable even when metadata is absent. [helper CTE](../../../../schemas/base_schema/50_functions/assessment_release_validation.sql#L62) [diagnostic rules](../../../../schemas/base_schema/50_functions/assessment_release_validation.sql#L109)

The earlier report's **missing Revision** case is different: a dangling member would produce a null revision backend and trigger the predicate. But that member state is invalid under the schema: Pool member revision keys are non-null and have a composite FK to `question_revision`; that table's backend is also `NOT NULL`. Metadata has a separate FK *to* the Revision, with no reciprocal requirement that each Revision have metadata. So missing metadata is representable; a missing Revision is not valid under the declared schema. [revision schema](../../../../schemas/base_schema/20_tables/published_question.sql#L28) [Pool member schema](../../../../schemas/base_schema/20_tables/question_pool.sql#L79)

Thus there is **no valid-state source defect and no need to couple Backend to metadata**. The prior report's diagnosis is true only for a dangling, FK-invalid member. If defensive handling for that impossible state is desired, the smallest guard is `member_backend IS NOT NULL AND (...)`; it would still report a genuine mismatch when metadata is missing.

## Oracle 08

The retired one-time oracle source `tests/e2e/assessment_saved_response/08_m12_release_validation.sql`
(line 170) deleted only the metadata row and checked that the Pool member and Revision remained. It
expected entry 92 to report unavailable plus insufficient items, entry 93 unavailable without
insufficiency, and no Type or classification mismatch. It **did not assert that backend mismatch
was absent**, despite the notice wording "without metadata mismatches." This historical one-time
proof is no longer durable test evidence.

That is an oracle coverage gap, not evidence of a current source bug. I made no edits and ran no tests or runtime checks; this is a read-only source trace.
