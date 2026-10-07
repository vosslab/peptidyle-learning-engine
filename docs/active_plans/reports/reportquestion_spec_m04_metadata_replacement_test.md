# M04 Draft metadata replacement persistence test

The baseline owner selects
`webwork_draft_creation_keeps_the_initial_source_binding_on_confirmation` exactly in
[local_stack_control/database_baseline_owner.py](../../../local_stack_control/database_baseline_owner.py#L294-L305).
That selector invokes the ignored connected PostgreSQL test in
[crates/learning-data-access/tests/authoring_draft_source_postgres.rs](../../../crates/learning-data-access/tests/authoring_draft_source_postgres.rs#L87).
Within that test, the [metadata replacement assertions](../../../crates/learning-data-access/tests/authoring_draft_source_postgres.rs#L307-L388)
replace all six fields (title, description, Tags, license, citation, and language), reload and
compare the full metadata value, confirm omitted Hint and Worked Solution remain intact, and compare
the complete source-binding tuple before and after replacement, including source object ID and
checksum.

The current connected run recorded `1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`
for this exact selector in `output_question_spec/fresh_database_gate/checkpoint_diagnostics_20261006.log`.
The earlier frozen-checkpoint pass is also preserved in
`output_question_spec/fresh_database_gate/checkpoint_20261006.log`.
The current run therefore supports this narrow M04 database-test result. The full fresh-database
gate, M04 production-stack browser journeys, and M03 dependency acceptance remain pending.

Earlier source and compile checks:

- `rustfmt --edition 2024 --check crates/learning-data-access/tests/authoring_draft_source_postgres.rs` - passed.
- `git diff --check -- crates/learning-data-access/tests/authoring_draft_source_postgres.rs docs/active_plans/reports/reportquestion_spec_m04_metadata_replacement_test.md` - passed.
- `source ./source_me.sh && cargo test -p learning-data-access --test authoring_draft_source_postgres --no-run` - historical report state: pending host-compiler coordination. The connected baseline run above subsequently compiled and executed the exact ignored selector.

The test remains ignored outside the disposable acceptance runtime. This report records one
connected test result and does not claim complete M04 or M03 acceptance.
