# Question-spec fresh database gate

## Checkpoint: October 6, 2026

The first canonical attempt installed and replayed the schema, then failed in the M26 fixture
because it inserted `course_roster_profile` as `ple_private_owner`. That table has forced row-level
security and its insert policy applies to `ple_api_owner`
(`schemas/base_schema/60_policies/course_roster.sql:13-21`). The fixture role was corrected, and
the source-only schema style gate is now clean.

The second canonical attempt installed and replayed the corrected schema, verified the application
login, and passed six connected PostgreSQL tests before failing later in the M26 security catalog.
The confirmed `read_sysadmin_student_roster_record` projection failed inside its `RETURN QUERY`
around the membership and invitation status checks. The controller's old bounded diagnostic
retained only the last 2,048 characters, dropping PostgreSQL's initial `ERROR` and SQLSTATE. The
stack was cleaned after failure. The diagnostic now preserves redacted `ERROR`, `FATAL`, `PANIC`,
and related context before clipping; a focused test protects that behavior.

The current source suggests a likely type mismatch: the projection declares its first two output
columns as `text`, while `course_roster_profile.course_instance_id` and `student_account_id` use the
`ple_data.course_instance_id` and `ple_data.account_id` domains. The projection should cast those
columns to text. This remains an inference until the diagnostic is captured or a fresh run confirms
the casts.

The successful live selectors and counts from the second attempt were:

| Connected behavior | Passed | Filtered |
| --- | ---: | ---: |
| Draft Question source binding on confirmation | 1 | 0 |
| Student Account time-zone default | 1 | 0 |
| Assessment access projection | 1 | 0 |
| Empty Course lifecycle | 1 | 3 |
| Course active-lifetime validation | 1 | 3 |
| Course summary lifecycle state | 1 | 3 |

The M26 catalog failure occurred before attempt-expiry, grading lifecycle, Unrelease, M08 lineage,
and M03 Revision metadata selectors. The current reader-function source correction therefore has
not been exercised on a live database. The canonical baseline now places the M08 lineage selector
immediately before the M03 metadata selector; M03 remains last because it advances fixture
`BPFX-Y001` to Revision 2.

## Preflight evidence

| Check | Result |
| --- | --- |
| `source source_me.sh && cargo check -p server_core -p learning-data-access` | Pass |
| `source source_me.sh && python3 devel/generate_schema_tables_doc.py` | Pass; regenerated `docs/SCHEMA_TABLES.md` |
| `source source_me.sh && schema_style/check_schema_style.py` | Pass, clean after M08 NULL-parent comments |
| `source source_me.sh && cargo check -p server_core -p learning-data-access` | Pass after the M26 and M03 archive-affordance corrections |
| `source source_me.sh && bash tests/e2e/e2e_database_baseline.sh` | Two fresh schema installs passed; both runs stopped in the M26 security catalog, first at fixture RLS setup and second in the confirmed reader query |
| `source source_me.sh && python3 -c 'import local_stack_control.disposable_stack_adapter; import pytest; raise SystemExit(pytest.main(["-q", "tests/test_local_stack_lifecycle.py", "-k", "postgres_error_cause"]))'` | Pass: 1 passed, 28 deselected |

The second-run log is `output_question_spec/fresh_database_gate/e2e_database_baseline_after_corrections.log`.
The focused M08 lineage and M03 Revision tests, corrected M26 catalog, and full unmodified fresh
schema proof remain pending.

## Current-source third attempt

The third canonical run used the current production SQL and API source with no temporary function
overlay. The fresh schema installed and replayed, and the application login check passed. The M26
security catalog passed, including the confirmed Sysadmin roster projection and catalog assertions.
The direct Assessment Attempt finalization selector also passed (4 passed, 0 failed). The six
connected selectors listed above passed again. The run then stopped before later acceptance cases
because the `blueprint_course_postgres` integration target did not compile. The controller removed
the disposable stack; `podman ps` was empty after failure.

The exact compile-only command was:

```sh
source source_me.sh && cargo test --manifest-path Cargo.toml -p learning-data-access --features postgres --test blueprint_course_postgres --no-run
```

It exited 101 and reported three test-source errors: `lineage_fork.rs` imported two unavailable
root-level input types; `lineage_fork.rs` moved `application_pool` into the store before closing it;
and `question_revision_metadata.rs` moved `question_id` before a later borrow. No database was
used by this compile-only command. The dedicated test-compile owner corrected the imports and
ownership moves without changing assertions; the exact compile-only command now passes. The
current-source third-run log is
`output_question_spec/fresh_database_gate/e2e_database_baseline_current_source.log`.

Current evidence includes successful M26 catalog execution, six earlier connected selectors, and
the four-case grading finalization selector. M08 lineage and M03 Revision metadata behavior have
not run yet. A second attempt was started after the corrected test target's `--no-run` preflight
passed and both fresh reviews passed. The root then reported concurrent M04 Rust/model edits, so
this attempt no longer matched a coherent frozen source snapshot. It was canceled with Ctrl-C
during the in-VM `cargo build --release -p project-tools`, before schema installation or any
connected selector. The log is empty because controller output was buffered at interruption. The
controller cleanup was verified: no owned containers, volumes, networks, build/controller process,
or workspace contents remained. This is canceled/inconclusive, not a product failure. The canceled
attempt log is `output_question_spec/fresh_database_gate/e2e_database_baseline_final_attempt.log`.

Current-source M08 lineage and M03 Revision metadata behavior remain unverified. Rerun the
canonical baseline only after the coordinated source is coherent and the matching
`blueprint_course_postgres --no-run` preflight passes; the registered order runs M08 immediately
before M03, with M03 last.

## Frozen checkpoint run: October 6, 2026

The canonical command was run against the coordinated frozen source after the three PostgreSQL
test targets compiled and the independent M04/M17 reviews passed:

```sh
source ./source_me.sh && bash tests/e2e/e2e_database_baseline.sh
```

The fresh schema and application login completed. The M04 authoring Draft source-binding selector,
Student Account time-zone selector, Student Assessment Access selector, and the four registered
Course lifecycle selectors passed. The broad direct Assessment Attempt finalization selector then
failed, so the separate M17 highest-Attempt case, Unrelease, M08 lineage, and final M03 Revision
metadata case did not run. The exact redacted owner output is preserved in
`output_question_spec/fresh_database_gate/checkpoint_20261006.log`.

The owner reported only `test failed, to rerun pass -p learning-data-access --test
grading_lifecycle_[private]`; it did not retain the failing test name, assertion, SQLSTATE, or
captured cargo stdout/stderr. Although the error message said stack resources were retained for
diagnostics, the profile's final reset had already removed the disposable project and cleared its
runtime workspace. Read-only owner discovery confirmed `ple-live-demo-browser` had zero containers,
volumes, and networks. The manifest and cleanup capability were gone, so the retained-target
diagnostic action could not be used and there is no further cleanup to perform. This failure is
inconclusive about the product behavior until the grading failure is isolated with exact output.

The database gate remains incomplete. M04 connected behavior passed in this run; M17 runtime,
M08 lineage, M03 Revision metadata, Unrelease, and all later selectors remain unverified.

## Frozen diagnostic rerun: October 6, 2026

The canonical command was rerun with the updated redacted Rust-panic extractor and the corrected
baseline selector:

```sh
source ./source_me.sh && bash tests/e2e/e2e_database_baseline.sh
```

Fresh schema setup, application verification, the security catalog, and Assessment Attempt expiry
completed. The six connected selectors above passed again. The broad direct Assessment Attempt
finalization selector then stopped in the M17 test
`partial_credit_toggle_reorders_highest_submitted_attempt_from_retained_fractions` during fixture
setup. The run log shows a bare UUID assertion in the fixture's
[started Attempt ID check](../../../crates/learning-data-access/tests/grading_lifecycle_postgres/partial_credit_policy.rs#L155-L156): it expected Attempt 2
(`f5710000-0000-0000-0000-000000000002`), but `start_assessment_attempt` resumed the still-open
Attempt 1 (`f5710000-0000-0000-0000-000000000001`). This is the `started_attempt_id` check in the
fixture setup, before the test's partial-credit policy loop and its three-value
[highest-Attempt assertion](../../../crates/learning-data-access/tests/grading_lifecycle_postgres/partial_credit_policy.rs#L457-L461);
it does not show a grading SQL failure. The exact
redacted run log is `output_question_spec/fresh_database_gate/checkpoint_diagnostics_20261006.log`.
The grading selector registers five ignored tests, including this M17 case. The bounded panic
diagnostic does not retain Cargo's per-test summary, so outcomes for the other four are unknown and
are not reported as passes.

After this run, the test owner corrected the fixture to submit Attempt 1 before requesting Attempt
2. The current test source records that sequential lifecycle at
[`crates/learning-data-access/tests/grading_lifecycle_postgres/partial_credit_policy.rs`](../../../crates/learning-data-access/tests/grading_lifecycle_postgres/partial_credit_policy.rs#L195-L200).
The owner reports compile, rustfmt, and diff checks passed for the correction; no connected runtime
rerun has exercised it yet, so M17 policy behavior remains pending.

The profile owner completed its final reset after the failure. Post-run read-only inventory found
zero containers, zero volumes, and no project network; only Podman's built-in `podman` network
remained. The final workspace reset is also enforced by the profile owner, which returned only the
original grading failure rather than a cleanup failure. No second stack was started. M08 lineage,
M03 Revision metadata, Unrelease, and the later Question Library selectors were not reached. The
fresh database gate remains incomplete pending runtime verification of the corrected M17 fixture and
a later canonical run. The six earlier connected selectors, including the exact M04 metadata
replacement test, passed in this diagnostic run; the final disposable cleanup found zero resources.
