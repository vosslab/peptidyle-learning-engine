# M17 Blueprint fixture assertion correction

The approved [Question-spec implementation plan](../active/question_spec_implementation_plan.md#m17-apply-partial-credit-settings)
requires copied Blueprint Assessment settings to retain their intended value. The focused
`blueprint_course_input_is_one_nested_question_revision_tree` test already sets
`partial_credit_enabled` to `false`; its JSON assertion used a stale `content` wrapper.

## Verified shape

- [blueprint_children.rs](../../../crates/question_model/src/blueprint_course/blueprint_children.rs#L106-L110)
  declares `CreateBlueprintModuleInput.assessments` as
  `Vec<BlueprintAssessmentContentInput>`.
- [assessment_content.rs](../../../crates/question_model/src/blueprint_course/assessment_content.rs#L167-L181)
  declares `defaults` directly on `BlueprintAssessmentContentInput`.

The assertion now reads
`modules[0].assessments[0].defaults.activity_rules.partialCreditEnabled`. The expected value remains
explicitly `false`. This changes no product or wire shape, and leaves current Assessment and copied
Blueprint settings unchanged.

## Verification

- The focused `question_model` test passed: 1 test, 164 filtered out.

  ```bash
  source ./source_me.sh && cargo test -p question_model blueprint_course_input_is_one_nested_question_revision_tree
  ```
- `source ./source_me.sh && cargo fmt --check -p question_model` - passed.
- `git diff --check -- crates/question_model/src/blueprint_course.rs` and a trailing-whitespace scan
  of this report - passed. Scoped diff review confirmed the existing source edits were preserved.

## Limit

This is focused model-test evidence only. It does not establish M17's connected PostgreSQL or
integrated acceptance gates; independent reviews remain separate.
