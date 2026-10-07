# Question-spec baseline selector correction

The canonical PostgreSQL baseline now runs the ignored `grading_lifecycle_postgres`
integration tests once. That broad selector includes the M17 partial-credit toggle
regression, so a second exact M17 invocation would repeat the same fixed fixture in
the same database. Removed that duplicate invocation while retaining the shared
summary logger and the existing M04, M08, and final M03 selector order.

## Verification

- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test grading_lifecycle_postgres -- --list` passed without connecting to PostgreSQL. The compiled test target lists five tests, including
  `partial_credit_policy::partial_credit_toggle_reorders_highest_submitted_attempt_from_retained_fractions` once.
- Source inspection confirms that test is ignored for the disposable PostgreSQL runtime and remains covered by the broad `--ignored --test-threads=1` selector.
- `source ./source_me.sh && python3 -m py_compile local_stack_control/database_baseline_owner.py` passed.

This is selector registration evidence only. The canonical fresh-database runtime has
not been rerun by this correction.
