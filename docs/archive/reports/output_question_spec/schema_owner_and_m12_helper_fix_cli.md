# Root M12 schema-owner and helper report

## Correction (2026-10-06)

The earlier report treated missing metadata as if it made the member backend unknown and proposed a
metadata-presence guard. The current source and schema show that premise was wrong for valid database
states. The context-checked helper patch did not apply, no helper edit was made, and the proposed
root helper edit is unnecessary and closed.

`assessment_pool_release_issues` reads `member_backend` from `question_revision.backend` and reads
metadata through a separate `LEFT JOIN`. Its backend diagnostic compares the member backend, Pool
backend, and backend support; it does not depend on metadata. The current helper is correct for this
state model, and backend diagnostics must remain independent of metadata. A missing metadata row is
a representable unavailable member and does not suppress a genuine backend mismatch. A missing
Revision would yield a NULL backend, but `question_pool_member` has non-null exact Revision fields
and a composite foreign key to `question_revision`; that invalid state is prevented by the schema.
See [m12_backend_diagnosis_review_cli.md](m12_backend_diagnosis_review_cli.md),
[50_functions/assessment_release_validation.sql](../../../../schemas/base_schema/50_functions/assessment_release_validation.sql#L62),
[20_tables/question_pool.sql](../../../../schemas/base_schema/20_tables/question_pool.sql#L79), and
[20_tables/published_question.sql](../../../../schemas/base_schema/20_tables/published_question.sql#L28).

The retired one-time Oracle 08 source
`tests/e2e/assessment_saved_response/08_m12_release_validation.sql` (line 141) exercised missing
metadata: it deleted only the metadata row, verified the member and Revision remained, checked
unavailable and entry-specific insufficiency results, and rejected Type and classification
mismatches. The prior statement that there was no exact missing-metadata assertion was stale. That
oracle did not separately assert that a backend mismatch was absent; this historical one-time proof
is no longer durable test evidence and does not establish a source defect.

## Retained source evidence

- The completed checksum-owner correction remains in [10_types.sql](../../../../schemas/base_schema/10_types.sql#L243):
  `ple_private_owner` owns the checksum functions, and the file restores `ple_data_owner` before
  creating public-ID domains.
- The helper grant file revokes PUBLIC execution and grants the helper to `ple_api_owner`; no
  `ple_app` grant was added. See
  [70_grants/assessment_release_validation.sql](../../../../schemas/base_schema/70_grants/assessment_release_validation.sql#L12).
- The helper, oracle 08, metadata-correction assertions, and fairness work remain owned by the first
  M12 manager. Other managers retain their source, oracle, and main M12 report edits.

The source trace was read-only. No source, schema, oracle, test, or runtime work was performed for
this documentation correction. The earlier SQL parse, schema-style, and scoped whitespace results
remain historical evidence; they were not rerun here. Connected canonical fresh-database acceptance
remains pending.
