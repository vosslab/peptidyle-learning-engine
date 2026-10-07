# M09 Pool Save permission ledger

Status: the SQL permission slice and approved `ple_api` callable path passed fresh source SPEC and QUALITY review. Connected PostgreSQL acceptance remains open because disposable installation failed before oracle 09.

## Task and dependencies

| Work | Owner | State | Dependency/evidence |
|---|---|---|---|
| SQL owner-check predicate and focused permission oracle/registration | SQL worker | Implemented; SPEC/QUALITY/integration accepted | D1; oracle uses persistent `:'created_pool_id'` from `\gset` |
| Ordinary Pool Save API/UI editor | Editor worker | Source integration complete; runtime pending | Store switches to `ple_app` and calls the exact `ple_api` wrapper. `ple_app` gets EXECUTE only on that wrapper; it has no `ple_data` USAGE or direct data-function EXECUTE. |
| Explicit Pool fork helpers | M11 owner | Separate ownership | Keep fork hunks distinct |
| Release/post-issue shared-function hunks | M12 owner | Separate ownership | Keep release hunks distinct; see limitation below |
| Connected runtime execution | Root | Attempted; blocked before oracle 09 | Disposable schema install failed before Pool Save assertions; see the latest evidence rows below |
| Fresh SPEC review | Root-assigned reviewer | Passed for approved wrapper path | Verified exact wrapper ACL, pg_catalog-only SECURITY DEFINER wrapper, direct `ple_app` invocation in oracle, and retained role behavior assertions |
| Fresh QUALITY review | Different root-assigned reviewer | Passed for approved wrapper path | Verified least-privilege boundary and catalog assertions; connected runtime remains open |

## Decision D1 and propagation

Authenticate an active Instructor or Sysadmin from existing session identity, lock the target Pool,
reject an Instructor whose actor ID is not the Pool owner, and permit any Sysadmin. Preserve tuple,
license, classification, CAS, and no-op behavior. Add no product role, permission, or workflow. Root
authorized the ordinary application call path through the exact `ple_api` SECURITY DEFINER wrapper;
the earlier no-new-grant wording did not prohibit this infrastructure EXECUTE grant. The editor
store calls that wrapper as `ple_app`. The exact SQL signature, call shape, return behavior, and
focused oracle paths are in the report.

## Evidence and review log

| Date | Owner | Evidence or decision | Result |
|---|---|---|---|
| 2026-10-06 | Root | D1 and ownership split confirmed; persistent `:'created_pool_id'` required for permission fixture | Confirmed |
| 2026-10-06 | Docs worker | Read M09-M12 plan section, root sorting decision, and changelog; drafted boundary and cross-reference | Complete; no implementation claim |
| 2026-10-06 | SQL worker | Implemented owner-check permission gate and focused oracle; reported schema style, normalized SQLFluff parse, schema module outer parse, shell syntax, and scoped whitespace checks passed | Connected PostgreSQL/runtime acceptance not run; local `psql` unavailable |
| 2026-10-06 | Editor worker | Traced data-access Save through `ple_app` calling the function directly; only `ple_api_owner` has EXECUTE and no `ple_api` wrapper exists | Historical pre-wrapper evidence; superseded by the approved 2026-10-06 callable-wrapper decision and current source integration |
| 2026-10-06 | SPEC reviewer | Full Pool Save path review: NOT COMPLIANT; SQL-slice SPEC recheck: COMPLIANT | Historical pre-wrapper review; superseded by fresh post-wrapper source SPEC approval below. PostgreSQL execution was not performed |
| 2026-10-06 | Root | `source ./source_me.sh && ./schema_style/check_schema_style.py -s schemas/base_schema` -> exit 0, clean; SQLFluff PostgreSQL parse of `question_pools.sql` and `09_pool_save_permissions.sql` using temporary `/tmp/m09-sqlfluff.cfg` (`large_file_skip_byte_limit = 0`) -> exit 0; `bash -n tests/e2e/e2e_assessment_saved_response.sh` -> exit 0; scoped `git diff --check` -> exit 0 | Historical check; at that point `source source_me.sh && command -v psql` -> exit 1. Later disposable PostgreSQL 17 setup attempts and their exact blockers are recorded below; actual oracle execution remains pending |
| 2026-10-06 | QUALITY reviewer | Accepted the bounded SQL permission slice | Historical pre-wrapper review; superseded by fresh post-wrapper security approval below |
| 2026-10-06 | Root | Independent integration review | Historical pre-wrapper integration status; superseded by the current wrapper decision/source review. Connected runtime remains unverified; see disposable install attempts below |
| 2026-10-06 | Root | Fresh PostgreSQL 17 disposable install attempt | Canonical install failed at `10_types.sql:252` with `permission denied for schema ple_private`; a disposable-only setup grant advanced to a separate parse failure at `question_library_operations.sql:464`; oracle 09 did not run and repository source was unchanged |
| 2026-10-06 | Root | `devel/generate_schema_tables_doc.py` and `schema_style/check_schema_style.py` | Generated schema tables and catalog snapshot; style result clean |
| 2026-10-06 | SPEC reviewer | Fresh source review after wrapper hardening | Approved wrapper, exact role grants, app call, and owner/Instructor/Student/Sysadmin oracle coverage; runtime remains open |
| 2026-10-06 | QUALITY reviewer | Fresh security review after wrapper hardening | Approved scoped `pg_catalog` search path and exact catalog assertion; runtime remains open |

## Separate M12 observation

At dispatch, `08_m12_release_validation.sql` referenced `:'shared_pool_id'` without a producer in
the saved-response files inspected. This is a separate M12-owned limitation. Do not repair or attribute
it to M09 permission work.
