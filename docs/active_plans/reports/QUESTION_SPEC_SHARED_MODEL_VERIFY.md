# Shared `question_model` verification

## Scope

This bounded verification followed coherent handoffs of the M09 model/domain removed-position
fixtures and the M27 Blueprint Theme model fixtures. It ran the shared model library tests once and
made no source changes. Only those model-fixture handoffs were considered ready: M06 Type/metadata
and M09 LDA/API work remains in progress. The root 29-milestone Question-spec plan remains ongoing.

## Command and result

```bash
source source_me.sh && cargo test -p question_model --lib
```

Cargo compiled `question_model` and executed 165 tests: 164 passed, 1 failed, 0 ignored, and 0
filtered out. The command exited with status 101.

The six search tests passed: the Question Library search normalization test and all five
`question_search::tests`. The
`answer::tests::text_match_modes_use_camel_case_names` serialization test also passed.

## Failure

`student_work::identifiers::tests::issued_question_identity_is_stable_and_distinguishes_frozen_content`
failed at [identifiers.rs](../../../crates/question_model/src/student_work/identifiers.rs#L150).
The test's supposed-valid Question ID was rejected with: `Published Question ID random characters
must be exact uppercase Crockford Base32`.

The model tests compiled, so the earlier M27 Theme fixture blocker is cleared. Source fixes for the
remaining failure belong to their assigned owners.

## Limits

This is one `question_model` library test run. No database, generation, full-stack, or live acceptance
was performed. The package-wide gate is not green, and this result does not complete a Question-spec
milestone or the root plan.
