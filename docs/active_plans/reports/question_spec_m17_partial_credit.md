# M17 Partial Credit Setting

## Implementation

The existing `AssessmentActivityRules` aggregate now contains
`partial_credit_enabled` / `partialCreditEnabled`, defaulting to `true`. The
Assessment policy snapshot stores the flag, includes it in its content hash,
and carries it through policy replacement, Blueprint publication/adoption,
Assessment Template read/save/copy, and workspace read/save.

Stored Backend fractions remain unchanged. Current score readers use the flag
from the current Assessment policy snapshot, including Gradebook Attempt
selection, submitted Attempt rescore/history, and finalization results. Attempt
policy snapshots remain pinned for their existing disclosure and timing
semantics. With the flag off, Normal and Extra Credit entries award points only
for a stored fraction of exactly one. Full Credit still awards all points for
any non-NULL response, while unanswered responses and Excluded entries earn
zero.

The Assessment workspace, Assessment Template editor, and Blueprint reusable
defaults editor expose the setting. Blueprint update review also displays its
current/proposed value. Shared published-question fixtures and authorized TS
harnesses carry the new required field.

## Focused verification

- `source ./source_me.sh && cargo test -p question_model activity_rules --lib` - 10 passed.
- `source ./source_me.sh && node --import tsx --test tests/test_assessment_template_settings_model.mjs tests/test_blueprint_course_model.mjs tests/test_blueprint_course_client.mjs tests/test_assessment_template_client.mjs tests/test_assessment_summary_policy_decoder.mjs tests/test_live_assignment_release_validation.mjs tests/test_assignment_workspace_questions.mjs tests/test_assignment_client.mjs tests/test_base_assignment_policy_autosave_model.mjs tests/test_nested_identity_contracts.mjs` - 76 passed.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json` - passed.
- `source ./source_me.sh && schema_style/check_schema_style.py` - clean.
- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test grading_lifecycle_postgres --no-run` - compiled successfully.
- Scoped Prettier check passed after formatting the three changed UI/test files.

The connected PostgreSQL test was not run. The focused rescore integration case
now stores a 0.5 Backend fraction and verifies toggling partial credit
true->false->true changes its points 2.5->0->2.5 without changing that fraction.
It also checks the 0/1 cases. The separate same-Assessment, two-Attempt test
must verify the current-setting highest-Attempt reorder before M17 acceptance.
Fresh-database execution and the dependent M03 acceptance gate remain pending.

## Limits

No full-stack rebuild, schema generation, connected database execution, or
browser capture was performed for this handoff. The model contract was
generated through the coordinated M04/M17 TypeScript generation pass. Fresh
specification and quality review remain required; this report records focused
implementation evidence only.
