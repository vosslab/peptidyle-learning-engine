# M17 current Assessment policy in gradebook scoring

`ple_private.read_assessment_gradebook_evidence` reads
`partial_credit_enabled` from the policy snapshot referenced by the ordinary
`ple_data.assessment` row. This makes a current Assessment policy edit
recalculate submitted Attempt points and highest-Attempt selection from the
retained grading fractions. Each Attempt's pinned policy snapshot remains
historical evidence and is not used to decide the current award setting.

The existing registered PostgreSQL regression
`partial_credit_policy::partial_credit_toggle_reorders_highest_submitted_attempt_from_retained_fractions`
checks policy changes A -> B -> A, verifies the selected highest submitted
Attempt changes accordingly, and confirms retained outcomes remain unchanged.
It is owned by the grading lifecycle test lane and was not run in this
correction pass; connected acceptance remains pending.

## Checks

- Source inspection confirmed the policy join follows
  `assessment.assessment_policy_snapshot_id`; it does not follow
  `assessment_attempt.assessment_policy_snapshot_id`.
- Source-only schema-style check passed through `source ./source_me.sh &&
  python3 -c ...`: zero findings. The checker API loaded SQL source directly,
  bypassing the stale generated catalog snapshot.
- No PostgreSQL, compile, or full quality checks were run.
