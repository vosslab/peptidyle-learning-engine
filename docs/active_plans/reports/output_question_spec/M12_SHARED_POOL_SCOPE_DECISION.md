# M12 shared-Pool scope decision

## Decision

An ordinary Question Pool is a reusable shared object. `save_question_pool_members` replaces the
member set for one Pool ID, so a successful Save changes future selections in every Assessment that
references that Pool. Existing Attempts retain the Questions already selected for them, including
their exact Question Revision Tuples and recorded Pool Edit Number. Adding a Pool to an Assessment
does not create a fork; an independent Pool remains an explicit fork.

There is no Assessment-local member-removal action. When an ordinary Pool Save removes an existing
Question ID, keep the current post-issue guard and apply its issue check to every currently
available Assessment Pool Entry referencing that Pool. Reject the removal if that Question was
issued to any Student in any affected Assessment. An issue in an Assessment that does not reference
this Pool does not affect the Save, and a retired Pool Entry is outside the available-entry check.
The guard is the composition of the shared-Pool rule and the Assessment-specific removal limit; it
does not create a new Pool ownership or lifecycle policy.

Keep selection-count sufficiency separate from this removal guard. Each available Assessment Pool
Entry is checked against its own requested count at Release Validation. Human Guidance allows Pool
changes to be saved when an unreleased Assessment requests more Questions than the Pool can provide.
This decision adds no selection-count rejection to ordinary Pool Save.

Explicit Pool forks remain ordinary independent Pools. Whole-Pool removal from an issued Assessment
continues to exclude that entry's earned and possible points from every Attempt under the existing
fairness rule.

## Evidence and limits

- [HUMAN_GUIDANCE.md](../../../HUMAN_GUIDANCE.md#L1316) says the Assessment owns its selection count,
  many Assessments may reference one Pool, Pool changes affect future selections wherever
  referenced, Attempts retain selected Questions, and forks are explicit.
- [HUMAN_GUIDANCE.md](../../../HUMAN_GUIDANCE.md#L1709) allows saving a Pool with too few Questions
  for an unreleased Assessment, requires Release Validation, and limits post-issue individual
  removal to Questions not issued in that Assessment. It retains the whole-Pool point-exclusion
  fairness rule.
- [question_pools.sql](../../../../schemas/base_schema/50_functions/question_pools.sql#L393) checks a
  removed Question against each available referencing Assessment Entry and tests issuance using
  that Entry's Assessment ID. The same function replaces the member rows for the Pool ID at
  [question_pools.sql](../../../../schemas/base_schema/50_functions/question_pools.sql#L478).
- [assessment_release_validation.sql](../../../../schemas/base_schema/50_functions/assessment_release_validation.sql#L96)
  calculates valid-member sufficiency independently for each Assessment Entry.
- The retired one-time oracle source `tests/e2e/assessment_saved_response/10_m12_pool_lifecycle.sql`
  asserted removal of an unissued member and rejection when the Question was issued in the same
  available Assessment (line 279). Its second-Assessment case asserted that an issue in another
  available Assessment referencing the shared Pool also prevents removal (line 409). Those
  historical assertions are no longer durable test evidence. Existing Attempt tuple retention
  remains asserted by the current fairness oracle at
  [07_assessment_fairness.sql](../../../../tests/e2e/assessment_saved_response/07_assessment_fairness.sql#L287).

This is a source-and-guidance scope decision. The retired one-time oracle source described above
was inspected while present, not run here; the current fairness oracle was inspected but not run.
The decision makes no connected database, full-build, or runtime acceptance claim. Root retains
canonical runtime execution and overall M12 acceptance.

The cross-reference example is queued for a wording check at tomorrow's review. That editorial
check does not gate the current M12 plan or reopen the settled enforcement rule; see the narrow
[question_spec_implementation_uncertainties.md](../../decisions/question_spec_implementation_uncertainties.md#m12-shared-pool-scenario-cross-reference-wording).
