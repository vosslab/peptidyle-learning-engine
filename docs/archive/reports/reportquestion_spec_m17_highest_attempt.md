# M17 highest-Attempt PostgreSQL regression

## Evidence added

`crates/learning-data-access/tests/grading_lifecycle_postgres/partial_credit_policy.rs`
adds an ignored connected-PostgreSQL regression using one Assessment, two
submitted Attempts, and two equal two-point Questions in each Attempt. It stores
synthetic immutable fractions through the existing finalization API: Attempt A
has `0.75, 0.75`; Attempt B has `1.0, 0.0`.

The test toggles the current Assessment policy `true -> false -> true`. It checks
each Attempt's current submitted score through the existing finalization reader,
checks highest-Attempt selection through
`ple_private.read_assessment_gradebook_evidence`, and verifies the four stored
fractions plus each Attempt's entry and policy snapshot IDs remain unchanged.
The finalization reader must report `already_submitted` with no backend work,
showing that policy changes recalculate current points without another Backend
grading operation.

## Verification

The connected test is registered in
`crates/learning-data-access/tests/grading_lifecycle_postgres.rs` and requires
the disposable PostgreSQL acceptance runtime.

- Compile-only command: `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test grading_lifecycle_postgres --no-run`
- Connected command: `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test grading_lifecycle_postgres partial_credit_policy::partial_credit_toggle_reorders_highest_submitted_attempt_from_retained_fractions -- --ignored --exact`
- Repository quality routes after implementation lanes are coherent: `source ./source_me.sh && ./launchers/run_fast_checks.sh`; full compliance: `source ./source_me.sh && ./launchers/all_test.sh`.

No compile-only or connected PostgreSQL command was run for this handoff. The
test therefore remains pending runtime acceptance on a coherent fresh database.
The test uses synthetic graded records to isolate the policy boundary and does
not claim any Native Multiple Choice Backend fraction behavior. Update the main
M17 report and implementation ledger after the compile and connected gates
complete; fresh specification and quality review remain required.
