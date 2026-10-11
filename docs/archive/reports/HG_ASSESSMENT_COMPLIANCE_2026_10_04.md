# Human Guidance Assessment compliance audit - 2026-10-04

## Scope and method

This is a fresh source and focused-test review of the current Assessment
implementation against the current `HUMAN_GUIDANCE.md` requirements below. It
does not inherit the pre-interview checklist's verified marks. It excludes the
deferred optional Question Feedback timing and deferred Native JSON regrade
decision.

Reviewed Human Guidance requirements:

- `HUMAN_GUIDANCE.md:1576-1582` - limited edits after issue, the Pool-member
  guard, whole-entry removal, fairness, and Unrelease.
- `HUMAN_GUIDANCE.md:1666-1681` - immediately visible automated scores and
  separately controlled correct-answer disclosure.
- `HUMAN_GUIDANCE.md:1145-1150` and `1758-1775` - immutable credit fractions,
  current point values, and recalculation without another Backend call.

Focused tests run:

- `cargo test -p domain student_feedback_release`: 16 passed. These prove the
  existing configurable score-release behavior; they do not prove current HG.
- `cargo test -p question_model current_points_recalculate_without_changing_recorded_credit`:
  1 passed.
- `node --import tsx --test tests/test_assignment_workspace_questions.mjs tests/test_student_course_progress_presentation.mjs tests/test_student_course_attempt_history.mjs`:
  13 passed. Several assertions preserve the now-rejected "Score not released"
  behavior, so passing does not establish compliance.
- `tests/e2e/e2e_assessment_saved_response.sh`: passed against a disposable
  PostgreSQL 17 container. It exercised ordinary Student response finalization,
  grading, and Assessment-owned Pool construction. It did not attempt the
  changed post-issue edit or score-withholding behavior, so it is runtime
  evidence for the normal path only.
- One-time A-04 probe: passed against a second disposable PostgreSQL 17
  container. It reused the repository's `assessment_saved_response_oracle.sql`,
  issued and finalized a two-Question Attempt (one fixed Question and one Pool
  Question), then removed the Pool with the normal `save_assessment` path. The
  submitted Attempt remained `2 / 2`, proving the retired Pool still supplied
  its `1 / 1` contribution. This was a one-time probe, not a durable test.
  A replayable diagnostic is retained at
  [`a04_retired_pool_score_probe.sql`](hg_compliance_2026_10_04/a04_retired_pool_score_probe.sql).
  Its fresh disposable replay also passed, with the same `2 / 2` result:
  [`a04_replay_pass.log`](hg_compliance_2026_10_04/a04_replay_pass.log).
  The earlier replay setup failures are retained separately in
  [`a04_replay.log`](hg_compliance_2026_10_04/a04_replay.log); they exposed
  missing diagnostic-fixture setup, not a product behavior change.

No production files were changed.

## Priority findings

### A-01 - score-release controls still hide automated scores

**Status: mismatch - highest priority.**

HG requires each Question score and the total to be visible as soon as an
Attempt is submitted and automatically graded. An Instructor cannot hide or
delay them, there is no separate posting step, and another Student's completion
does not affect them (`HUMAN_GUIDANCE.md:1668-1669`).

The shipped model still stores an independently configurable `feedback_score`
timing. `ple_private.student_assessment_score_is_released` accepts
`during_attempt`, `after_submit`, `after_due`, `after_close`, and a hidden
case (`schemas/base_schema/50_functions/grading_access.sql:14-29`). The Student
landing suppresses both earned and possible points unless that timing releases
them (`schemas/base_schema/50_functions/student_assessment_landing.sql:119-147`),
and Attempt History does the same
(`schemas/base_schema/50_functions/student_course_attempt_history.sql:137-150`).
The UI explicitly renders "Score not released"
(`src/pages/assessment_overview_page.tsx:51-55`; `src/pages/student_course_attempt_history_page.tsx:110-112`).

The focused domain test suite passed because it validates this old timing model,
including date/close release. This is a real, tested divergence, not merely a
stale comment.

Correct-answer timing is a separate HG decision and should remain separate;
this finding does not ask to change it.

### A-02 - post-issue content saves allow changes beyond the limited list

**Status: mismatch - high priority fairness risk.**

Once an Assessment is issued, HG allows only point changes, Question order,
limited unissued Pool-member removal, and whole-Pool removal with the stated
fairness effect (`HUMAN_GUIDANCE.md:1576-1582`). HG treats complete standalone
Question removal as tentative ("I would probably allow"), although it makes
the scoring effect firm if that removal is permitted.

`ple_data.save_assessment` calls `ple_data.replace_assessment_entries` with no
check for an issued Attempt (`schemas/base_schema/50_functions/assessments.sql:451`).
That replacement function can:

- update a fixed entry's Question ID and Revision as well as its point value
  (`schemas/base_schema/50_functions/assessments.sql:151-164`);
- insert a new fixed Question entry (`schemas/base_schema/50_functions/assessments.sql:166-188`);
- change per-entry scoring and Question-level attempt settings
  (`schemas/base_schema/50_functions/assessments.sql:124-150`);
- change Pool selection count and selected order
  (`schemas/base_schema/50_functions/assessments.sql:309-334`); and
- retire omitted entries (`schemas/base_schema/50_functions/assessments.sql:343-346`).

The Questions editor exposes the same unrestricted remove action whenever an
entry is available, with no issued-Student condition
(`src/pages/assessment_workspace/assessment_workspace_questions_page.tsx:241-253`; `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx:333-346`).
This permits future Attempts to differ from already issued Attempts in ways HG
does not allow.

No focused test establishes enforcement of the current post-issue limits.

### A-03 - Pool member replacement has no issued-Question guard

**Status: mismatch - high priority fairness risk.**

HG permits removing an individual Pool Question only when it has not been
issued to any Student in that Assessment (`HUMAN_GUIDANCE.md:1578`).

The endpoint named `append_assessment_question_pool_fork_members` accepts a
complete member list and forwards it to `save_question_pool_members`
(`schemas/base_schema/50_functions/assessment_pool_forks.sql:180-183`). That
underlying function deletes every current member and inserts the submitted list
(`schemas/base_schema/50_functions/question_pools.sql:312-321`). The Assessment
wrapper checks ownership and edit numbers, but does not check Assessment status,
issued Questions, or whether a removed member was delivered
(`schemas/base_schema/50_functions/assessment_pool_forks.sql:147-205`). The
front end calls this replace operation directly
(`src/pages/assessment_workspace/assessment_workspace_questions_page.tsx:577-596`).

Thus an Instructor can replace a released Pool's membership, including removal
of a Question already issued in that Assessment. There is no source or test
evidence of the required per-member issue guard.

### A-04 - removing a Pool stops future delivery but does not remove its score from existing Attempts

**Status: mismatch - highest priority fairness risk.**

HG says a bad Pool may be removed and that both earned and possible points must
be excluded from **every** Attempt, including existing Attempts
(`HUMAN_GUIDANCE.md:1579-1581`).

The current editor removes an entry from its save payload
(`src/pages/assessment_workspace/assessment_workspace_questions_model.ts:119-127`),
and `replace_assessment_entries` marks the omitted entry `retired`
(`schemas/base_schema/50_functions/assessments.sql:343-346`). This correctly
keeps it out of future delivery: the Student landing counts only `available`
entries (`schemas/base_schema/50_functions/student_assessment_landing.sql:64-68`).

Existing Attempt scoring, however, looks up the same retired entry's current
fixed-Question or Pool point value and falls back only when no row exists
(`schemas/base_schema/50_functions/assessment_attempt_finalization.sql:67-78`; the
same pattern is used by Gradebook evidence at
`schemas/base_schema/50_functions/grading_access.sql:91-102`). Retirement alone
does not change the stored scoring rule or point value. `score_recorded_credit`
therefore continues to return the Pool's points possible and, for normal scoring,
earned points (`schemas/base_schema/50_functions/grading.sql:16-32`).

Even an `excluded` scoring rule would not meet the stated Assessment-total rule:
it zeros earned points but still returns the supplied points as `points_possible`
(`schemas/base_schema/50_functions/grading.sql:23-29`). The existing remove path
does not apply either an exclusion or a zero point value. No permanent regression test covers Pool
removal after issue and rescoring all Attempts.

A one-time disposable PostgreSQL probe now confirms this path at runtime. It
used the standard saved-response oracle setup, created a submitted Attempt with
one fixed and one Pool Question (each worth one point), removed only the Pool
through `ple_api.save_assessment`, then called
`ple_api.prepare_student_assessment_attempt_finalization` as the Student. The
result remained `2 / 2` and the probe's assertion passed. The removed Pool
therefore still supplied its `1 / 1` score contribution after retirement.

### A-05 - Quiz and Exam all-completed behavior is a hard gate, not a selectable default

**Status: mismatch - medium priority; affects Quiz/Exam answer disclosure.**

HG makes the all-Students-completed condition the default for Quiz and Exam
correct answers, but says Instructors can change Assessment-type defaults and
explicitly lets an Instructor change the correct-answer setting when a Student
goes AWOL (`HUMAN_GUIDANCE.md:1670`, `1675-1676`).

The policy page exposes a separate `Correct answer` timing selector
(`src/pages/assessment_workspace/assessment_workspace_policies_page.tsx:50-59`,
`726-743`). However, `gate_quiz_exam_answers_for_current_cohort` forcibly clears
both the answer and answer explanation whenever a Quiz or Exam has an
incomplete current cohort, regardless of the selected timing
(`crates/domain/src/student_feedback_release.rs:44-61`). The test
`quiz_and_exam_answers_wait_for_every_current_student` deliberately verifies
that permanent gate (`crates/domain/src/student_feedback_release/tests.rs:24-50`).

The code therefore has the setting but does not honor an Instructor override.
This is separate from A-01: it concerns correct answers, not score visibility.

## Freshly supported behaviors

### S-01 - current point values rescale stored credit without Backend work

**Status: supported by source and focused unit test.**

HG requires stored immutable credit fractions, score calculation from current
point values, and recalculation without another Backend interaction
(`HUMAN_GUIDANCE.md:1145-1150`, `1759-1775`). The finalization read projects the
stored `normalized_credit` through the current fixed-Question or Pool point
value (`schemas/base_schema/50_functions/assessment_attempt_finalization.sql:63-78`).
`current_points_recalculate_without_changing_recorded_credit` passed and checks
that a stored `0.67` credit changes from `1.34` at two points to `2.01` at three
points while the stored credit remains `0.67`
(`crates/question_model/src/student_work/grading.rs:217-234`).

This support applies to point-value edits. It does not cure the Pool-removal
fairness gap above.

### S-02 - Unrelease remains a distinct destructive reset

**Status: source and existing end-to-end coverage present; not rerun in this audit.**

The current repository has a dedicated Unrelease API and a connected oracle
(`schemas/base_schema/50_functions/assessment_operations.sql:218-237`; `tests/e2e/e2e_unrelease_connected.sh`). This is consistent with HG's rule that
Unrelease deletes Student Work and then permits ordinary editing
(`HUMAN_GUIDANCE.md:1582`, `1683-1691`). I did not rerun the database-connected
oracle, so this is not fresh runtime certification.

### S-03 - submission-viewing and correct-answer timing controls are separate

**Status: supported by source; no new browser test.**

HG calls for separate timing settings for viewing submissions and correct
answers (`HUMAN_GUIDANCE.md:1666-1667`). The policy model stores
`submitted_response` independently from `question_answer`
(`schemas/base_schema/50_functions/assessment_attempt_history.sql:77-85`), and
the Assessment Properties editor renders each as a separate selector
(`src/pages/assessment_workspace/assessment_workspace_policies_page.tsx:50-59`,
`726-743`). This finding concerns the independence of the controls only;
A-05 covers the Quiz/Exam answer override that currently defeats the selected
answer timing.

## Audit conclusion

The essential pre-fix work is concentrated in five connected areas: remove
score withholding; enforce the narrow post-issue edit set at the trusted write
boundaries; prevent removal of Pool members already issued; make whole-Pool
removal zero both earned and possible points for every Attempt; and turn the
Quiz/Exam all-completed rule back into an Instructor-changeable default. The
existing current-point recalculation mechanism is a useful foundation, but it
is insufficient for entry removal by itself.
