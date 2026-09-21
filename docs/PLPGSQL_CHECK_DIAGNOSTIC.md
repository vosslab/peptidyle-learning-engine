# One-time `plpgsql_check` diagnostic

This is a disposable diagnostic receipt, not a permanent acceptance gate. It
records the 2026-09-20 experiment and the 2026-09-21 current-source rerun against
the PostgreSQL 17 base schema.

## Reproducing the container

The normal PostgreSQL service image does not include `plpgsql_check`. The
migrator image has `psql`, but it is not the PostgreSQL server image and cannot
load a server extension. For this experiment, a temporary derivative image was
built from the current pinned PostgreSQL image:

```Dockerfile
FROM docker.io/library/postgres@sha256:7958605b474b3d264a969cb3a123d6aa00ad1e1fe9da8a69984dabb704d93317

USER root
RUN apt-get update \
    && apt-get install --no-install-recommends --yes build-essential ca-certificates git postgresql-server-dev-17 \
    && git clone --depth 1 --branch v2.9.0 https://github.com/okbob/plpgsql_check.git /tmp/plpgsql_check \
    && make -C /tmp/plpgsql_check USE_PGXS=1 \
    && make -C /tmp/plpgsql_check USE_PGXS=1 install \
    && rm -rf /tmp/plpgsql_check /var/lib/apt/lists/*

USER postgres
```

`postgresql-server-dev-17` pulled the current PostgreSQL 17.11 packages while
building the extension. The diagnostic therefore used PostgreSQL 17 semantics,
but not the exact patch release of the pinned service image. The temporary
image was tagged
`localhost/ple-postgres-plpgsql-check:cleanup-20260921` for the rerun.

The disposable database was prepared as follows:

1. Build the Dockerfile above and start the temporary image with an empty data
   directory and the repository mounted read-only at `/workspace`.
2. Create `ple_plpgsql_check` as the administrator.
3. Generate the repository's normal role bootstrap with
   `local_stack_control.lifecycle_database.migration_principal_bootstrap_sql`.
4. Load `schemas/base_schema/install.sql` as `ple_migrator` with `psql -X
   --single-transaction -v ON_ERROR_STOP=1`.
5. As the administrator, run `CREATE EXTENSION plpgsql_check`.
6. Run `plpgsql_check_function_tb` over every `plpgsql` function in `ple_%`
   schemas. Trigger functions were checked again with their actual trigger
   relation supplied.

The extension was compiled from the upstream `v2.9.0` tag. This image and the
container were intentionally not added to `containers/compose.yaml`,
`containers/Containerfile.api`, or `launchers/all_test.sh`.

The `--single-transaction` flag is important: the install uses `SET LOCAL ROLE`
while creating objects, so loading the file as separate `psql` transactions can
lose the migration role and fail partway through. The checker functions are in
the `public` schema and must be called as the PostgreSQL administrator (the
extension's default function privileges do not grant the migration role
access). Ordinary functions are passed as `p.oid::regprocedure`; trigger
functions are checked a second time with their attached `tgrelid::regclass`.
The call enables `fatal_errors => false`, `other_warnings => true`,
`performance_warnings => false`, and `extra_warnings => true`.

The exact one-time shell sequence was equivalent to the following. It keeps the
extension image, database data, and result outside the repository and records the
checker output on the host:

```sh
DIAGNOSTIC_DIR=$(mktemp -d)
cp /private/tmp/ple-plpgsql-check/Dockerfile "$DIAGNOSTIC_DIR/Dockerfile"
IMAGE=localhost/ple-postgres-plpgsql-check:cleanup-20260921
CONTAINER=ple-plpgsql-check-20260921
podman build --pull=never -t "$IMAGE" "$DIAGNOSTIC_DIR"
podman run --detach --rm --name "$CONTAINER" \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_DB=ple_plpgsql_check \
    -v "$PWD:/workspace:ro" "$IMAGE"
podman exec "$CONTAINER" pg_isready -U postgres -d ple_plpgsql_check
source source_me.sh
python3 -c "import local_stack_control.lifecycle_database as d; print(d.migration_principal_bootstrap_sql('ple_plpgsql_check', 'migratorsecret'), end='')" \
    | podman exec -i "$CONTAINER" psql -X -v ON_ERROR_STOP=1 -U postgres -d ple_plpgsql_check
podman exec --env PGPASSWORD=migratorsecret "$CONTAINER" \
    psql -X --single-transaction -v ON_ERROR_STOP=1 -U ple_migrator -d ple_plpgsql_check \
    -f /workspace/schemas/base_schema/install.sql
podman exec "$CONTAINER" psql -X -v ON_ERROR_STOP=1 -U postgres -d ple_plpgsql_check \
    -c 'CREATE EXTENSION plpgsql_check'
```

The ordinary-function query used for the result file was:

```sql
SELECT p.oid::regprocedure AS function_name, checked.*
  FROM pg_catalog.pg_proc AS p
  JOIN pg_catalog.pg_namespace AS n ON n.oid = p.pronamespace
 CROSS JOIN LATERAL public.plpgsql_check_function_tb(
     p.oid::regprocedure,
     fatal_errors => false,
     other_warnings => true,
     performance_warnings => false,
     extra_warnings => true
 ) AS checked
 WHERE p.prolang = (
     SELECT language.oid
       FROM pg_catalog.pg_language AS language
      WHERE language.lanname = 'plpgsql'
 )
   AND n.nspname LIKE 'ple_%'
 ORDER BY function_name::text, checked.lineno NULLS FIRST;
```

Trigger functions were queried with the same call and the installed relation
as its second positional argument:

```sql
SELECT p.oid::regprocedure AS function_name,
       trigger_relation::text,
       checked.*
  FROM pg_catalog.pg_proc AS p
  JOIN pg_catalog.pg_namespace AS n ON n.oid = p.pronamespace
  JOIN pg_catalog.pg_trigger AS trigger_row ON trigger_row.tgfoid = p.oid
 CROSS JOIN LATERAL public.plpgsql_check_function_tb(
     p.oid::regprocedure,
     trigger_row.tgrelid::pg_catalog.regclass,
     fatal_errors => false,
     other_warnings => true,
     performance_warnings => false,
     extra_warnings => true
 ) AS checked
 CROSS JOIN LATERAL (VALUES (trigger_row.tgrelid::pg_catalog.regclass)) AS relation(trigger_relation)
 WHERE NOT trigger_row.tgisinternal
   AND p.prolang = (
       SELECT language.oid
         FROM pg_catalog.pg_language AS language
        WHERE language.lanname = 'plpgsql'
   )
   AND n.nspname LIKE 'ple_%'
 ORDER BY function_name::text, trigger_relation::text, checked.lineno NULLS FIRST;
```

The pre-cleanup result was retained as this historical summary receipt. It is
not a receipt for the final source:

```text
ordinary|functions=272|errors=0|warnings=0
triggers|errors=2|sqlstates=42703,42703
ordinary_rerun_date|2026-09-21
```

Receipt digest for those exact three lines, including their final newlines:
`ca621a66af6215cc3fbaf9a0066904f9676e84e263af8171cb22ae5baca031a6`.

## Findings

The ordinary function pass covered 272 PLE PL/pgSQL functions:

| SQLSTATE | Findings | Classification |
| --- | ---: | --- |
| `42804` | 62 | Real type/return-shape errors. Thirty are `RETURN QUERY` mismatches: 24 opaque-ID domain values, 5 enum values, and one `integer` returned where the API declares `bigint`. The other 32 write or compare `text` values against enum/domain columns. |
| `42883` | 24 | Real PostgreSQL operator or function-resolution errors. Sixteen are enum/text operator mismatches; eight call functions with the wrong or insufficiently typed arguments. |
| `42702` | 8 | Real ambiguous column references, including four in `ple_api.unrelease_assessment`. |
| `42703` | 6 | Real references to fields absent from the installed catalog, including stale `course_theme_id`, `blueprint_edit_number`, and `question_attempt_id` references. |
| `55000` | 1 | A control-flow false positive for `v_post`: the function checks `FOUND` and raises before reading the record when `SELECT INTO` finds no row, but the checker does not connect those facts. |

The largest repeated clusters are:

- domain/enum values returned from `RETURN QUERY` where the declared API
  result is `text`;
- JSON or `text` values inserted into enum columns such as `media_type`,
  `assessment_type`, `scoring_rule`, `selected_question_order`, and cleanup
  states;
- helper calls where a `text` or `unknown` expression is passed to an enum
  parameter, notably `ensure_assessment_policy_snapshot` and the Library
  discussion helpers;
- stale or unqualified names in Blueprint, Assessment Attempt, Course Banner,
  and Unrelease functions.

The pass also emitted six ordinary warnings and eleven extra warnings. The
ordinary warnings include six `STABLE` functions whose bodies use volatile
expressions, plus one unused variable. The extra warnings are mostly unused
variables or shadowing and are not acceptance blockers.

The trigger pass covered the installed trigger functions separately and found
10 errors:

- eight real enum/domain operator mismatches in Course Banner, Question Image,
  Question Availability, and Authoring Workspace trigger bodies;
- two context false positives for `OLD`/`NEW.course_instance_id` in
  `assert_assigned_instructor_membership`. The function contains a
  `TG_TABLE_NAME = 'course_instance'` branch, but the installed trigger is only
  attached to `course_membership_event`, whose row type has no such field.

## Comparison with the acceptance failures

`plpgsql_check` is a strong match for the semantic failures in function bodies:

- enum/domain casts and enum/text operators;
- wrong PL/pgSQL helper signatures and overload resolution;
- `RETURN QUERY` result-shape and result-type mismatches;
- missing catalog fields and ambiguous references.

The current receipt does not prove that already-fixed historical lines would
have produced the identical finding because those lines are no longer in the
installed schema. It does show the same failure families in current functions,
including the same missing-cast and function-resolution patterns.

It would not replace live PostgreSQL behavioral acceptance for failures caused
by values, state, or authority. In particular, it does not catch:

- timestamp ordering and transaction-time fixture mistakes;
- incorrect object addresses, checksums, or other fixture data rejected by a
  trigger;
- trigger-minted IDs and callers that ignore the returned ID;
- role membership, RLS, `SECURITY DEFINER` execution context, or capability
  boundaries as exercised by restricted logins;
- Rust/SQLx bind types and SQL issued by application code rather than a
  PL/pgSQL body;
- dynamic SQL bodies. Four authorization functions use dynamic SQL; the
  checker reports the static body but cannot semantically validate the runtime
  query string in the general case.

The result is strong evidence for an optional fast PostgreSQL semantic
preflight. It is not yet evidence for making the extension part of the normal
server image or `all_test.sh`: the extension packaging must be made
reproducible for the exact PostgreSQL image, and the real findings need a
separate repair pass before a clean signal can be established.

## Current-source cleanup rerun: 2026-09-21

After the initial receipt, the schema repair work addressed the repeated
domain/enum casts, helper argument types, return shapes, stale catalog names,
and trigger comparisons. A fresh disposable database was then rebuilt from
the current `base_schema` using the procedure above.

- Ordinary PLE PL/pgSQL functions: 272 checked, 0 errors and 0 warnings.
- Installed trigger functions: 105 checked, 0 errors and 0 warnings. The
  unreachable `course_instance` trigger branch that produced the previous two
  `42703` findings has been removed from the final source.

The final summary receipt is:

```text
ordinary|functions=272|errors=0|warnings=0
triggers|functions=105|errors=0|warnings=0
ordinary_rerun_date|2026-09-21
```

Receipt digest for those exact three lines, including their final newlines:
`c4c44ed56ddbd21521410bb93e22115557666460d731f08d0a8b5248b8e1fef1`.

This cleanup rerun is diagnostic evidence only. Live acceptance remains the
authority for transaction state, roles, RLS, trigger-minted identities, fixture
values, application bind types, and dynamic behavior.
