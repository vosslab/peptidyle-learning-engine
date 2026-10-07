# M09 Pool Save callable decision

Status: the `ple_api` wrapper and exact callable-role assertion are implemented. Fresh source SPEC
and QUALITY reviews passed. Connected PostgreSQL acceptance remains open because disposable schema
installation failed before oracle 09; see the [focused permission report](../QUESTION_SPEC_M09_POOL_SAVE_PERMISSION.md).

## Decision

D1 remains the product rule: the active owning Instructor or any active Sysadmin may save a Pool;
other Instructors and Students may not. This is the ordinary call path for that existing permission
check, not a new product permission, role, or workflow.

Expose one exact API entry point:
`ple_api.save_question_pool_members(text, bigint, text[], integer[])`. It is a thin
`SECURITY DEFINER` SQL wrapper owned by `ple_api_owner`, fixes `search_path` to `pg_catalog`, and
delegates by qualified name to `ple_data.save_question_pool_members`. Grant `ple_app` EXECUTE on the
wrapper. Keep the `PUBLIC` revoke and `ple_api_owner` EXECUTE grant on the data function, and remove
`ple_app` from that data-function ACL. Do not grant `ple_app` USAGE on `ple_data`.

The data-owner function remains authoritative for the active Instructor/Sysadmin check, Pool row
lock, Instructor-owner check, input validation, tuple and license rules, Edit Number CAS, and no-op
behavior. The API wrapper does not replace or relax those checks. The ordinary store calls the
exact `ple_api` wrapper; the `/api/question-pools/{id}/members` request remains unchanged.

## Ownership and handoff

- Callable-boundary worker: wrapper declaration and grants, store SQL namespace, and the focused
  catalog/app-invocation assertion in oracle 09.
- SQL permission lane: retain the existing active-role predicate, locked Pool-owner check, and
  owner/other-Instructor/Student/Sysadmin behavior assertions.
- M11: retain explicit Pool-fork helper hunks.
- M12: retain release-validation and post-issue hunks in shared Pool functions.
- Root: own connected PostgreSQL execution, final source-generation checks, and acceptance status.

See [M09_POOL_SAVE_PERMISSION_BOUNDARY.md](M09_POOL_SAVE_PERMISSION_BOUNDARY.md),
[M09_ORDINARY_POOL_EDITOR_BOUNDARY.md](M09_ORDINARY_POOL_EDITOR_BOUNDARY.md), and the
[M09 permission report](../QUESTION_SPEC_M09_POOL_SAVE_PERMISSION.md).

The affected rows in the [M09 permission ledger](M09_POOL_SAVE_PERMISSION_LEDGER.md) now reflect the
approved wrapper boundary and pending runtime status; no whole TODO or ledger rewrite was made.

## Revised handoff (2026-10-06)

- The exact wrapper declaration and grants, store SQL namespace, and oracle catalog/app-invocation assertion are implemented. Fresh source SPEC and separate QUALITY reviews passed; connected execution remains pending.
- The SQL permission lane retains the active-role predicate, locked Pool-owner check, and Instructor/Student/Sysadmin behavior assertions on the protected data function. M11 and M12 retain their separate helper and shared-function hunks.
- `ple_app` receives no `ple_data` schema USAGE. The exact `ple_api` wrapper is the application call boundary.
- Root owns connected PostgreSQL execution and acceptance status. The disposable install currently stops before oracle 09; see the focused report for the exact blockers. Final source-generation checks completed cleanly.
