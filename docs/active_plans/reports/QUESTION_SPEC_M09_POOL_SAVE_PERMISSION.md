# M09 Pool Save permission and callable integration

Status: the exact `ple_app` grant on the API wrapper and focused callable-role oracle assertion
are implemented. Fresh source SPEC and QUALITY reviews passed. Root's disposable PostgreSQL
installation failed before oracle 09, so database authorization and Save acceptance remain open.

## Authority and implementation

Human Guidance assigns a Pool to its creating Instructor and gives Sysadmins full administrative
authority. The active Pool owner Instructor or any active Sysadmin may save it; another Instructor
and a Student may not. The existing SECURITY DEFINER function checks active role, locks the Pool,
and checks its Instructor owner before writing. Tuple validation, license calculation,
classification, Edit Number concurrency, and no-op behavior stay unchanged.

The ordinary editor store switches to `ple_app` and calls the exact
`ple_api.save_question_pool_members(text, bigint, text[], integer[])` wrapper. The wrapper is a
`SECURITY DEFINER` SQL function owned by `ple_api_owner`, fixes `search_path` to `pg_catalog`, and
delegates by qualified name to the protected data function. The grant file revokes `PUBLIC` and
grants `ple_app` EXECUTE only on the API wrapper; the data function grants EXECUTE only to
`ple_api_owner`. `ple_app` has no `ple_data`
USAGE. The wrapper and grant declarations, store call, and catalog/app-call oracle assertion are
present in source; connected execution remains root-owned and pending. No product permission,
role, or workflow was added. The root decision and ownership boundaries are in
[ROOT_POOL_SAVE_CALLABLE_DECISION.md](output_question_spec/ROOT_POOL_SAVE_CALLABLE_DECISION.md),
[M09_POOL_SAVE_PERMISSION_BOUNDARY.md](output_question_spec/M09_POOL_SAVE_PERMISSION_BOUNDARY.md),
and [M09_ORDINARY_POOL_EDITOR_BOUNDARY.md](output_question_spec/M09_ORDINARY_POOL_EDITOR_BOUNDARY.md).

## Oracle coverage

[`09_pool_save_permissions.sql`](../../../tests/e2e/assessment_saved_response/09_pool_save_permissions.sql)
asserts the explicit `ple_app` EXECUTE grant on the exact API wrapper, its owner, `SECURITY DEFINER`
property, and exact fixed search path. It also checks `PUBLIC` denial on both functions, no direct
or effective app EXECUTE on the data function, and no app `ple_data` USAGE. It invokes the wrapper
for a no-op Save while `SET LOCAL ROLE ple_app`, then checks owner Save/no-op,
non-owner Instructor and Student denial with unchanged members and Edit Number, Sysadmin Save, and
stale Edit Number rejection on the protected data function under the SQL permission lane. The
oracle is registered in
[`assessment_saved_response_oracle.sql`](../../../tests/e2e/assessment_saved_response_oracle.sql).
Its Pool fixture uses the persistent psql variable `:'created_pool_id'` from the earlier `\gset`
fixture.

The ordinary editor store and API/UI remain owned by the editor lane. M11 owns explicit Pool-fork
helpers; M12 owns release-validation and post-issue function hunks. Root owns the disposable runtime
and final source-generation checks. No M11 or M12 hunk is attributed to this callable-role change.

## Source evidence and pending runtime

- Callable-boundary checks: PostgreSQL SQLFluff parsing passed for the API function, grants, and
  oracle; scoped schema style returned `clean`; focused compile checks passed for the store and
  server lane. Fresh source SPEC and separate QUALITY reviews passed.
- Root's `devel/generate_schema_tables_doc.py` and `schema_style/check_schema_style.py` run completed
  clean. A small PostgreSQL 17 probe confirmed the `pg_proc` owner, `prosecdef`, and one-element
  `proconfig` representation expected by the wrapper assertion. It did not install or call the M09
  wrapper, run its fixture, or exercise authorization behavior.
- The canonical disposable install stopped in `10_types.sql:252` with
  `permission denied for schema ple_private`. A disposable-only install attempt with a temporary
  setup grant advanced to a separate parse failure in `question_library_operations.sql:464`
  (`mismatched parentheses`). No
  repository source was changed for either attempt, and oracle 09 did not run.
- Connected owner/Sysadmin success, Instructor/Student denial, and application-role Save remain
  unverified. No full build or browser/API acceptance was run.

The affected rows in the [M09 permission ledger](output_question_spec/M09_POOL_SAVE_PERMISSION_LEDGER.md)
now reflect the wrapper boundary and pending runtime status. No whole TODO or ledger rewrite was made.

The separate M12 observation remains M12-owned: `08_m12_release_validation.sql` refers to the psql
variable `:'shared_pool_id'`, while the existing earlier fixture sets a transaction-local GUC. This
report neither changes nor accepts that M12 oracle. See the
[permission boundary](output_question_spec/M09_POOL_SAVE_PERMISSION_BOUNDARY.md) and
[root callable decision](output_question_spec/ROOT_POOL_SAVE_CALLABLE_DECISION.md).
