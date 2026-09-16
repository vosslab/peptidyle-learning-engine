## Assessments

- [ ] **Assessment** is the PLE object for organizing Questions into a graded or practice activity.
  - Evidence (source): `schemas/base_schema/assessments.sql` defines the Course aggregate as `ple_data.assessment`; current Type, release, Attempt, and API boundaries use that name.
  - Mismatch: Source naming is current, but the complete graded-or-practice product behavior requires the remaining delivery and content verification below.
- [x] PLE has **Blueprint Assessments** and **Course Instance Assessments**.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` is the reusable Blueprint Assessment aggregate; `schemas/base_schema/assessments.sql` `ple_data.assessment` is the current Course Instance Assessment aggregate with a required `course_id`.
- [x] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` contains Type, title, instructions, ordered entries, and validated `BlueprintAssessmentDefaults`; `BlueprintCourseModuleContent` owns those Assessments in Blueprint Course content.
- [ ] Course Instance Assessments deliver Questions to **Students**.
  - Evidence (source): `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` requires a released Course Assessment and current Student Course record before issuing Questions.
  - Mismatch: Source establishes the delivery boundary, but complete Student delivery acceptance remains separately open.
- [x] All Assessments use the same underlying Assessment model.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` and `crates/learning-data-access/src/assessment_release.rs` `LiveAssessmentWorkspace` share canonical Assessment Type, title, instructions, activity rules, and Student feedback rules. `crates/learning-data-access/src/postgres/course_blueprint_adoption.rs` `assessment_input` materializes `SaveLiveAssessmentInput` through `assessment_values_json` into ordinary `ple_data.assessment` rows; distinct reusable and delivery storage/lifecycle projections are not separate pedagogical models.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 connected `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` passed 1 test with 0 ignored. Its `crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs` `assert_adoption_projection` verified preserved Assessment Type, mixed ordered Pool/Fixed entries, nondefault points/scoring/retry/timing/activity/feedback rules, exact Revision pins, fresh independent daughter Pool IDs, and unset delivery dates. Supplemental read-only SQL verified two adopted Regular Assignments retained the source Type and were Unreleased with null dates. Artifact: `/private/tmp/ple-shared-assessment-adoption-artifacts.IkYuXY`. This architecture receipt does not establish every Type's Student delivery or completion.
- [ ] **Assignment** is not a separate object or category. The word appears only in the names
  **Regular Assignment**, **Practice Question Assignment**, and **Bonus Assignment**.
  - Evidence (source): `schemas/base_schema/assessments.sql`, Assessment Attempt SQL, and browser APIs use `assessment` generally; the closed Type set retains Assignment only in the three specified Type names.
  - Mismatch: A complete title/reference inventory and legacy-consumer cutover verification remain open.

### Assessment content

- [x] Assessments contain an ordered sequence of Published Questions and Question Pools.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` renders the mixed entry sequence; `schemas/base_schema/assessments.sql` `assessment_entry_active_authored_position_key` enforces distinct current positions with a deferred constraint, allowing atomic swaps and retired-position reuse. `schemas/base_schema/assessment_operations.sql` `ple_api.load_assessment_workspace_rows` projects only available current entries.
  - Evidence (runtime): accepted independent actual-server/private bundled-main browser proof at `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` saved Fixed A, an imported Pool, and Fixed B, moved the top-level Pool across Fixed A, and reloaded exact ordered entry IDs and Revision pins. Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`.
- [ ] Published Questions stay references to the same Question ID and exact Revision.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.replace_assessment_entries` preserves retained fixed-entry Question IDs and Revision Numbers and requires both values for saved entries.
  - Mismatch: The Assessment authoring input still permits ID-only Question selection; require an exact Revision at that input boundary before this invariant can be verified.
- [ ] Question Pools are copied by forking when added to another Assessment.
  - Verification pending: Source-contributor audit must confirm the import path creates an Assessment-owned fork rather than a reusable Pool copy; broad runtime evidence remains pending.
- [ ] A newly forked Question Pool initially contains the same Published Question IDs and exact Revisions as its source.
  - Verification pending: Source-contributor audit must confirm the import path carries every exact source Question ID and Revision into the new fork; broad runtime evidence remains pending.
- [ ] A forked Question Pool can be changed independently without changing its source Question Pool.
  - Verification pending: Source-contributor audit must confirm independent fork revision writes and source preservation; broad runtime evidence remains pending.
- [x] Published Questions and Question Pools remain distinct even though both can occupy positions in an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` branches on `fixedQuestion` and `questionPool`; `schemas/base_schema/assessments.sql` retains separate entry-kind storage and Assessment-owned Pool identity within one authored-position sequence.
  - Evidence (runtime): accepted mixed-entry proof at `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` reloaded distinct Fixed Question and Pool entries with exact pins, then retained the retired Pool's old Revision while a reimport received a distinct fork entry ID and Pool ID. Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`. Existing connected adoption regression also passed 1 test with 0 ignored under the corrected SQL, with two supplemental Type projections retaining unchanged pins: `/private/tmp/ple-shared-assessment-adoption-artifacts.UmP416`.
- [x] **Instructors** can add, remove, and reorder Published Questions and Question Pools.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` supplies Question addition, Pool import, and mixed-entry move/remove controls; `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` reaches `schemas/base_schema/assessments.sql` `ple_data.replace_assessment_entries` through authorized Instructor persistence.
  - Evidence (runtime): accepted actual-server/private bundled-main browser proof at `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` added both kinds, reordered the Pool across a Fixed Question, removed Fixed B and then the Pool, and saved/reloaded their absence from current content. Private retired IDs and the old Pool Revision remained; source Pool JSON was unchanged, and reimport created distinct fork entry/Pool IDs. Student direct real HTTP returned 404 without current-state change; browser error arrays were empty. Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`. This does not establish Student Work history, full Live Demo, authentication/TLS, or WeBWorK delivery.
- [x] Assessment Question-order randomization is called **Randomize question order**.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` renders the exact accessible checkbox label **Randomize question order**.
  - Evidence (runtime): the earlier accepted part 04 actual-main/HTTP receipt saved and reloaded this visible checkbox through `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`; this is separate from the mixed-entry artifact. C64 owns persisted Question-order behavior. This label receipt does not claim C505's planned filename rename or whole-milestone completion.

### Assessment types

- [x] PLE defines the available Assessment Types.
  - Evidence (source): `crates/question_model/src/assessment.rs` `AssessmentType` is the canonical closed model, and `generated/api/AssessmentType.ts` `ASSESSMENT_TYPE_VALUES` carries it to the browser contract.
- [ ] Assessment Type describes the pedagogical purpose of an Assessment and provides appropriate defaults.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states all five Instructor-selected purposes; `AssessmentWorkspaceCreatePage` and `BlueprintCourseCreateDialog` render the selected description. `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment`, `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults`, `StudentFeedbackReleaseRule::for_assessment_type`, and the curriculum publisher apply Type-aware Attempt/disclosure defaults.
  - Mismatch: Purpose classification and the accepted Attempt/disclosure subsets are established, but complete appropriate-default coverage remains owned by this broad row. This receipt does not claim every Type's persisted/runtime settings or Student delivery, enforce collaboration policy, infer learning age, or supply an Instructor exam calendar.
- [x] Assessment Types are **Regular Assignment**, **Practice Question Assignment**, **Bonus Assignment**, **Quiz**, and **Exam**.
  - Evidence (source): `crates/question_model/src/assessment.rs` `AssessmentType::ALL` contains exactly the five HG values; schema constraints and `generated/api/AssessmentType.ts` `ASSESSMENT_TYPE_VALUES` use the same closed set.
- [x] **Instructors** select an Assessment Type but cannot create new Assessment Types.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_create_page.tsx` `AssessmentWorkspaceCreatePage` requires one canonical Type before creation; `src/features/blueprint_course/blueprint_course_create_dialog.tsx` `BlueprintCourseCreateDialog` does the same for reusable content, with no free-form Type path or silent default.
  - Evidence (test): `tests/test_blueprint_course_ui.mjs` `createdContent` proves the selected canonical Type reaches creation; the focused Blueprint UI/model lane passed 10/10.
- [x] Blueprint Assessments and Course Instance Assessments use the same Assessment Types.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `assessment_type` and `schemas/base_schema/assessments.sql` `assessment_type` validate the same five values; `schemas/base_schema/course_blueprint_adoption.sql` `assessment_type` copies the selected Type during adoption.
- [x] **Instructors** can change Assessment settings independently of the defaults for its Type.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `EffectiveAssessmentPolicy` resolves explicit Course Instance property overrides separately from Blueprint defaults and Assessment Type.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-Store receipt loaded an Instructor Exam, saved its policy settings, and retained its Type through `crates/learning-data-access/src/postgres/assessment_release.rs` `PostgresLiveAssessmentStore`.
- [x] Changing Assessment settings does not change its Assessment Type.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `EffectiveAssessmentPolicy` does not expose Type as an editable property, while `crates/question_model/src/assessment.rs` `AssessmentType` remains part of Assessment identity.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-Store receipt saved Instructor Exam policy settings while retaining its Type through `crates/learning-data-access/src/postgres/assessment_release.rs` `PostgresLiveAssessmentStore`.
- [x] **Regular Assignments** give **Students** regular practice applying course ideas outside class.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states practice applying course ideas outside class in the Regular description; `src/pages/assessment_workspace/assessment_workspace_create_page.tsx` `AssessmentWorkspaceCreatePage` and `src/features/blueprint_course/blueprint_course_create_dialog.tsx` `BlueprintCourseCreateDialog` render that Instructor-selected purpose at both real creation entry points.
- [x] Regular Assignments reinforce current learning and may also introduce new topics.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states both reinforcement of current learning and introduction of new topics in the Regular description; both real creation surfaces render it. The Instructor chooses content and its pedagogical classification; PLE does not infer material age.
- [x] Regular Assignments are designed as practice for learning, not merely as one-time assessments.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` identifies learning practice in the Regular description. `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment` and `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults` leave Regular Attempt limits unset, unlike Quiz/Exam's one-Attempt rule; this is a purpose/default receipt, not complete Student delivery acceptance.
- [x] **Practice Question Assignments** provide focused review or study-guide practice using material already covered.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states focused review/study-guide practice of covered material in the Practice description; `AssessmentWorkspaceCreatePage` and `BlueprintCourseCreateDialog` render that Instructor-selected purpose. PLE does not infer when material was taught.
- [x] Practice Question Assignments may be worth a small number of points or a small amount of extra credit.
  - Evidence (source): `src/pages/assessment_workspace/assessment_fixed_question_points_editor.tsx` `AssessmentFixedQuestionPointsEditor` edits nonnegative Question point values independently of Type; `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` offers `extraCredit` entry scoring. `schemas/base_schema/grading.sql` `score_recorded_credit` retains earned points while `grade_contribution_points_possible` excludes extra-credit points from the denominator for Practice as well as other Types. The Instructor chooses the amount; no arbitrary numeric definition of small is imposed.
- [ ] Practice Question Assignments use the same whole-Attempt submission boundary as every other
  Assessment and show the correct answer immediately after that Assessment Attempt is submitted.
  - Evidence (runtime): accepted PostgreSQL 17 proof through the ordinary start, whole-submission, and history APIs returned zero history response-source rows before submission and one after submission; the native PLE summary then preserved the disclosed correct answer.
  - Mismatch: Backend-owned answers currently project as absent. Opaque WeBWorK post-submit answer disclosure remains unimplemented and requires renderer/adapter work without answer extraction, so the cross-backend row remains open.
- [x] **Bonus Assignments** provide optional extra credit.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states optional extra credit in the Bonus description at both Instructor creation surfaces; `schemas/base_schema/grading.sql` `grade_contribution_points_possible` gives Bonus a zero grade denominator without discarding earned points.
  - Evidence (runtime): the separately accepted neighboring Bonus grading receipt returned and rendered `8 / 0` through `schemas/base_schema/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments` and the M6 component. Optional is the Instructor-selected purpose, not a new completion/required-work policy field.
- [x] Bonus Assignments are worth zero points possible and add earned points directly to the grade.
  - Evidence (source): `schemas/base_schema/grading.sql` `grade_contribution_points_possible` makes Bonus points possible zero while `score_recorded_credit` continues to supply earned points; `schemas/base_schema/grading_access.sql` applies that contribution through the real Gradebook helper, and `schemas/base_schema/student_assessment_landing.sql` projects the same selected contribution to the Student API.
  - Evidence (runtime): accepted actual PostgreSQL 17 proofs exercised `schemas/base_schema/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments`, returning earned Bonus points with a zero denominator; the Student row was `8 / 0`. The strict Student decoder accepts the complete pair, and compiled M6 component evidence rendered it without changing raw Assessment Attempt scoring.
- [x] **Quizzes** assess understanding of recent material.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states assessment of recent material in the Quiz description at both Instructor creation surfaces. The Instructor selects that purpose and content; PLE does not infer learning age.
- [x] Quizzes may use more restrictive Attempt and collaboration settings than Regular Assignments.
  - Evidence (source): `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment` and `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults` select one Attempt for Quiz versus unset for Regular. `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` fixes Quiz to one Attempt and supplies editable Student instructions for Instructor-stated collaboration restrictions. This capability receipt does not claim a formal collaboration-policy field or automatic collaboration enforcement.
  - Evidence (runtime): the separately accepted neighboring one-Attempt receipt covered Quiz resume/submission and Attempt-2 denial through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
- [x] **Exams** are individual assessments associated with scheduled exam periods.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states individual assessment associated with a scheduled exam period in the Exam description; `AssessmentWorkspaceCreatePage` and `BlueprintCourseCreateDialog` render it as an Instructor-selected purpose. This does not claim an Instructor exam calendar or infer scheduled periods.
- [x] Exams may use more restrictive Attempt, timing, availability, and feedback settings.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` fixes Exam to one Attempt and exposes editable time limit, Available/Closes schedule, and six feedback-release controls including `never`. This is current UI capability evidence, not acceptance of every control's persistence/runtime enforcement.
  - Evidence (runtime): the separately accepted neighboring one-Attempt receipt covered Exam effective-one handling and expired-pending denial through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`; it does not expand this settings-capability receipt.
- [x] Quizzes and Exams allow one Assessment Attempt.
  - Evidence (source): `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` resolves Quiz and Exam to an effective limit of `1` before issue or resume.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt covered Quiz resume/submission and Attempt-2 denial, Exam effective-one handling, and expired-pending Exam denial through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.

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
- [x] Blueprint Assessments do not use Assessment Templates.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` defines Templates as Instructor-owned private state outside Courses and Blueprints with no Blueprint or source field.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt covered Template create, save, read, and direct Course Assessment copy without a Blueprint relationship through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
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
- [x] Newly added Blueprint Assessments are automatically copied to daughter Course Instances as unreleased Course Instance Assessments.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` materializes only newly added Assessments through `ple_data.append_course_assessments`; PostgreSQL Blueprint Store Save invokes the transaction boundary.
  - Evidence (test): connected `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` passed exact settings/Revision-pin and distinct daughter Pool-ID checks, inactive daughter handling, unrelated empty-Course nonmutation, actual existing Student Work preservation, unchanged adoption pin, and replay/no-op/stale refusal without duplicate copies. Accepted temporary malformed-payload rollback is supplemental: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. Existing connected adoption lifecycle regression passed 1 test with 0 ignored: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Decision: This closes only automatic-new copying, not existing-Assessment update offers, full Student delivery, or the whole Assessment milestone.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` was extended to invoke the `append.rs` helper above; its final integrated connected receipt and malformed-daughter rollback both passed in `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. This is extension of an existing permanent test, not a separate new test.

### Assessment Templates

- [x] An **Assessment Template** is a reusable set of settings for creating Course Instance Assessments.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` stores reusable settings, and `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template` makes a direct Course Assessment from them.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed the complete Template round trip and by-value Course Assessment copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] Assessment Templates are separate from Assessment Types.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` has a private UUID, owner, name, and settings while its required `assessment_type` is one closed Type field.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template create, save, read, and by-value copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] Every Assessment Template has one of the five Assessment Types.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` constrains `assessment_type` to the five canonical values.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template create, save, read, and by-value copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] **Instructors** can create and change their own Assessment Templates.
  - Evidence (source): `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface` supplies Instructor CRUD; owner authorization is enforced by the Template Store.
  - Evidence (runtime): accepted C515 actual-Store proof covered owner/nonowner and inactive authorization, stale CAS, and settings round trip through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
  - Evidence (runtime): separate actual-component proof covered browser Template CRUD at `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface`; it is not connected-server evidence.
- [x] Assessment Templates provide defaults for settings such as Attempts, timing, scoring, and disclosure.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` stores instructions, Attempt/time limits, late-work, seven activity rules, and six feedback-release timings including grade rule.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template settings round trip and copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] Creating a Course Instance Assessment from a Template copies its settings into the new Assessment.
  - Evidence (source): `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template` reads the owner-visible Template once and passes every portable setting by value to `ple_data.create_assessment_from_template_values`.
  - Evidence (runtime): accepted C516 SQL full-settings/copy-independence and actual-component UI proofs passed at `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template`.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] The new Course Instance Assessment can be changed independently after it is created.
  - Evidence (source): `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template` creates a direct Course Assessment with copied values and no Template link.
  - Evidence (runtime): accepted C516 SQL full-settings/copy-independence proof passed at `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template`.
- [x] Changing an Assessment Template does not change Assessments previously created from it.
  - Evidence (source): `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template` copies settings by value into the new Course Assessment and stores no Template identity.
  - Evidence (runtime): accepted C516 SQL full-settings/copy-independence proof passed at `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template`.
- [x] Assessment Templates do not contain Questions or Question Pools.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` defines identity, owner, Type, and reusable settings with no Question, Pool, content, point, or source field.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template creation and empty direct Course Assessment copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] Blueprint Assessments do not use Assessment Templates.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` defines Templates outside Courses and Blueprints; the by-value path creates only direct Course Assessments.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template copy to a direct Course Assessment without a Blueprint relationship through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
  - Owner: Same implementation finding as the earlier Assessment Templates bullet.

### Course Instance Assessment release and defaults

- [x] Course Instance Assessments start unreleased.
  - Evidence (source): `schemas/base_schema/assessments.sql` `assessment_status` defaults to `unreleased`; `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment` creates direct rows without overriding it, `schemas/base_schema/course_blueprint_adoption.sql` `ple_data.initialize_course_assessments` owns the same initial status for adopted rows, and `schemas/base_schema/assessment_template_copy.sql` `create_assessment_from_template_values` flows through direct `create_assessment` before copying policy values.
  - Evidence (runtime): `src/pages/assessment_workspace/assessment_workspace_create_page.tsx` `AssessmentWorkspaceCreatePage` was exercised in accepted private actual-main/HTTP proof: a newly Empty Course's direct Practice Assessment was Unreleased at creation, before its later validated release. Separate accepted `src/pages/course_list_page.tsx` `TeachingCourseListPage` Public Blueprint adoption produced an Unreleased Practice Assessment at creation. This initial-state fact does not establish Student delivery or all release rules.
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
- [x] Release Validation should check required settings such as point values, Attempt limits, and time limits for valid ranges.
  - Evidence (source): `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` reaches `schemas/base_schema/assessments.sql` `ple_data.replace_assessment_entries`, which rejects point values outside `0` through `1000000000.9999` or four decimal places; table checks retain that bound for alternate writers. The Assessment table permits only null or positive whole-Assessment Attempt/time limits and requires one Attempt for Quiz and Exam.
  - Evidence (source): `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues` separately requires an Assessment Attempt time limit before release.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-API receipt for `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` atomically rejected `1000000001` and `1000000000.99999`; exact `1000000000.9999` saved and released.
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
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule` gives score, correctness, submitted response, correct answer, answer explanation, and class statistics independent timing fields; `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` exposes those six fields. Question Feedback is shown when provided and has no delayed-release state.
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
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `ple_private.current_student_cohort_completed_assessment` derives the current active Student cohort from immutable submission evidence without a snapshot or latch. `crates/server/src/assessment_delivery/history.rs` `history_decision` calls `gate_quiz_exam_answers_for_current_cohort`, and `project_released_content` applies that gate.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed the two-current-Student cohort transition through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
  - Evidence (runtime): accepted independent PostgreSQL 17 installed-predicate proof with administrator-inserted synthetic fixtures verified never-started blocking, pending-invitation exclusion, joined-current-membership blocking, Account-deactivation membership preservation, Course-end noncompletion, ended-episode exit/new-episode rejoin, and retained submission behavior: `/private/tmp/ple-assessment-cohort-transition-artifacts.nWdHzT`.
  - Mismatch: Opaque WeBWorK answer display remains unimplemented.
  - Verification pending: Connected HTTP correct-answer display/release must verify that the current gate controls the projection.
- [x] A Quiz or Exam Attempt is complete when the **Student** submits it or its time limit expires and
  PLE submits it automatically.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` inserts one submitted-Attempt record for `student` or `deadline` finalization.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt covered Quiz Student submission, generic deadline finalization, and expired-pending Exam denial through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
  - Decision: The accepted composition uses the type-independent submission authority. Quiz/Exam worker finalization was not directly run.
- [x] Assessment Attempt completion does not depend on correctness or score.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` resolves finalization from Student-versus-deadline state; correctness and score are not completion conditions.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed zero/partial whole submission and deadline finalization through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
- [ ] Until then, Quizzes and Exams do not disclose correct answers.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `history_decision` calls `gate_quiz_exam_answers_for_current_cohort`, and `project_released_content` applies the resulting decision before projecting released content.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed the two-current-Student cohort transition through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
  - Evidence (runtime): accepted independent PostgreSQL 17 installed-predicate proof with administrator-inserted synthetic fixtures verified never-started blocking, pending-invitation exclusion, joined-current-membership blocking, Account-deactivation membership preservation, Course-end noncompletion, ended-episode exit/new-episode rejoin, and retained submission behavior: `/private/tmp/ple-assessment-cohort-transition-artifacts.nWdHzT`.
  - Mismatch: Opaque WeBWorK answer display remains unimplemented.
  - Verification pending: Connected HTTP answer-withholding must verify the current gate; the SQL proof is not whole-submit-pipeline acceptance.
- [x] Optional Question Feedback is shown when the Question Backend provides it.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_student_feedback` releases backend-provided feedback.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `withheld_question_answer_is_absent_while_provided_feedback_is_shown` verifies provided native feedback without answer disclosure.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] Question Feedback does not use Assessment correct-answer disclosure settings.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule` has no Question Feedback timing field; `crates/domain/src/student_feedback_release.rs` `project_student_feedback` projects supplied feedback independently while `StudentFeedbackReleaseDecision` continues to gate correct answer and explanation.
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
- [x] **Instructors** control the number of permitted Assessment Attempts.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.save_assessment` and `ple_data.save_assessment_policies` accept `assessment_attempt_limit` only after `current_session_account_is_course_instructor`; issuance applies that saved value subject to Quiz/Exam effective-one.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt covered unlimited retries, Quiz Attempt-2 denial, and expired-unlimited new-Attempt behavior through `crates/learning-data-access/src/postgres/assessment_attempt.rs` `PostgresAssessmentAttemptStore`.
- [x] Regular Assignments default to unlimited Attempts.
  - Evidence (source): `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment` defaults to a nullable Attempt limit for unlimited Attempts; only Quiz and Exam override it to one at issuance.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed unlimited retries after perfect and nonperfect submissions through `crates/learning-data-access/src/postgres/assessment_attempt.rs` `PostgresAssessmentAttemptStore`.
- [x] **Students** may repeat an Assessment as often as its settings allow, including practicing toward a perfect score.
  - Evidence (source): `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` counts issued Attempts only with an effective finite limit; `NULL` permits another.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed unlimited perfect/nonperfect retries and expired-unlimited new Attempt through `crates/learning-data-access/src/postgres/assessment_attempt.rs` `PostgresAssessmentAttemptStore`. A separate isolated actual-server native Practice proof published the checked-in PKU Question through Draft authoring, enrolled the Student through roster import/claim, saved and submitted its correct response for a disclosed 1/1 score, then issued a distinct second Assessment Attempt despite that perfect score. This proves this one unlimited Practice transport case, not the Regular Assignment default or a complete Student browser journey.
- [x] When an Assessment permits multiple Attempts, the highest Assessment Attempt score is used as the
  Student's Assessment score.
  - Evidence (source): `schemas/base_schema/grading_access.sql` `read_assessment_gradebook_evidence` independently selects the highest grading-complete submitted Attempt by earned points, then uses the latest Attempt only when no score is established; `ple_api.read_course_gradebook` consumes that private answer-free helper.
  - Evidence (runtime): accepted actual PostgreSQL 17 evidence exercised `schemas/base_schema/grading_access.sql` `ple_api.read_course_gradebook`, proving an earlier higher earned score beats a later lower score and a later unfinished or pending Attempt does not replace it. Current Question points recalculated the selected score from `8` to `16`; a Bonus contribution retained a zero possible denominator; and a latest unscored expired Attempt remained the fallback when no completed score existed.
  - Evidence (source): `schemas/base_schema/student_assessment_landing.sql` `ple_private.read_student_released_assessment_landing_evidence` keeps progress, completion, and resume state on the latest Attempt but obtains the Assessment score from the selected highest Attempt and applies that Attempt's copied disclosure timing. `src/api/decoders/live_student_course_landing.ts` `decodeAssessmentSummary` requires direct `assessmentScore` and rejects the retired `score` alias; `src/pages/student_course_landing_page.tsx` labels it `Assessment score`.
  - Evidence (runtime): accepted actual PostgreSQL 17 evidence exercised `schemas/base_schema/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments`, preserving an earlier higher score across later lower, unfinished, and pending Attempts, including inverse selected-Attempt/latest-Attempt disclosure cases. The focused decoder/presentation lane passed 8/8, and compiled M6 component evidence passed.
- [ ] Assessment Attempt submission and grading are fully automatic and require no **Instructor** action.
  - Mismatch: `schemas/base_schema/assessment_attempt_finalization.sql` has a current automatic finalization path, but no current runtime receipt establishes the complete submission-and-grading row after the retired oracle was removed.
- [ ] Automatic grading does not require a separate Student or **Instructor** grading workflow.
  - Mismatch: The current finalization source records direct grading, but no current runtime receipt establishes the complete no-workflow behavior after the retired oracle was removed.

### Assessment responses and submission

- [x] The Student submission action submits the whole Assessment Attempt.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` creates one Assessment submission and finalizes every open issued Question in that Attempt.
  - Evidence (runtime): accepted C525 actual-Store evidence exercised whole zero- and partial-response submissions through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
- [ ] A Question either has a complete saved response or has no saved response.
  - Mismatch: Complete-response contract was not verified.
- [x] PLE saves complete Question responses as the **Student** works.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` saves an ordinary Student response and asserts its persisted `saved` state.
- [ ] The Student may change a saved response while the Assessment Attempt remains open.
  - Mismatch: Current response persistence source exists, but no current runtime receipt establishes saved-response replacement after the retired oracle was removed.
- [x] Submitting the Assessment Attempt finalizes all saved Question responses together as Student Work.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` verifies every saved response before inserting the Attempt submission and Question submissions.
  - Evidence (runtime): accepted C525 actual-Store evidence exercised whole partial-response submission and history through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
- [ ] Questions without a saved response remain visibly unanswered when the Attempt is submitted.
  - Mismatch: Visible unanswered-state behavior needs runtime evidence.
- [x] An unanswered Question receives zero credit and counts as incorrect without being sent to the
  Question Backend.
  - Evidence (source): `schemas/base_schema/grading.sql` `ple_private.score_recorded_credit` treats null retained credit as unanswered zero earned points while retaining current points possible; evaluated zero credit remains a distinct grading outcome.
  - Evidence (runtime): `schemas/base_schema/assessment_attempt_operations_api.sql` `ple_api.prepare_student_assessment_attempt_finalization` was invoked by `/private/tmp/ple-unanswered-connected-proof/run.sh --isolated` running the non-versioned temporary fixture `/private/tmp/ple-unanswered-connected-proof/proof.sql` against fresh PostgreSQL 17; its ordinary prepare/commit, history, and Gradebook calls observed no unanswered submission/result, one evaluated-zero result/receipt, incorrect history, `0 / 8` versus `8 / 8`, then `0 / 13` versus `13 / 13`; artifact `/private/tmp/ple-unanswered-connected-artifacts.vUexkI/proof.log` (exit 0). This is not a permanent test and does not exercise Attempt expiry.
  - Owner: Same implementation finding as the later unanswered Assessment scoring bullet.
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
- [ ] When an Attempt expires, PLE submits the whole Attempt, finalizing its saved responses. Other
  Questions remain visibly unanswered, receive zero credit, and count as incorrect without being
  sent to the Question Backend.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_expired_student_assessment_attempt_finalization` delegates deadline whole-Attempt finalization to `ple_private.commit_assessment_attempt_finalization`, which closes missing responses as `closed_unanswered`.
  - Verification pending: `/private/tmp/ple-unanswered-connected-proof/run.sh --isolated` ran only ordinary `ple_api.prepare_student_assessment_attempt_finalization` and `ple_api.commit_student_assessment_attempt_finalization` from its temporary SQL fixture; it did not invoke `ple_api.commit_expired_student_assessment_attempt_finalization` or otherwise exercise Attempt expiry.

### Student Work

- [x] Student Work keeps the exact Published Question Revision delivered to the **Student**.
  - Evidence (source): `schemas/base_schema/assessment_attempts.sql` `issued_question_is_immutable` stores issued Question revision identity under foreign-key protection.
- [ ] For a Question Pool, Student Work keeps the exact Question Pool Revision and Published Question Revision selected.
  - Mismatch: Pool selection retention is not verified.
- [x] Student Work keeps each saved response as finalized with the submitted Attempt and the grading outcome returned by the Question Backend.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` checks every saved-response snapshot, inserts the submitted Attempt and exact saved responses in one transaction, and supplies each evaluated normalized credit to `schemas/base_schema/grading.sql` `ple_private.record_direct_automated_grading_result` for retained grading evidence.
  - Decision: This is current persistence-source evidence, not a fresh connected/backend receipt. The retired SQL oracle and legacy Assignment transport shell are not current runtime proof.
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
  - Evidence (source): `schemas/base_schema/grading.sql` `ple_private.record_direct_automated_grading_result` inserts the supplied credit fraction into `ple_private.grading_result.normalized_credit`, constrained to the inclusive unit interval and retained under `grading_result_is_immutable`.
  - Evidence (runtime): `schemas/base_schema/grading.sql` `ple_private.record_direct_automated_grading_result` is invoked by the accepted private PostgreSQL 17 production-SQL lifecycle fixture at `/private/tmp/ple-current-rescore-proof/proof.sql`; its artifact `/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` (exit 0) retained the submitted `0.5` fraction while rescoring current points from `8` to `13`. This SQL-only fixture simulates the initial synchronous Backend credit and does not establish Backend transport or HTTP.
- [x] Course Instance Assessment scores are calculated from stored credit fractions and current Question point values.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` joins retained grading credit to the current Assessment entry point value and calls `schemas/base_schema/grading.sql` `ple_private.score_recorded_credit`; current scoring treatment remains an explicit input.
  - Evidence (runtime): `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` is invoked by the accepted private PostgreSQL 17 production-SQL lifecycle fixture at `/private/tmp/ple-current-rescore-proof/proof.sql`; its artifact `/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` (exit 0) first recorded `0.5` as `4 / 8`, then authorized an expected-current Instructor save to points `13` and observed `6.5 / 13` for the answered Question, `0 / 13` unanswered, and `6.5 / 26` through replay, history, Student landing, and Instructor Gradebook reads. This does not establish HTTP, rendering, or Backend transport.
- [x] An unanswered Question contributes zero points to the Assessment score and counts as incorrect.
  - Evidence (source): `schemas/base_schema/grading.sql` `ple_private.score_recorded_credit` distinguishes null retained credit from an evaluated zero-credit result, returning zero earned points for unanswered work even under `full_credit` while retaining the current denominator.
  - Evidence (runtime): `schemas/base_schema/assessment_attempt_operations_api.sql` `ple_api.prepare_student_assessment_attempt_finalization` was invoked by `/private/tmp/ple-unanswered-connected-proof/run.sh --isolated` running the non-versioned temporary fixture `/private/tmp/ple-unanswered-connected-proof/proof.sql` against fresh PostgreSQL 17; its ordinary prepare/commit calls verified `0 / 8` unanswered versus `8 / 8` evaluated zero, no unanswered submission/result, one evaluated result/receipt and Assessment submission across both replays, incorrect history, `0 / 13`, `13 / 13`, and Gradebook `13 / 26`; artifact `/private/tmp/ple-unanswered-connected-artifacts.vUexkI/proof.log` (exit 0). This is not a permanent test and does not exercise Attempt expiry.
- [x] When an Assessment has multiple submitted Attempts, the highest Assessment Attempt score is the
  Student's Assessment score.
  - Evidence (source): `schemas/base_schema/grading_access.sql` `read_assessment_gradebook_evidence` selects the highest grading-complete submitted Attempt by earned points; `schemas/base_schema/student_assessment_landing.sql` consumes that same helper while retaining latest-Attempt progress separately. The retired configurable grade-rule enum, field, SQL columns, and editor choices are absent from the production model and UI.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 lifecycle proof exercised `schemas/base_schema/grading_access.sql` `ple_api.read_course_gradebook` and the Student landing helper across four submitted or issued Attempts with individual scores `12`, `4`, `16`, and `NULL`. Instructor Gradebook and Student landing both projected `12 / 16`, `12 / 16`, and `16 / 16`; the fourth in-progress Attempt did not replace the selected `16 / 16` score, while Student landing continued to show latest-Attempt state. Artifact: `/private/tmp/ple-highest-score-proof/artifacts.rQxVs8/proof.log` (exit 0). This privileged fixture and simulated backend proof does not establish HTTP, rendering, actual backend grading, or unlimited-Attempt eligibility.
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
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.prepare_assessment_attempt_finalization` reads an already-submitted Attempt by joining immutable credit to current Assessment entry points; `schemas/base_schema/grading.sql` `ple_private.score_recorded_credit` applies those current points at read time.
  - Evidence (runtime): `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` is invoked by the accepted private PostgreSQL 17 production-SQL fixture at `/private/tmp/ple-current-rescore-proof/proof.sql`; its artifact `/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` (exit 0) changed both current entry values from `8` to `13` and advanced the expected-current edit number, then observed `0.5` rescored from `4 / 8` to `6.5 / 13` and the Assessment from `4 / 16` to `6.5 / 26` across three replays and the history, landing, and Gradebook projections. This does not establish HTTP or rendering.
- [ ] Score recalculation does not require another Question Backend interaction.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.prepare_assessment_attempt_finalization` returns `already_submitted` with the current score before source resolution or backend-evaluation preparation; `schemas/base_schema/grading.sql` `ple_private.score_recorded_credit` is a pure SQL calculation over retained credit and current points.
  - Evidence (source): `crates/server/src/assessment_delivery/submission.rs` branches `StudentAssessmentAttemptFinalizationPreparationOutcome::AlreadySubmitted { score }` directly to `Submitted { score }` before the ready-path Backend work.
  - Evidence (runtime): the accepted private PostgreSQL 17 fixture at `/private/tmp/ple-current-rescore-proof/proof.sql` observed `already_submitted` with null question-attempt, Backend, source-object, and response work fields during each rescore replay; artifact `/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` (exit 0). This proves the SQL state and code branch, not actual HTTP request counts or Backend transport, so this row remains open.
  - Mismatch: No actual HTTP request count or Backend-transport observation confirms the SQL and server control-flow evidence.
- [x] Score recalculation does not change the stored Question grading outcome.
  - Evidence (source): `schemas/base_schema/grading.sql` `ple_private.score_recorded_credit` only calculates a score and performs no writes; `grading_result_is_immutable` rejects updates through `ple_private.reject_grading_evidence_change`. `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.prepare_assessment_attempt_finalization` reads existing grading outcomes without replacing them.
  - Evidence (runtime): `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` is invoked by the accepted private PostgreSQL 17 fixture at `/private/tmp/ple-current-rescore-proof/proof.sql`; its artifact `/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` (exit 0) retained the `0.5` fraction and matched before/after JSON hashes for every `grading_result`, `question_submission`, `assessment_submission`, and `automated_grading_receipt` row after the authorized points edit and replay. This does not establish HTTP, rendering, or Backend transport.
