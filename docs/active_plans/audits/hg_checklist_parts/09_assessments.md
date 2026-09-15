## Assessments

- [ ] **Assessment** is the PLE object for organizing Questions into a graded or practice activity.
  - Mismatch: The implemented product calls this object an Assignment.
- [x] PLE has **Blueprint Assessments** and **Course Instance Assessments**.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` is the reusable Blueprint Assessment aggregate; `schemas/base_schema/assessments.sql` `ple_data.assessment` is the current Course Instance Assessment aggregate with a required `course_id`.
- [x] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` contains Type, title, instructions, ordered entries, and validated `BlueprintAssessmentDefaults`; `BlueprintCourseModuleContent` owns those Assessments in Blueprint Course content.
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

- [x] PLE defines the available Assessment Types.
  - Evidence (source): `crates/question_model/src/assessment.rs` `AssessmentType` is the canonical closed model, and `generated/api/AssessmentType.ts` `ASSESSMENT_TYPE_VALUES` carries it to the browser contract.
- [ ] Assessment Type describes the pedagogical purpose of an Assessment and provides appropriate defaults.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule::for_assessment_type`, `schemas/base_schema/assessments.sql` `create_assessment`, `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults`, and `crates/project-tools/src/curriculum_content/publication.rs` apply the accepted Type-aware disclosure defaults at the real creation boundaries.
  - Mismatch: These sources establish the C524 disclosure subset, not every appropriate Type default or the full pedagogical-purpose behavior required by this broad row.
- [x] Assessment Types are **Regular Assignment**, **Practice Question Assignment**, **Bonus Assignment**, **Quiz**, and **Exam**.
  - Evidence (source): `crates/question_model/src/assessment.rs` `AssessmentType::ALL` contains exactly the five HG values; schema constraints and `generated/api/AssessmentType.ts` `ASSESSMENT_TYPE_VALUES` use the same closed set.
- [x] **Instructors** select an Assessment Type but cannot create new Assessment Types.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_create_page.tsx` `AssessmentWorkspaceCreatePage` requires one canonical Type before creation; `src/features/blueprint_course/blueprint_course_create_dialog.tsx` `BlueprintCourseCreateDialog` does the same for reusable content, with no free-form Type path or silent default.
  - Evidence (test): `tests/test_blueprint_course_ui.mjs` `createdContent` proves the selected canonical Type reaches creation; the focused Blueprint UI/model lane passed 10/10.
- [x] Blueprint Assessments and Course Instance Assessments use the same Assessment Types.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `assessment_type` and `schemas/base_schema/assessments.sql` `assessment_type` validate the same five values; `schemas/base_schema/course_blueprint_adoption.sql` `assessment_type` copies the selected Type during adoption.
- [x] **Instructors** can change Assessment settings independently of the defaults for its Type.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `EffectiveAssessmentPolicy` resolves explicit Course Instance property overrides separately from Blueprint defaults and Assessment Type.
  - Evidence (source): `crates/learning-data-access/tests/assessment_policies_postgres.rs` `policy_save_is_isolated_conflict_checked_and_reports_unreleased_invalid_dates` seeds an adopted Quiz with `before policy save` instructions and a 300-second limit, then sends independent `persisted policy` instructions and a 600-second limit through the Properties save input.
  - Decision: The current ignored PostgreSQL acceptance fixture has not been rerun after this update. It is source evidence of the mutable instruction/time-limit case, not a current runtime acceptance receipt.
- [x] Changing Assessment settings does not change its Assessment Type.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `EffectiveAssessmentPolicy` does not expose Type as an editable property, while `crates/question_model/src/assessment.rs` `AssessmentType` remains part of Assessment identity.
  - Evidence (source): `crates/learning-data-access/tests/assessment_policies_postgres.rs` `policy_save_is_isolated_conflict_checked_and_reports_unreleased_invalid_dates` uses a closed policy-save input with instructions and a time limit, but no Assessment Type field; its adopted Quiz fixture retains the fixed Type while those settings are changed.
  - Decision: An actual PostgreSQL acceptance rerun of the current fixture remains pending; this does not cite the retired Quiz Attempt-limit 3-to-5 runtime claim.
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
  - Evidence (runtime): accepted PostgreSQL 17 proof through the ordinary start, whole-submission, and history APIs returned zero history response-source rows before submission and one after submission; the native PLE summary then preserved the disclosed correct answer.
  - Mismatch: Backend-owned answers currently project as absent. Opaque WeBWorK post-submit answer disclosure remains unimplemented and requires renderer/adapter work without answer extraction, so the cross-backend row remains open.
- [ ] **Bonus Assignments** provide optional extra credit.
  - Mismatch: No Bonus Assignment type behavior was found.
- [x] Bonus Assignments are worth zero points possible and add earned points directly to the grade.
  - Evidence (source): `schemas/base_schema/grading.sql` `grade_contribution_points_possible` makes Bonus points possible zero while `score_recorded_credit` continues to supply earned points; `schemas/base_schema/grading_access.sql` applies that contribution through the real Gradebook helper, and `schemas/base_schema/student_assessment_landing.sql` projects the same selected contribution to the Student API.
  - Evidence (runtime): accepted actual PostgreSQL 17 proofs exercised `schemas/base_schema/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments`, returning earned Bonus points with a zero denominator; the Student row was `8 / 0`. The strict Student decoder accepts the complete pair, and compiled M6 component evidence rendered it without changing raw Assessment Attempt scoring.
- [ ] **Quizzes** assess understanding of recent material.
  - Mismatch: No Quiz type behavior was found.
- [ ] Quizzes may use more restrictive Attempt and collaboration settings than Regular Assignments.
  - Mismatch: No type-specific settings behavior was found.
- [ ] **Exams** are individual assessments associated with scheduled exam periods.
  - Mismatch: No Exam type behavior was found.
- [ ] Exams may use more restrictive Attempt, timing, availability, and feedback settings.
  - Mismatch: No type-specific settings behavior was found.
- [ ] Quizzes and Exams allow one Assessment Attempt.
  - Mismatch: Quiz and Exam Attempt limits remain configurable rather than being enforced as one.

### Assessment type appearance

- [x] Each Assessment Type has its own PLE-defined Font Awesome icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` assigns one required Font Awesome icon name and bundled glyph to every canonical Type.
- [x] Assessment Type icons remain consistent across PLE themes.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` owns the theme-independent Type-to-icon mapping.
- [x] Each Assessment Type also has its own theme-defined color.
  - Evidence (source): `src/styles/assessment_types.css` `--ple-assessment-type-regular-assignment` defines the five semantic Type color properties at the root and Course-theme scope.
- [x] Themes may change Assessment Type colors but preserve the meaning of each Type.
  - Evidence (source): `src/styles/assessment_types.css` `course-theme-scope` overrides the five semantic Type properties by Type identity; an accepted nested-theme fixture verified the actual scoped cascade.
- [x] Assessment Type should never be communicated by color alone.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentCard` always renders the Type label and its genuine bundled glyph; color is supplementary.
- [x] Icons and labels should remain sufficient to identify the Assessment Type without color.
  - Evidence (source): `src/assessment_type_presentation.ts` `assessmentTypePresentation` returns the canonical Type label and bundled icon metadata independently of color.
- [x] **Regular Assignment** uses the Font Awesome `pen-to-square` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Regular Assignment to `pen-to-square` and the genuine bundled glyph.
- [x] **Practice Question Assignment** uses the Font Awesome `arrows-spin` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Practice Question Assignment to `arrows-spin` and the genuine bundled glyph.
- [x] **Bonus Assignment** uses the Font Awesome `star` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Bonus Assignment to `star` and the genuine bundled glyph.
- [x] **Quiz** uses the Font Awesome `circle-question` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Quiz to `circle-question` and the genuine bundled glyph.
- [x] **Exam** uses the Font Awesome `file-signature` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Exam to `file-signature` and the genuine bundled glyph.

### Blueprint Assessments

- [x] A **Blueprint Assessment** is an Assessment in a **Blueprint Course**.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintCourseModuleContent` contains ordered `BlueprintAssessmentContent`; `schemas/base_schema/blueprints.sql` `blueprint_revision_assessment` records each stable Blueprint Assessment member of a Blueprint Course Revision.
- [x] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` validates reusable content and `BlueprintAssessmentDefaults` before a Blueprint Course Revision is constructed.
  - Owner: Same implementation finding as the earlier Assessments bullet.
- [x] Blueprint Assessments have an Assessment Type.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `assessment_type` requires the closed five-Type value; `src/api/decoders/blueprint_course.ts` `assessmentType` strictly requires it in both reusable-content input and view decoding without fallback.
  - Evidence (test): `tests/test_blueprint_course_client.mjs` `B1 client sends Revision and metadata validators to their separate routes` covers create/save/view Type round trips plus missing and unknown rejection; the focused Blueprint client lane passed 16/16.
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

- [x] A **Course Instance Assessment** is an Assessment in a **Course Instance**.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.assessment` requires `course_id` referencing `ple_data.course_instance`; `crates/question_model/src/assessment.rs` `AssessmentOrigin` names direct creation in a Course Instance and adopted creation from an exact Blueprint Assessment.
- [ ] Course Instance Assessments are the Assessments delivered to **Students**.
  - Mismatch: Delivery source implements Assignments rather than the HG Assessment model.
- [x] Course Instance Assessments have an Assessment Type, Questions, Question Pools, point values, and points possible.
  - Evidence (source): `schemas/base_schema/assessments.sql` `assessment_type` adds the closed Type to the existing Assessment content, Pool, and points model; `schemas/base_schema/course_blueprint_adoption.sql` `assessment_type` preserves it during adoption.
  - Evidence (source): `crates/learning-data-access/tests/assessment_policies_postgres.rs` `policy_save_is_isolated_conflict_checked_and_reports_unreleased_invalid_dates` supplies the current adopted Quiz fixture: it changes instructions and a 300-second time limit to 600 seconds through a Type-free Properties input.
  - Decision: The prior Quiz Attempt-limit 3-to-5 runtime wording is retired. The current ignored PostgreSQL fixture still needs an actual acceptance rerun; this source evidence does not manufacture one.
- [ ] Course Instance Assessments also have delivery settings such as due dates, release status, and Student availability.
  - Mismatch: Assignment delivery settings exist, but the complete HG Assessment behavior is not verified.
- [x] Course Instance Assessments copied from a Blueprint Assessment can be changed for the needs of that Course Instance.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `initialize_course_assessments` creates a fresh Course Assessment with immutable adopted provenance; `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` updates that Course Assessment through its Course-owned reference.
  - Evidence (runtime): accepted C503 PostgreSQL 17 evidence covered adopted Assessment load/save with exact Blueprint Course, Revision, and Assessment provenance retained; `crates/learning-data-access/src/postgres/assessment_release.rs` `PostgresLiveAssessmentStore` exercised the adopted load/save production mapper. The standalone C503 proof did not rerun the full publisher-backed installation-data seed.
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
- [x] Releasing a Course Instance Assessment requires an automated and interactive **Assessment Release Validation** process.
  - Evidence (source): `schemas/base_schema/assessment_operations.sql` `ple_api.validate_assessment_release` projects the trusted release issues only to an authorized Course Instructor; `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` invokes it from the unreleased Assessment Properties workflow.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-API receipt proved the authorized validation/release boundary and rollback behavior at `schemas/base_schema/assessment_operations.sql` `ple_api.validate_assessment_release`; the accepted actual Properties component receipt exercised the interactive readiness flow at `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`.
- [ ] Assessment Release Validation checks the Assessment settings and data required for release.
  - Mismatch: The canonical `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues` checks the current release conditions, but complete required-setting coverage remains unverified; see the separate valid-range and Question-validity rows.
- [ ] Validation should catch missing, invalid, or unreasonable values and explain what the **Instructor** needs to fix.
  - Mismatch: The accepted evidence demonstrates five actionable date issues (missing Due, 24-hour and Course Active-limit boundaries, and two date-order violations), correction, and rerun through `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues` and `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`. It does not establish missing, invalid, or unreasonable validation beyond dates; the required-setting valid-range and Question-validity rows remain open.
- [x] Release Validation should require a due date at least 24 hours in the future and no later than the
  Course Instance's six-month Active limit.
  - Evidence (source): `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues` rejects a missing Due date, a Due date less than 24 hours ahead at release, and a Due date after `course_instance.active_until_at`.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-API receipt exercised the missing-Due, 24-hour, and Course Active-limit boundaries from `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues`.
- [x] Release Validation should check that release, due, and other dates occur in a valid order.
  - Evidence (source): `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues` rejects Available after Due and Due after Closes.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-API receipt exercised both invalid orderings and the corrected valid ordering from `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues`.
- [ ] Release Validation should check required settings such as point values, Attempt limits, and time limits for valid ranges.
  - Mismatch: No verified complete required-setting validation was found.
- [ ] Release Validation should check that the Assessment contains Questions and that required Question settings are valid.
  - Mismatch: No verified Assessment Question validation was found.
- [x] The **Instructor** should be able to correct validation problems and run Release Validation again.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` keeps Assessment Properties editable, provides issue-specific save-and-rerun instructions, and exposes `Check release readiness` again after correction.
  - Evidence (runtime): accepted actual Properties component receipt exercised invalid dates, correction, a second readiness check, and release through `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`.
- [x] An Assessment can be released only after Release Validation passes.
  - Evidence (source): `schemas/base_schema/assessment_release_validation.sql` `ple_data.validate_assessment_release` raises on every issue, and `schemas/base_schema/assessment_operations.sql` `ple_data.release_assessment` invokes that hard gate before changing release state.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-API receipt proved unauthorized access is denied, invalid release rolls back, and corrected release succeeds through `schemas/base_schema/assessment_operations.sql` `ple_api.release_assessment`.
- [ ] Releasing an Assessment makes it available to **Students** according to its dates and access settings.
  - Mismatch: Assignment delivery source exists, but live access behavior needs runtime evidence.
- [ ] Student Work begins when a **Student** starts an Assessment Attempt.
  - Mismatch: Attempt issuance source exists, but live Student Work behavior needs runtime evidence.
- [x] New Course Instance Assessments default to accepting submissions only through the due date.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.create_assessment` applies the `reject` late-work default; `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` pins the immutable expiration to the effective Due date for that rule and `ple_private.save_student_assessment_attempt_response` rejects an expired save. Explicit `accept` and `mark_late` overrides remain valid and do not use Due as expiration.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-Student-API proofs exercised `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.save_student_assessment_attempt_response` and the ordinary finalization API, rejecting post-Due save and commit for the default rule while the expiry worker retained both accepted pre-Due saved responses; explicit `accept` and `mark_late` cases remained open through Due and used Closes.
- [x] New Course Instance Assessments default to starting new Attempts only through the due date.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.create_assessment` applies the `reject` late-work default; `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` rejects a post-Due start and uses the effective accommodated Due date when present. Explicit `accept` and `mark_late` overrides remain valid.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-Student-API proofs exercised `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt`, rejecting a post-Due start under `reject`, honoring an accommodated Due date, and keeping explicit `accept` and `mark_late` cases available through Closes.
- [x] Late work defaults to rejected.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.create_assessment`, `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults`, and `crates/project-tools/src/curriculum_content/publication.rs` `blueprint_input` apply `reject` at direct and reusable-content default creation boundaries while preserving explicit Instructor overrides.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-API proof exercised `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` and `ple_private.save_student_assessment_attempt_response` through immutable Due expiry and post-Due start/save/commit denial, while confirming that explicit `accept` and `mark_late` remain valid alternatives.
- [x] Assessment disclosure settings remain separate and independently configurable.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule` gives score, correctness, submitted response, Question Feedback, correct answer, answer explanation, and class statistics independent timing fields; `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` exposes each field separately.
  - Evidence (test): accepted actual-component proof exercised `src/features/blueprint_course/blueprint_course_create_dialog.tsx` `BlueprintCourseCreateDialog`, switching Type both ways while preserving title, entries, and the resulting Type defaults.
- [x] **Regular Assignments** and **Bonus Assignments** should rarely show the correct answer.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule::for_assessment_type` defaults `question_answer` to `never` for Regular and Bonus while leaving the setting independently configurable.
- [x] Regular and Bonus Assignments show the **Student's** response and whether it was correct or incorrect.
  - Evidence (source): `schemas/base_schema/assessments.sql` `create_assessment` defaults `submitted_response` and `per_item_correctness` to `after_submit`; `crates/server/src/assessment_delivery/history.rs` `project_history` projects those fields independently through the existing server-redacted summary.
  - Evidence (test): accepted actual-component proof exercised `src/pages/assessment_attempt_page.tsx` `submitAttempt`, navigating an accepted whole submission to that summary while preserving existing failure behavior on the Attempt page.
- [ ] **Practice Question Assignments** show correct answers immediately after Assessment Attempt
  submission.
  - Evidence (source): the Rust, SQL, Blueprint, and curriculum-publication creation boundaries default Practice `question_answer` to `after_submit` while Question Feedback and answer explanation remain independent.
  - Evidence (runtime): accepted PostgreSQL 17 ordinary start and whole-submit proof found zero response-source rows before submission and one after; the native PLE summary preserved the disclosed correct answer.
  - Mismatch: Backend-owned answers currently project as absent, and opaque WeBWorK answer disclosure remains unimplemented. This universal row stays open pending safe backend-owned disclosure without answer extraction.
- [ ] **Quizzes** and **Exams** show correct answers after all **Students** in the Course have completed the Assessment.
  - Evidence (runtime): fresh PostgreSQL 17 release proof rejected a Quiz configured to disclose answers after submission with `quiz_exam_answer_disclosure_requires_course_completion_rule`; the same Quiz released after both answer fields were set to `never`.
  - Mismatch: Quiz and Exam completion now means Student submission or time expiry, regardless of score or correctness. Eventual release after every current Student completes remains unimplemented.
- [ ] A Quiz or Exam Attempt is complete when the **Student** submits it or its time limit expires and
  PLE submits it automatically.
  - Mismatch: Completion currently follows the generic Assessment Attempt lifecycle and does not implement the required Quiz/Exam submission-or-expiry rule.
- [ ] Assessment Attempt completion does not depend on correctness or score.
  - Mismatch: Current completion behavior does not establish the required score- and correctness-independent rule for every Assessment Attempt.
- [ ] Until then, Quizzes and Exams do not disclose correct answers.
  - Evidence (source): `schemas/base_schema/assessments.sql` fails closed when Quiz or Exam correct-answer or answer-explanation disclosure is not `never`.
  - Mismatch: This provisional hard guard prevents early disclosure but does not implement eventual release after every current Student completes.
- [x] Optional Question Feedback is shown when the Question Backend provides it.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_student_feedback` releases backend-provided feedback.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] Question Feedback does not use Assessment correct-answer disclosure settings.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `StudentFeedbackReleaseDecision` and `project_student_feedback` gate Question Feedback, correct answer, and answer explanation independently.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] Unreleasing a Course Instance Assessment permanently deletes its Student Work and returns to a pre-release state.
  - Evidence (test): `tests/e2e/e2e_unrelease_connected.sh` `psql_admin` runs the connected unrelease deletion and pre-release-state oracle.

### Assessment Attempts

- [x] An **Assessment Attempt** is one Student attempt at a Course Instance Assessment.
  - Evidence (source): `schemas/base_schema/assessment_attempts.sql` `ple_private.assessment_attempt` requires both a `student_record_id` and an `assessment_id`; that Assessment ID references the Course-owned `ple_data.assessment` aggregate.
- [x] Blueprint Assessments do not have Assessment Attempts.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` contains only reusable content and defaults; `schemas/base_schema/assessment_attempts.sql` `ple_private.assessment_attempt` permits Attempts only through `ple_data.assessment`, the Course Instance Assessment aggregate.
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
  - Evidence (source): `schemas/base_schema/grading_access.sql` `read_assessment_gradebook_evidence` independently selects the highest grading-complete submitted Attempt by earned points, then uses the latest Attempt only when no score is established; `ple_api.read_course_gradebook` consumes that private answer-free helper.
  - Evidence (runtime): accepted actual PostgreSQL 17 evidence exercised `schemas/base_schema/grading_access.sql` `ple_api.read_course_gradebook`, proving an earlier higher earned score beats a later lower score and a later unfinished or pending Attempt does not replace it. Current Question points recalculated the selected score from `8` to `16`; a Bonus contribution retained a zero possible denominator; and a latest unscored expired Attempt remained the fallback when no completed score existed.
  - Evidence (source): `schemas/base_schema/student_assessment_landing.sql` `ple_private.read_student_released_assessment_landing_evidence` keeps progress, completion, and resume state on the latest Attempt but obtains the Assessment score from the selected highest Attempt and applies that Attempt's copied disclosure timing. `src/api/decoders/live_student_course_landing.ts` `decodeAssessmentSummary` requires direct `assessmentScore` and rejects the retired `score` alias; `src/pages/student_course_landing_page.tsx` labels it `Assessment score`.
  - Evidence (runtime): accepted actual PostgreSQL 17 evidence exercised `schemas/base_schema/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments`, preserving an earlier higher score across later lower, unfinished, and pending Attempts, including inverse selected-Attempt/latest-Attempt disclosure cases. The focused decoder/presentation lane passed 8/8, and compiled M6 component evidence passed.
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
  - Evidence (source): `schemas/base_schema/assessment_attempts.sql` `issued_question_is_immutable` stores issued Question revision identity under foreign-key protection.
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
  - Evidence (source): `schemas/base_schema/grading_access.sql` `read_assessment_gradebook_evidence` independently selects the highest grading-complete submitted Attempt by earned points, then uses the latest Attempt only when no score is established; `ple_api.read_course_gradebook` consumes that private answer-free helper.
  - Evidence (runtime): accepted actual PostgreSQL 17 evidence exercised `schemas/base_schema/grading_access.sql` `ple_api.read_course_gradebook`, proving an earlier higher earned score beats a later lower score and a later unfinished or pending Attempt does not replace it. Current Question points recalculated the selected score from `8` to `16`; a Bonus contribution retained a zero possible denominator; and a latest unscored expired Attempt remained the fallback when no completed score existed.
  - Evidence (source): `schemas/base_schema/student_assessment_landing.sql` `ple_private.read_student_released_assessment_landing_evidence` keeps progress, completion, and resume state on the latest Attempt but obtains the Assessment score from the selected highest Attempt and applies that Attempt's copied disclosure timing. `src/api/decoders/live_student_course_landing.ts` `decodeAssessmentSummary` requires direct `assessmentScore` and rejects the retired `score` alias; `src/pages/student_course_landing_page.tsx` labels it `Assessment score`.
  - Evidence (runtime): accepted actual PostgreSQL 17 evidence exercised `schemas/base_schema/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments`, preserving an earlier higher score across later lower, unfinished, and pending Attempts, including inverse selected-Attempt/latest-Attempt disclosure cases. The focused decoder/presentation lane passed 8/8, and compiled M6 component evidence passed.
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
