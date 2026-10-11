# M09 Pool Save permission boundary

## Product rule

Ordinary Pool Save permits its owner Instructor and any Sysadmin to edit Pool metadata and members. Other Instructors and Students cannot save changes. Human Guidance assigns ownership to the creating Instructor, says edits take effect on Save, and gives Sysadmins full administrative authority. Pool forks have new owners. See [HUMAN_GUIDANCE.md](../../../HUMAN_GUIDANCE.md) sections "Question Pools" and "Administrative authority".

## Approved implementation decisions

Authenticate an active Instructor or Sysadmin from the existing session identity. Lock the target
Pool, then reject an Instructor whose actor ID differs from the Pool owner. Permit any Sysadmin.
Keep tuple validation, license calculation, classification constraints, compare-and-save
concurrency (CAS), and no-op behavior unchanged. Add no product role, product permission, or
workflow. The application call path is the exact `ple_api.save_question_pool_members(text, bigint,
text[], integer[])` wrapper in [ROOT_POOL_SAVE_CALLABLE_DECISION.md](ROOT_POOL_SAVE_CALLABLE_DECISION.md).
It is owned by `ple_api_owner`, fixes `search_path` to `pg_catalog`, and delegates to the protected
data function. `ple_app` receives EXECUTE on that wrapper; it receives neither EXECUTE on the data
function nor USAGE on `ple_data`.

The focused permission SQL oracle should retain the new Pool ID as psql's persistent `:'created_pool_id'` variable via `\gset`. The transaction-local `ple.shared_pool_id` is unavailable after COMMIT.

## Hunk ownership

- SQL worker: active-role predicate, locked Pool-owner check, behavior oracle, and test registration.
- Callable-boundary worker: owns the thin `ple_api` wrapper, exact app grant, and matching
  catalog/app-invocation assertion; connected runtime remains pending.
- Pool editor worker: ordinary Pool Save API/UI editor. Its store switches to `ple_app` and calls
  `ple_api.save_question_pool_members`. The HTTP request and product permission remain unchanged.
- M11: explicit Pool fork helpers.
- M12: release-validation and post-issue behavior in shared Pool functions.
- Root: disposable runtime execution, final source-generation checks, and acceptance evidence.

Keep these hunks distinct in shared files. Source integration of the app-role grant does not by
itself establish connected Save behavior. M09 permission acceptance does not accept the ordinary
editor API/UI outcome or connected runtime behavior. See
[ROOT_POOL_SAVE_CALLABLE_DECISION.md](ROOT_POOL_SAVE_CALLABLE_DECISION.md).

## Separate observed M12 limitation

The inspected `08_m12_release_validation.sql` references `:'shared_pool_id'` without a producer in the saved-response files inspected at task dispatch. This remains an M12-owned issue; this permission task does not change that oracle.
