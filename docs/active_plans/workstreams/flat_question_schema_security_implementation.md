# Flat-question schema security implementation

> **Historical workstream record.** This package is retained as implementation evidence, not
> current task direction. Current authority is the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

## Scope

This change implements the database-only private-grading boundary for the
closed `flat_single_choice_v1` native family. It changes only the pre-data
catalog and operations migrations; it does not add a browser-visible API.

## Security contract

- `ple_flat_question_grading_material(tenant, problem, version)` is a
  `SECURITY DEFINER` capability owned by `ple_grader`. It accepts only the
  caller's tenant context and returns only `answer_key.key_payload` and
  `answer_key.key_sha256`.
- The function permits a row only when the published version is `native`, its
  answer-free published model declares `flat_single_choice_v1`, and the
  version is public or has a grant for the supplied tenant. QTI and every
  other native family therefore return no answer material through this path.
- `PUBLIC` has no execute privilege. Only `ple_grading_reader` receives
  execute privilege, and that login retains no direct table grants.
- The definer receives a narrowly scoped RLS-protected read policy and
  `SELECT` on the answer-free `problem_version_payload` solely to prove the
  family. `answer_key` remains forced RLS with its existing grader policy.
- Promotion accepts only the bounded three-field canonical-byte envelope:
  `publicSha256`, `payloadSha256`, and `payloadBase64`. The base64 field is
  limited to the encoded form of the 256 KiB flat private-payload maximum;
  its checksum is independently verified by the grader store before use.

## Promotion serialization

`ple_promote_flat_question_grading` locks `workspace_draft`, not the source
binding. The application source write path takes that same draft lock, and a
draft change clears its stale source binding through
`ple_clear_workspace_flat_question_source`. Keeping a second `FOR UPDATE` on
the source would require a broader `UPDATE` grant for the security-definer
owner without protecting an independent mutation path.

PostgreSQL requires `UPDATE` privilege to acquire `FOR UPDATE`, so
`ple_grader` receives only `SELECT, UPDATE(updated_at)` on
`workspace_draft`. The function never changes `updated_at`; this
non-semantic column grant exists only for row locking, while the forced tenant
RLS policy continues to constrain the definer's visibility.

## Validation

- `git diff --check -- schemas/migrations/2026080802_catalog_authoring.sql schemas/migrations/2026080805_operations_analytics.sql` passed.
- `cargo check -p learning-data-access --features postgres` passed.
- A targeted live role-denial proof is still required: call this function as
  `ple_grading_reader`, prove app/student direct access is denied, and prove
  QTI and another native family return no row.

## Integration note

The PostgreSQL store implementation queries the function with
`SELECT key_payload, key_sha256 FROM ple_flat_question_grading_material($1,
$2, $3)`. The function's returned column names intentionally match that query.
