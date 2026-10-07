# Question-spec focused compile preflight

Status: Two current-source PostgreSQL test targets compile. The grading lifecycle target does
not compile because the new regression uses `Uuid::new_v4()` while the active UUID crate features
do not expose it. No connected PostgreSQL tests ran; compile success is not runtime acceptance.

## Commands and results

Commands were run through the repository environment with the PostgreSQL feature enabled:

- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test authoring_draft_source_postgres --no-run` - passed; emitted `tests/authoring_draft_source_postgres.rs`.
- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test blueprint_course_postgres --no-run` - passed; emitted `tests/blueprint_course_postgres.rs`.
- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test grading_lifecycle_postgres --no-run` - failed with 11 `E0599` errors in `crates/learning-data-access/tests/grading_lifecycle_postgres/partial_credit_policy.rs`. Each reports that `Uuid::new_v4()` is unavailable for `Uuid` under the current feature set. The calls are at lines 23, 24, 26, 27, 97, and 107 (11 calls total). The compiler notes `new_v5` as a similarly named available function.

## Interpretation

The authoring draft source target covers the M04 metadata replacement case, and the blueprint target
covers the M03/M08 PostgreSQL tests. Both compile with current source. The grading lifecycle target
contains the M17 highest-Attempt policy regression and is blocked at compilation by UUID generation.
A focused correction is needed before the M17 test can be compiled. All database-connected execution
remains pending the coordinated fresh-database gate.
