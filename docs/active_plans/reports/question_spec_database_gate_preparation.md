# Question-spec fresh database gate preparation

## Prepared scope

The canonical database baseline retains the existing M04 authoring selector
`authoring_draft_source_postgres::webwork_draft_creation_keeps_the_initial_source_binding_on_confirmation`.
Its connected test now also verifies Draft metadata replacement persistence, retained support text,
and an unchanged source-binding tuple. The selector already used the baseline's PostgreSQL feature,
ignored-test, exact-match, serial execution, and owner-provided database environment.

Added the focused M17 selector
`grading_lifecycle_postgres_partial_credit_policy::partial_credit_toggle_reorders_highest_submitted_attempt_from_retained_fractions`
with that same connected-test configuration. It runs after the broad grading finalization selector
and before the remaining destructive or revision-mutating cases.

The existing M08 Blueprint lineage selector remains registered. The Question Revision metadata
selector remains the final connected Rust selector because it advances shared fixture
`BPFX-Y001` to Revision 2. The relevant order is therefore M04 authoring metadata, M17 highest
Attempt selection, M08 lineage, then M03 Question Revision metadata.

## Gate status

This report records selector readiness only. No stack, fresh database, or connected selector was
started. The manager owns the later exclusive run after source coordination, compile-only
preflights, and reviews establish a coherent snapshot.

The preparing commands are:

```sh
source ./source_me.sh && cargo test --manifest-path Cargo.toml -p learning-data-access --features postgres --test authoring_draft_source_postgres --no-run
source ./source_me.sh && cargo test --manifest-path Cargo.toml -p learning-data-access --features postgres --test grading_lifecycle_postgres --no-run
source ./source_me.sh && cargo test --manifest-path Cargo.toml -p learning-data-access --features postgres --test blueprint_course_postgres --no-run
```

The selector change was inspected against the exact Rust test function names and the existing
owner-provided `DATABASE_URL`/acceptance-runtime environment path. Compile-only command results and
the later fresh-database outcomes are pending the manager's gate.
