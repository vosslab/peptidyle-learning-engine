# M11 Canonical Selector Registration

Status: the accepted Course-publication test is registered in the canonical fresh-database
baseline. Connected PostgreSQL execution remains pending the root-owned acceptance run.

## Target and purpose

The exact ignored test target is:

```text
blueprint_course_postgres_course_publication_pool_reference::course_publication_preserves_an_ordinary_pool_reference
```

The test exercises actual Course-to-Blueprint publication while preserving the ordinary Pool
reference. It verifies the published Blueprint retains the Pool ID and requested selection count,
the source Pool owner remains unchanged, and publication creates no source-linked Pool child. The
Assessment stays unreleased so release-time adequacy remains outside this M11 identity check. Full
test behavior is described in the [Course-to-Blueprint acceptance report](M11_COURSE_BLUEPRINT_ACCEPTANCE.md).

## Canonical registration

`local_stack_control/database_baseline_owner.py` now runs this selector after the existing Manual
WebWork Type and Sysadmin selectors. The three Watch selectors retain their prior order, and earlier
M03 registrations are unchanged. The new result key is
`Course publication Pool reference PostgreSQL acceptance`.

The stanza reuses the existing serial ignored `blueprint_course_postgres` command shape, including
the current application-role environment, `--exact`, and `--test-threads=1`:

```sh
source ./source_me.sh && cargo test -p learning-data-access --features postgres --test blueprint_course_postgres blueprint_course_postgres_course_publication_pool_reference::course_publication_preserves_an_ordinary_pool_reference -- --ignored --exact --test-threads=1
```

## Review and focused checks

- Fresh Luna SPEC review: pass; confirmed the ignored target and insertion point.
- Distinct fresh QUALITY review: accepted with no findings; confirmed target spelling, serial flags,
  result-key convention, selector ordering, and import-only Rust formatting.
- `source ./source_me.sh && python3 -m compileall -q local_stack_control/database_baseline_owner.py` - pass.
- `source ./source_me.sh && pyflakes local_stack_control/database_baseline_owner.py` - pass.
- `source ./source_me.sh && python3 -m pytest -q tests/test_database_baseline_owner.py tests/test_live_demo_browser_disposable_owner.py::test_database_baseline_profile_allows_only_its_postgres_oracle_commands` - pass, 10 tests.
- `source ./source_me.sh && rustfmt --edition 2024 --config skip_children=true --check crates/learning-data-access/tests/blueprint_course_postgres.rs` - pass after the required import-order cleanup.
- `source ./source_me.sh && rustfmt --edition 2024 --check crates/learning-data-access/tests/blueprint_course_postgres/course_publication_pool_reference.rs` - pass.
- `git diff --check -- local_stack_control/database_baseline_owner.py crates/learning-data-access/tests/blueprint_course_postgres.rs` - pass; both report files also passed the focused trailing-whitespace scan.

No database connection or stack launch occurred in this registration task. The prior direct selector
attempt is recorded in the [acceptance report](M11_COURSE_BLUEPRINT_ACCEPTANCE.md): the target
compiled, then stopped while loading the unavailable disposable-runtime manifest. Connected runtime
acceptance remains pending the root-owned fresh-database gate.
