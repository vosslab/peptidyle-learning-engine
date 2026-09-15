## Assessments

- [ ] **Assessment** is the PLE object for organizing Questions into a graded or practice activity.
  - Mismatch: The implemented product calls this object an Assignment.
- [ ] PLE has **Blueprint Assessments** and **Course Instance Assessments**.
  - Mismatch: No Assessment model with these two product categories was found.
- [ ] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Mismatch: Blueprint Assignment source exists, but does not establish the HG Assessment model.
- [ ] Course Instance Assessments deliver Questions to **Students**.
  - Mismatch: Delivery source uses Assignment terminology and contract.
- [ ] All Assessments use the same underlying Assessment model.
  - Mismatch: No shared Assessment model was found.
- [ ] **Assignment** is not a separate object or category. The word appears only in the names
  **Regular Assignment**, **Practice Question Assignment**, and **Bonus Assignment**.
  - Mismatch: Assignment is the current general object name throughout the product.

### Assessment content

- [ ] Assessments contain an ordered sequence of Questions and Question Pools.
  - Mismatch: Assignment entry source was found, but the HG Assessment behavior is not verified.
- [ ] **Instructors** can add, remove, and reorder Questions and Question Pools.
  - Mismatch: Source was not verified through the complete instructor behavior.
- [ ] Questions and Question Pools remain distinct even though both can occupy positions in an Assessment.
  - Mismatch: Current Assignment entry types do not establish the named Assessment behavior.
- [ ] Assessment Question-order randomization is called **Randomize question order**.
  - Mismatch: No matching product label was found.

### Assessment types

- [ ] PLE defines the available Assessment Types.
  - Mismatch: No Assessment Type model was found.
- [ ] Assessment Type describes the pedagogical purpose of an Assessment and provides appropriate defaults.
  - Mismatch: No Assessment Type defaults were found.
- [ ] Assessment Types are **Regular Assignment**, **Practice Question Assignment**, **Bonus Assignment**, **Quiz**, and **Exam**.
  - Mismatch: The five required types are not implemented as a model.
- [ ] **Instructors** select an Assessment Type but cannot create new Assessment Types.
  - Mismatch: No instructor selection-only Assessment Type workflow was found.
- [ ] Blueprint Assessments and Course Instance Assessments use the same Assessment Types.
  - Mismatch: No shared Assessment Type implementation was found.
- [ ] **Instructors** can change Assessment settings independently of the defaults for its Type.
  - Mismatch: No type-default override behavior was verified.
- [ ] Changing Assessment settings does not change its Assessment Type.
  - Mismatch: No Assessment Type identity behavior was found.
- [ ] **Regular Assignments** give **Students** regular practice applying course ideas outside class.
  - Mismatch: No Regular Assignment type behavior was found.
- [ ] Regular Assignments reinforce current learning and may also introduce new topics.
  - Mismatch: No Regular Assignment type behavior was found.
- [ ] Regular Assignments are designed as practice for learning, not merely as one-time assessments.
  - Mismatch: No Regular Assignment type behavior was found.
- [ ] **Practice Question Assignments** provide focused review or study-guide practice using material already covered.
  - Mismatch: No Practice Question Assignment type behavior was found.
- [ ] Practice Question Assignments may be worth a small number of points or a small amount of extra credit.
  - Mismatch: No Practice Question Assignment type behavior was found.
- [ ] Practice Question Assignments use the same whole-Attempt submission boundary as every other
  Assessment and show the correct answer immediately after that Assessment Attempt is submitted.
  - Mismatch: Whole-attempt source exists, but type-specific disclosure is not verified.
- [ ] **Bonus Assignments** provide optional extra credit.
  - Mismatch: No Bonus Assignment type behavior was found.
- [ ] Bonus Assignments are worth zero points possible and add earned points directly to the grade.
  - Mismatch: No Bonus Assignment scoring behavior was found.
- [ ] **Quizzes** assess understanding of recent material.
  - Mismatch: No Quiz type behavior was found.
- [ ] Quizzes may use more restrictive Attempt and collaboration settings than Regular Assignments.
  - Mismatch: No type-specific settings behavior was found.
- [ ] **Exams** are individual assessments associated with scheduled exam periods.
  - Mismatch: No Exam type behavior was found.
- [ ] Exams may use more restrictive Attempt, timing, availability, and feedback settings.
  - Mismatch: No type-specific settings behavior was found.

### Assessment type appearance

- [ ] Each Assessment Type has its own PLE-defined Font Awesome icon.
  - Mismatch: No Assessment Type icon mapping was found.
- [ ] Assessment Type icons remain consistent across PLE themes.
  - Mismatch: No Assessment Type icon mapping was found.
- [ ] Each Assessment Type also has its own theme-defined color.
  - Mismatch: No Assessment Type color mapping was found.
- [ ] Themes may change Assessment Type colors but preserve the meaning of each Type.
  - Mismatch: No Assessment Type theme contract was found.
- [ ] Assessment Type should never be communicated by color alone.
  - Mismatch: No implemented Assessment Type presentation was found.
- [ ] Icons and labels should remain sufficient to identify the Assessment Type without color.
  - Mismatch: No implemented Assessment Type presentation was found.
- [ ] **Regular Assignment** uses the Font Awesome `pen-to-square` icon.
  - Mismatch: No required icon mapping was found.
- [ ] **Practice Question Assignment** uses the Font Awesome `arrows-spin` icon.
  - Mismatch: No required icon mapping was found.
- [ ] **Bonus Assignment** uses the Font Awesome `sparkles` icon.
  - Mismatch: No required icon mapping was found.
- [ ] **Quiz** uses the Font Awesome `square-q` icon.
  - Mismatch: No required icon mapping was found.
- [ ] **Exam** uses the Font Awesome `file-signature` icon.
  - Mismatch: No required icon mapping was found.

### Blueprint Assessments

- [ ] A **Blueprint Assessment** is an Assessment in a **Blueprint Course**.
  - Mismatch: Blueprint Assignment source does not implement the HG object naming/model.
- [ ] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Mismatch: Blueprint Assignment source does not establish the HG Assessment model.
  - Owner: Same implementation finding as the earlier Assessments bullet.
- [ ] Blueprint Assessments have an Assessment Type.
  - Mismatch: No Assessment Type field was found.
- [ ] Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
  - Mismatch: Ordered entries exist, but published Question and Pool Assessment behavior is not verified.
  - Owner: Same implementation finding as the earlier Courses bullet.
- [ ] Blueprint Assessments define Question point values and points possible.
  - Mismatch: Point values exist in Assignment source, but Blueprint Assessment behavior is not verified.
- [ ] Blueprint Assessments have no **Students**, Student Work, due dates, release dates, or other Course Instance delivery settings.
  - Mismatch: Blueprint Assignment source does not establish this full absence contract.
- [ ] Blueprint Assessments do not use Assessment Templates.
  - Mismatch: No Assessment Template model was found.
- [ ] Creating a daughter Course Instance from a Blueprint Course copies its Blueprint Assessments into the Course Instance.
  - Mismatch: Copy behavior is outside this source-only verification and uses Assignment terminology.

### Course Instance Assessments

- [ ] A **Course Instance Assessment** is an Assessment in a **Course Instance**.
  - Mismatch: The implemented object is an Assignment.
- [ ] Course Instance Assessments are the Assessments delivered to **Students**.
  - Mismatch: Delivery source implements Assignments rather than the HG Assessment model.
- [ ] Course Instance Assessments have an Assessment Type, Questions, Question Pools, point values, and points possible.
  - Mismatch: No Assessment Type model was found.
- [ ] Course Instance Assessments also have delivery settings such as due dates, release status, and Student availability.
  - Mismatch: Assignment delivery settings exist, but the complete HG Assessment behavior is not verified.
- [ ] Course Instance Assessments copied from a Blueprint Assessment can be changed for the needs of that Course Instance.
  - Mismatch: No verified HG Assessment copy-and-edit behavior was found.
- [ ] Newly added Blueprint Assessments are automatically copied to daughter Course Instances as unreleased Course Instance Assessments.
  - Mismatch: No verified automatic Assessment propagation behavior was found.

### Assessment Templates

- [ ] An **Assessment Template** is a reusable set of settings for creating Course Instance Assessments.
  - Mismatch: No Assessment Template model was found.
- [ ] Assessment Templates are separate from Assessment Types.
  - Mismatch: Neither model was found.
- [ ] Every Assessment Template has one of the five Assessment Types.
  - Mismatch: No Assessment Template or five-type model was found.
- [ ] **Instructors** can create and change their own Assessment Templates.
  - Mismatch: No Assessment Template workflow was found.
- [ ] Assessment Templates provide defaults for settings such as Attempts, timing, scoring, and disclosure.
  - Mismatch: No Assessment Template defaults behavior was found.
- [ ] Creating a Course Instance Assessment from a Template copies its settings into the new Assessment.
  - Mismatch: No Assessment Template creation behavior was found.
- [ ] The new Course Instance Assessment can be changed independently after it is created.
  - Mismatch: No Assessment Template creation behavior was found.
- [ ] Changing an Assessment Template does not change Assessments previously created from it.
  - Mismatch: No Assessment Template behavior was found.
- [ ] Assessment Templates do not contain Questions or Question Pools.
  - Mismatch: No Assessment Template model was found.
- [ ] Blueprint Assessments do not use Assessment Templates.
  - Mismatch: No Assessment Template model was found.
  - Owner: Same implementation finding as the earlier Assessment Templates bullet.

### Course Instance Assessment release and defaults

- [ ] Course Instance Assessments start unreleased.
  - Mismatch: Assignment release exists, but Course Instance Assessment behavior is not verified.
- [ ] Releasing a Course Instance Assessment requires an automated and interactive **Assessment Release Validation** process.
  - Mismatch: Assignment release validation exists, but the HG Assessment workflow is not verified.
- [ ] Assessment Release Validation checks the Assessment settings and data required for release.
  - Mismatch: Current validation is Assignment-specific.
- [ ] Validation should catch missing, invalid, or unreasonable values and explain what the **Instructor** needs to fix.
  - Mismatch: Complete instructor-facing validation behavior was not verified.
- [ ] Release Validation should require a due date at least 24 hours in the future and no later than the
  Course Instance's six-month Active limit.
  - Mismatch: No verified 24-hour and six-month release rule was found.
- [ ] Release Validation should check that release, due, and other dates occur in a valid order.
  - Mismatch: No verified complete date-order validation was found.
- [ ] Release Validation should check required settings such as point values, Attempt limits, and time limits for valid ranges.
  - Mismatch: No verified complete required-setting validation was found.
- [ ] Release Validation should check that the Assessment contains Questions and that required Question settings are valid.
  - Mismatch: No verified Assessment Question validation was found.
- [ ] The **Instructor** should be able to correct validation problems and run Release Validation again.
  - Mismatch: No verified correction-and-rerun workflow was found.
- [ ] An Assessment can be released only after Release Validation passes.
  - Mismatch: Existing release endpoint does not establish the HG Assessment gate.
- [ ] Releasing an Assessment makes it available to **Students** according to its dates and access settings.
  - Mismatch: Assignment delivery source exists, but live access behavior needs runtime evidence.
- [ ] Student Work begins when a **Student** starts an Assessment Attempt.
  - Mismatch: Attempt issuance source exists, but live Student Work behavior needs runtime evidence.
- [ ] New Course Instance Assessments default to accepting submissions only through the due date.
  - Mismatch: No verified default was found.
- [ ] New Course Instance Assessments default to starting new Attempts only through the due date.
  - Mismatch: No verified default was found.
- [ ] Late work defaults to rejected.
  - Mismatch: No verified default was found.
- [ ] Assessment disclosure settings remain separate and independently configurable.
  - Mismatch: No Assessment disclosure model was found.
- [ ] **Regular Assignments** and **Bonus Assignments** should rarely show the correct answer.
  - Mismatch: No type-specific disclosure behavior was found.
- [ ] Regular and Bonus Assignments show the **Student's** response and whether it was correct or incorrect.
  - Mismatch: No type-specific disclosure behavior was found.
- [ ] **Practice Question Assignments** show correct answers immediately after Assessment Attempt
  submission.
  - Mismatch: No type-specific disclosure behavior was found.
- [ ] **Quizzes** and **Exams** show correct answers after all **Students** in the Course have completed the Assessment.
  - Mismatch: No type-specific disclosure behavior was found.
- [ ] Until then, Quizzes and Exams do not disclose correct answers.
  - Mismatch: No type-specific disclosure behavior was found.
- [x] Optional Question Feedback is shown when the Question Backend provides it.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_student_feedback` releases backend-provided feedback.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] Question Feedback does not use Assessment correct-answer disclosure settings.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `StudentFeedbackReleaseDecision` has a separate question-feedback policy field.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] Unreleasing a Course Instance Assessment permanently deletes its Student Work and returns to a pre-release state.
  - Evidence (test): `tests/e2e/e2e_unrelease_connected.sh` `psql_admin` runs the connected unrelease deletion and pre-release-state oracle.

### Assessment Attempts

- [ ] An **Assessment Attempt** is one Student attempt at a Course Instance Assessment.
  - Mismatch: The implemented object is an Assignment Attempt.
- [ ] Blueprint Assessments do not have Assessment Attempts.
  - Mismatch: No Blueprint Assessment model was found.
- [ ] Question responses are saved as the **Student** works and remain part of the Attempt across browser sessions.
  - Mismatch: Saved-response persistence is tested after a browser reload, but no evidence establishes persistence across a distinct browser session.
- [ ] **Instructors** control the number of permitted Assessment Attempts.
  - Mismatch: Assignment attempt limit exists, but instructor behavior is not verified.
- [ ] Regular Assignments default to unlimited Attempts.
  - Mismatch: No Regular Assignment type default was found.
- [ ] **Students** may repeat an Assessment as often as its settings allow, including practicing toward a perfect score.
  - Mismatch: Repeat behavior needs runtime evidence.
- [x] When an Assessment permits multiple Attempts, the highest Assessment Attempt score is used as the
  Student's Assessment score.
  - Evidence (test): `crates/domain/src/scoring.rs` `hand_computed_fixture_agrees_for_batch_and_incremental_scoring` proves `AssignmentAttemptGradeRule::Highest` selects the highest completed Attempt score.
- [x] Assessment Attempt submission and grading are fully automatic and require no **Instructor** action.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` commits ordinary Student finalization and checks immutable automated grading evidence.
- [x] Automatic grading does not require a separate Student or **Instructor** grading workflow.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` proves the Student finalization API creates the grading result directly.

### Assessment responses and submission

- [x] The Student submission action submits the whole Assessment Attempt.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` commits one ordinary Student Attempt finalization and asserts one Assignment submission.
- [ ] A Question either has a complete saved response or has no saved response.
  - Mismatch: Complete-response contract was not verified.
- [x] PLE saves complete Question responses as the **Student** works.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` saves an ordinary Student response and asserts its persisted `saved` state.
- [x] The Student may change a saved response while the Assessment Attempt remains open.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` replaces saved response A with response B before finalization and rejects the stale A snapshot.
- [x] Submitting the Assessment Attempt finalizes all saved Question responses together as Student Work.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` commits the prepared saved response into immutable submission and grading evidence.
- [ ] Questions without a saved response remain visibly unanswered when the Attempt is submitted.
  - Mismatch: Visible unanswered-state behavior needs runtime evidence.
- [x] An unanswered Question receives zero credit and counts as incorrect without being sent to the
  Question Backend.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_expired_student_assignment_attempt_finalization` verifies an expired all-unanswered Attempt has no invented backend result and zero-credit scoring.
- [ ] PLE treats an incomplete Question response as unsaved, although the Question interface may keep the Student's unfinished input while they work.
  - Mismatch: Incomplete response behavior was not verified.
- [ ] A Question Backend may evaluate a response before Assessment submission when needed for its interaction.
  - Mismatch: No verified pre-submission backend evaluation behavior was found.
- [ ] When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
  - Mismatch: No test/runtime proof of immediate backend grading outcome was recorded.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [ ] The **Student** does not see the grading outcome until the Assessment Attempt is submitted.
  - Mismatch: Student grading-outcome timing needs runtime evidence.

### Assessment Attempt timing and expiration

- [ ] Each Assessment Attempt has a time limit.
  - Mismatch: Assignment time limit is optional rather than required for each attempt.
- [ ] Attempt time limits help **Students** develop an accurate sense of expected working speed.
  - Mismatch: This pedagogical effect is not implemented as verifiable product behavior.
- [x] Timed Assessment Attempts use wall-clock time.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` observes database `clock_timestamp()` reach the persisted expiry before rejecting late work.
- [x] The server owns the Attempt start and expiration times.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` reads the persisted server `expires_at` against database `clock_timestamp()`.
- [x] Attempt time continues while the **Student** is disconnected or the browser is closed.
  - Evidence (test): `tests/e2e/e2e_live_demo_assignment_attempt.sh` `prove_background_expiry` proves the generic worker finalizes an expired Attempt without a further Student interaction.
- [ ] A **Student** may reconnect, reload, or use another browser session to resume the same active Attempt.
  - Mismatch: `tests/e2e/e2e_live_demo_assignment_attempt.sh` `prove_start` repeats start with the same cookie; it does not prove reconnect, reload, or a distinct browser session.
- [ ] Resuming an Attempt does not reset, pause, or extend its time limit.
  - Mismatch: Resume source returns the active Attempt, but no test or runtime observation verifies that its time limit remains unchanged.
- [x] Attempt expiration is checked whenever a **Student** interacts with the Attempt.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` proves a save rechecks the server clock after lock waiting and returns `expired`.
- [x] Background processing ensures expired Attempts are submitted even when the **Student** is no longer connected.
  - Evidence (test): `tests/e2e/e2e_live_demo_assignment_attempt.sh` `prove_background_expiry` waits only for generic-worker immutable evidence, then proves the expired Attempt is submitted.
- [x] When an Attempt expires, PLE submits the whole Attempt, finalizing its saved responses. Other
  Questions remain visibly unanswered, receive zero credit, and count as incorrect without being
  sent to the Question Backend.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_expired_student_assignment_attempt_finalization` checks deadline finalization of saved work and no invented result for unanswered Questions.

### Student Work

- [x] Student Work keeps the exact Published Question Revision delivered to the **Student**.
  - Evidence (source): `schemas/base_schema/attempts.sql` `issued_question_is_immutable` stores issued Question revision identity under foreign-key protection.
- [ ] For a Question Pool, Student Work keeps the exact Question Pool Revision and Published Question Revision selected.
  - Mismatch: Pool selection retention is not verified.
- [x] Student Work keeps each saved response as finalized with the submitted Attempt and the grading outcome returned by the Question Backend.
  - Evidence (test): `tests/e2e/e2e_live_demo_assignment_attempt.sh` `prove_webwork_submission` submits a live WeBWorK-backed response and checks its renderer-derived stored credit against the submitted score.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` proves the saved response, submitted Attempt, stored credit, and automated receipt commit together without later replacement.
- [ ] Changes to Assessment content do not replace Question evidence already delivered in existing Attempts.
  - Mismatch: Historical evidence isolation is not verified.
- [ ] PLE should retain only the additional historical Student Work data needed to interpret or grade that work correctly.
  - Mismatch: No minimal-retention contract was verified.

### Assessment scoring

- [ ] Blueprint Assessments and Course Instance Assessments assign point values to Questions.
  - Mismatch: Assignment point values exist, but the HG Assessment model is not implemented.
- [ ] A Question Backend returns an immutable credit fraction for each complete response it evaluates.
  - Mismatch: The live WeBWorK submission check observes stored credit, but no direct adapter test proves a Question Backend returns that fraction as an immutable outcome.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] PLE stores the credit fraction as the Question grading outcome.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` reads the immutable stored normalized credit after finalization.
- [x] Course Instance Assessment scores are calculated from stored credit fractions and current Question point values.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` changes current entry points and proves the same stored 0.67 credit rescales the score.
- [x] An unanswered Question contributes zero points to the Assessment score and counts as incorrect.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_expired_student_assignment_attempt_finalization` asserts an all-unanswered expired Attempt remains in the denominator at zero credit.
- [x] When an Assessment has multiple submitted Attempts, the highest Assessment Attempt score is the
  Student's Assessment score.
  - Evidence (test): `crates/domain/src/scoring.rs` `hand_computed_fixture_agrees_for_batch_and_incremental_scoring` proves `AssignmentAttemptGradeRule::Highest` selects the highest completed Attempt score.
  - Owner: Same implementation finding as the earlier Assessment Attempts bullet.
- [x] PLE uses Question point values directly to calculate Assessment scores.
  - Evidence (test): `crates/question_model/src/student_work/grading.rs` `current_points_recalculate_without_changing_recorded_credit` tests current point values rescale recorded credit directly.
- [ ] PLE does not use separate Question weights, Grade Categories, weighted categories, Course Grade
  Schemes, or Course percentage calculations.
  - Mismatch: Absence of every prohibited model was not verified.
- [ ] For the pilot, grade export uses CSV or TSV only and exports point-based Assessment scores.
  - Mismatch: No grade export implementation matching this contract was found.
- [ ] The Instructor handles Course-level weighting or percentage calculations in the home LMS.
  - Mismatch: No product boundary or export guidance establishing this behavior was verified.
- [x] Changing Question point values recalculates affected Assessment scores.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` changes entry points from two to three and proves score recalculation.
- [x] Score recalculation does not require another Question Backend interaction.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` replays finalization with an empty result array after changing current points and proves the stored credit rescores without another supplied backend outcome.
- [x] Score recalculation does not change the stored Question grading outcome.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` proves rescoring retains stored normalized credit and the original receipt.
