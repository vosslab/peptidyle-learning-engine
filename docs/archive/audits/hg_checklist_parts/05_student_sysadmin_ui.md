### Student interface
- [ ] Guidance about the student interface
  - Reason: product decision still unclear
  - Question: Does this bullet require a Student guidance surface, or does it only introduce the Student interface rules that follow?
  - Mismatch: the bullet names no Student-facing behavior. One reading treats it as the heading for the following Student rules; the other would require a guidance surface Human Guidance does not describe.

#### General Student interface
- [x] The Student interface should focus on current Courses, Coursework, and work that needs attention.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` lists current courses; `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` lists assigned work.

- [x] **Coursework** is the Student-facing collective term for Weekly Assignments, Unit Review Assignments, Bonus Assignments, Quizzes, and Exams.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` names those five Types, and `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` labels the Student collection Coursework.

- [x] Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `assessmentContent` puts `assessmentTypePresentation` on each item and in its open action.

- [x] The Student interface should make the next useful action easy to find.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` puts Open Course on each current Course.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentList` puts the typed Coursework action on each item.

- [x] Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.
  - Evidence (source): `src/components/page_frame.css` `width: min(100%, var(--ple-reading-max-inline))` keeps Student page content within the viewport width.
  - Evidence (source): `src/components/record_list/record_list.css` `overflow-wrap: anywhere` keeps Course and Coursework rows inside the list width.
  - Evidence (test): `tests/playwright/student_course_entry_m6_evidence.mjs` `Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.` rendered the shipped Your courses list, All Coursework page, and Course landing in headless Chromium at the laptop, portrait-tablet, narrow-phone, and square viewports, kept Open Course and Resume Weekly Assignment inside each width, and hit those actions. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Student layouts should adapt smoothly at intermediate widths, with readable long titles and controls that wrap or rearrange in the task's reading order.
  - Evidence (source): `src/components/page_frame.css` `overflow-wrap: anywhere` keeps a long page title inside the content width.
  - Evidence (source): `src/components/student_assessment_presentation.css` `.student-assessment-action-region` places the primary action before the secondary action and stacks that region at max-width 56rem.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `Student layouts should adapt smoothly at intermediate widths, with readable long titles and controls that wrap or rearrange in the task's reading order.` rendered the shipped Coursework landing in headless Chromium, kept a long title inside the viewport at 1280 and at 800, kept the primary action left of the secondary action at 1280, and stacked the secondary action under the primary action at 800. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Every Student browser action should be usable with the keyboard alone.
  - Evidence (source): `src/components/record_list/record_list.tsx` `RecordActionControl` renders each Course and Coursework action as a link or a button.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `Your Courses` is a link back to the Student course list.
  - Evidence (test): `tests/playwright/student_course_entry_m6_evidence.mjs` `Every Student browser action should be usable with the keyboard alone.` reached Resume Weekly Assignment, Review Bonus Assignment, Start Bonus Assignment, Open Quiz, Your Courses, and Open Course by Tab only, activated each with Enter, and rejected any focused control that was not a native link, button, or form control. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Student pages should use names meaningful to Students.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` uses "Your courses" and "Open assigned work".

- [x] Student navigation and pages should contain only Student interfaces and capabilities.
  - Evidence (source): `src/route_access_boundary.tsx` `withRouteAccessBoundary` admits an authenticated session only through `userRoleMayAccessRoute` and otherwise says the page is not available to this account.
  - Evidence (source): `src/routes.ts` `appRoutes` wraps every declared product route in that boundary.
  - Evidence (source): `crates/server/src/live_student_course_landing.rs` `student_profile_role_is_allowed` admits only the Student role before Student Course reads.
  - Evidence (source): `crates/server/src/assessment_delivery.rs` `student_with_sessions` returns a session hash only when the authenticated role is Student.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student navigation and pages should contain only Student interfaces and capabilities.` denied Instructor and Sysadmin access to every Student route, kept Student Ribbon links on those routes or Profile, withheld a Student Profile image upload, and bound each Student route to its Student page.

- [x] Student content entry should use the response controls provided by Questions and other Student activities.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` passes the current Question presentation's response format to `QuestionPresentationResponseControl`.

- [x] Students should have no upload capabilities. Instructor-created content should use text boxes.
  - Evidence (source): `src/features/profile_avatar/profile_avatar_role.ts` `profileRoleMayManageImage` shows a Profile image upload only to an Instructor or Sysadmin.
  - Evidence (source): `crates/server/src/profile_avatar.rs` `replace_profile_image` refuses every other role before reading an image.
  - Evidence (source): `crates/server/src/draft_question_images.rs` `instructor_session_hash` admits only an Instructor session before a Draft Question image upload.
  - Evidence (source): `crates/server/src/course_appearance.rs` `instructor_session_hash` admits only an Instructor session before a Course Banner upload.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_workspace.tsx` `Student-facing prompt` is a text box for Instructor-created Question content.
  - Evidence (source): `src/pages/course_roster_page.tsx` `importRoster` records Course roster rows from a text box.
  - Evidence (test): `tests/test_ribbon_route_contract.mjs` `Students should have no upload capabilities. Instructor-created content should use text boxes` found file inputs only for the Course Banner, Profile image, and HOTSPOT image, refused those Instructor routes and the Profile image control for a Student, and kept the Question prompt and roster import as text boxes.
  - Owner: 03_shell.md / General interface design (first occurrence; identical requirement and status).

#### Student Ribbon interface
- [x] The Student Ribbon should use familiar Student language rather than internal PLE terms such as Assessment.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` labels Student navigation Coursework, Grades, and Courses.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Ribbon labels use Coursework language`.

- [x] Student Tier 1 navigation uses **Coursework**, **Grades**, and **Courses**, in that order.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_ONE` declares that Student order.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Coursework keeps its collective Tier 1 label without an Attempt-only row`.

- [x] **Coursework** and **Grades** show relevant information across all of the Student's enrolled Courses. When a Student has more than one Course, records clearly identify their Course.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentAllCourseworkPage` and `src/pages/student_course_grades_page.tsx` `StudentScoresPage` group every enrolled Course by short name and long name.

- [x] **Coursework** and **Grades** each have a stable Tier 2 set. Opening Coursework, an Attempt, a review, or another item does not change the Tier 2 choices or their order.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_TWO` fixes the Student rows independently of the route.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Tier 2 choices and order stay fixed across routes and Course context`.

- [x] I accept grayed-out Tier 2 choices; keep each in place instead of hiding or removing it, then dynamically inserting or restoring it.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `UnavailableRibbonChoice` stays in `isRequiredTaskArea` rows, including Student Coursework and Grades.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `the fixed shortcut stays disabled when no resumable Attempt is reported`.

- [x] Student Tier 2 must include the choices below. Additional choices may be added when they provide a useful Student navigation destination.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_TWO` lists the Student Coursework, Grades, and Courses choices.

  - [x] **Coursework**
    - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` labels the Student tab Coursework.

    - [x] **All Coursework**: Shows all Coursework across the Student's enrolled Courses.
      - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentAllCourseworkPage`.

    - [x] **Due Soon**: Shows Coursework across the Student's enrolled Courses that is approaching its due date.
      - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentDueSoonPage`.
      - Evidence (test): `tests/test_student_coursework_presentation.mjs` `isInStudentDueSoonWindow`.

    - [x] **Completed**: Shows Coursework across the Student's enrolled Courses for which the Student has submitted at least one Attempt. Completed does not mean that the Student earned a perfect score.
      - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentCompletedPage`.
      - Evidence (test): `tests/test_student_coursework_presentation.mjs` `hasSubmittedStudentAttempt`.

    - [x] **Active Attempt**: I want a quick return to an Assessment with a running clock. Keep it visible but disabled when no Attempt's clock is running.
      - Evidence (source): `src/ribbon/ribbon_contract.ts` `deriveRibbonModel`.
      - Evidence (test): `tests/test_ribbon_contract.mjs` `Active Attempt links directly to the server-selected resumable Attempt`.

  - [x] **Grades**
    - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` labels the Student tab Grades.

    - [x] **Scores**: Shows the Student's released Coursework scores across their enrolled Courses.
      - Evidence (source): `src/pages/student_course_grades_page.tsx` `StudentScoresPage`.

    - [x] **Response Stats**: Shows statistics about the Student's responses across Coursework, subject to Student-visible score and per-Question feedback rules.
      - Evidence (source): `src/pages/student_course_response_stats_page.tsx` `ResponseStatsRecord`.
      - Evidence (test): `crates/browser-api-contract/src/student_course_response_stats.rs` `contract_contains_only_disclosed_counts_and_exact_question_revision`.

    - [x] **Attempt History**: Shows the Student's previous Assessment Attempts across their enrolled Courses and provides access to review them when permitted.
      - Evidence (source): `src/pages/student_course_attempt_history_page.tsx` `StudentAttemptHistoryPage`.

    - [x] **Latest Feedback**: Provides quick access to the most recent feedback available to the Student across their enrolled Courses. It remains visible but disabled when no feedback is available.
      - Evidence (source): `src/ribbon/ribbon_contract.ts` `studentLatestFeedback`.
      - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Tier 2 choices and order stay fixed across routes and Course context`.

  - [x] **Courses**
    - Evidence (source): `src/ribbon/ribbon_schema.ts` `CURRENT_STUDENT_COURSES`.

    - [x] Tier 2 shows the short names of the Student's currently enrolled Courses.
      - Evidence (source): `src/ribbon/ribbon_contract.ts` `taskAreasFor`.

    - [x] Selecting a Course opens that Course.
      - Evidence (source): `src/ribbon/ribbon_contract.ts` `studentCourseLanding`.

    - [x] The current Course is visually identified when the Student is viewing Course-specific content.
      - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Coursework keeps its collective Tier 1 label without an Attempt-only row`.

    - [x] The Course list changes when the Student's Course enrollment changes; navigating within a Course does not change the list or its order.
      - Evidence (source): `src/ribbon/ribbon_contract.ts` `taskAreasFor` keeps the supplied `studentCourses` order.

- [x] On narrow screens, use a compact navigation arrangement that keeps the product identity, current location, navigation controls, and Profile readable and reachable.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `AppRibbon` renders the P mark, selected location, Tier 1 controls, and Profile.
  - Evidence (source): `src/ribbon/app_ribbon.css` `@media (max-width: 40rem)` compacts that navigation on a narrow screen.

#### Student Course and Coursework interface
- [x] I use Coursework and Grades across all my enrolled Courses; opening a Course from Courses does not pin or filter those global views.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentAllCourseworkPage` and `src/pages/student_course_grades_page.tsx` `StudentScoresPage` load every enrolled Course.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `global Student Tier 2 destinations stay available without Course context`.

- [x] When I view Course-specific content, the Ribbon, breadcrumb, PageFrame, and relevant records should make the Course clear.
  - Evidence (source): `src/ribbon/ribbon_contract.ts` `courseBreadcrumbItem` and `src/pages/student_course_landing_page.tsx` `StudentCourseworkCourseSection`.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `an Attempt breadcrumb identifies its explicit Course context`.

- [x] Students should be able to see their active Courses and Coursework from the main navigation.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` provides the current-Course index; `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` provides its work.

- [x] Course invitations should show the Course name and relevant Instructor and term information before the Student accepts the invitation.
  - Evidence (source): `src/pages/student_course_invitation_page.tsx` `StudentCourseInvitationPage` shows the Course name, Instructor, and term before Accept invitation.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_pending_student_course_invitations` returns only that Student's pending Course, Instructor, and term.

- [x] Course pages should make upcoming, available, completed, and missed Coursework easy to distinguish.
  - Evidence (source): `src/pages/student_coursework_presentation.ts` `studentCourseworkDisplay` and `src/pages/student_course_landing_page.tsx` `assessmentContent`.
  - Evidence (test): `tests/test_student_coursework_presentation.mjs` `Coursework status labels distinguish upcoming, available, completed, and missed work`.

- [x] Coursework lists should make due dates, Type, and completion status easy to scan.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `assessmentContent`.

- [x] Progress should keep Coursework visible when it has Attempts but no released score, clearly marked **Score not released**.
  - Evidence (source): `src/pages/student_course_progress_page.tsx` `StudentCourseProgressPage` and `src/student_course_progress_presentation.ts` `studentAssessmentScoreStateLabel`.

- [x] Submitted work should remain distinct from a perfect score; **Completed** means at least one submitted Attempt.
  - Evidence (source): `src/student_course_progress_presentation.ts` `completedStudentAssessmentCount` and `studentAssessmentScoreStateLabel`.
  - Evidence (test): `tests/test_student_coursework_presentation.mjs` `hasSubmittedStudentAttempt`.

- [x] **Response Stats** should show actual Student response outcomes across Assessment Types, subject to Student-visible score and per-Question feedback rules.
  - Evidence (source): `src/pages/student_course_response_stats_page.tsx` `ResponseStatsRecord`.
  - Evidence (test): `crates/browser-api-contract/src/student_course_response_stats.rs` `contract_contains_only_disclosed_counts_and_exact_question_revision`.

- [x] Measured Question display time should be labeled as approximate **time shown with the Question**, with its sample count. It does not measure attention or effort and does not affect grades.
  - Evidence (source): `src/pages/student_course_response_stats_page.tsx` `durationLabel`.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_operations.sql` `checkpoint_student_question_display_duration`.

- [x] Keep Coursework entries compact in height so Students can scan several items at once.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentList` renders each Coursework item through the shared record row.
  - Evidence (source): `src/components/record_list/record_list.css` `.record-list__row--semantic` keeps that row a short bordered list entry.

- [x] Keep essential Coursework information and the main action visible, with fuller access and timing details available through progressive disclosure.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` keeps the start action next to `StudentAssessmentStartFacts`.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `StudentAssessmentDecisionDetails` places Available, Closes, and Late work in `student-assessment-decision__details`.
  - Evidence (source): `src/components/student_assessment_presentation.css` `.student-assessment-decision__details:not([open]) > :not(summary)` hides that fuller timing until the Student opens it.

- N/A Coursework lists may provide filters for **Weekly Assignments**, **Unit Review Assignments**, **Bonus Assignments**, **Quizzes**, and **Exams**.
  - Reason: Human Guidance permits these Type filters and does not require a Coursework list to offer them.

- [x] Each Coursework item should clearly show its Assessment Type using its label and Type icon.
  - Evidence (source): `src/components/record_list/record_list.tsx` `assessmentTypePresentation` renders the Type icon and label together.

- [x] Before starting Coursework, Students should see its title, Type, Question count, points possible, time limit, and previous Attempts.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` presents the title, specific Type, Question count, points possible, time limit, and previous Attempts before start.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `StudentAssessmentStartFacts` owns the compact Question, points, and time-limit facts.
  - Evidence (runtime): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` was exercised by accepted isolated actual-server/exact-main Student proof `/private/tmp/ple-course-empty-artifacts.JCFLm9` after a real roster import/claim. It opened a Released direct Practice Assessment before Start and showed title, Practice Type, one Question, one point, the exact one-hour limit, and an explicit zero-previous-Attempt state; an Unreleased sibling was omitted and an outsider received 404. A separate accepted native Student HTTP/browser run `/private/tmp/ple-course-empty-artifacts.ONrLSK` whole-submitted a real graded 1/1 Attempt, then reopened the overview before starting another. The same six facts included an actual "Previous attempts" Attempt 1 Submitted link; its clicked history showed recorded PKU response and 1/1 score. The overview's previous-Attempt score is optional under the current DTO, so this row does not require that optional value or claim every Student viewport.

- [x] Present the "Before you start" settings as a compact summary. Keep each label beside its value in aligned rows, using a compact grid when width permits.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `StudentAssessmentStartFacts` renders the Before you start summary.
  - Evidence (source): `src/components/student_assessment_presentation.css` `.student-assessment-start-facts .assessment-facts > div` places each label beside its value, and the facts list uses a compact grid.
  - Evidence (source): `src/browser_environment.ts` `student_assessment_presentation.css` loads that layout for the application shell.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `Before you start keeps each label beside its value in a compact grid` renders `StudentAssessmentStartFacts`.

- [x] Group Question count and points together, and group availability, deadlines, and Attempt rules into clearly readable sections with concise spacing.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `StudentAssessmentStartFacts` keeps Questions and Points possible together, and `StudentAssessmentDecisionDetails` sections Deadlines and Attempt rules apart from Availability.
  - Evidence (source): `src/components/student_assessment_presentation.css` `.student-assessment-decision__sections` spaces those sections with the compact decision gap.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `Coursework start facts group questions, deadlines, and Attempt rules` renders `StudentAssessmentStartFacts`.

- [x] Express unset or unlimited settings in Student language, such as "No closing time" or "Unlimited Attempts". Format dates in the selected display zone.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `No closing time` labels an unset close, and `Unlimited Attempts` labels an unset Attempt limit.
  - Evidence (source): `src/format_datetime.ts` `createDisplayDateTimeFormatter` formats those instants in the selected display zone.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `unset Coursework limits use Student language in the selected display zone` renders `StudentAssessmentDecisionDetails`.

- [x] Keep the start action close to this summary so Students can review the rules and begin with minimal scrolling.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` renders the start button in `student-assessment-action-region` immediately after `StudentAssessmentStartFacts`.

#### Student Coursework interface
- [x] Students see one Question at a time while completing Coursework.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders one keyed current presentation in one `article.question-card`.

- [x] While completing Coursework, navigation should provide access to every Question and its saved
  status, with direct jumps between Questions.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` renders every position, saved-status label, and position button.
  - Evidence (test): `tests/test_student_assessment_attempt_navigation.mjs` `Student Question navigation renders ordered, answer-free states with one current Question`.

- [x] Leaving a Question and returning should preserve its saved response.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `activatePosition` saves before changing position; `src/pages/assessment_attempt_page.tsx` `loadPresentation` restores the persisted `savedResponse` when the Student returns.

- [x] The current Question and overall progress should remain easy to see.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` owns the visible current/total Question and saved-count summary; `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders this component without the retired duplicate eyebrow.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, accepted supplied `/private/tmp/ple-compact-student-navigation.md` and independent `/private/tmp/ple-compact-navigation-independent-review.md` show visible current/total progress at 1280 and 390 pixels.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders active Question navigation and its loading state only while the Attempt is active; submitted and expired states retain the terminal message without current/saved navigation.
  - Evidence (runtime): `src/pages/assessment_attempt_page.tsx` `AttemptExperience`, accepted authenticated Avery R-4 browser receipt `/private/tmp/ple-attempt-finished-nav-fixed.png`, shows the terminal heading with no active Question navigation, no misleading `Question - of 4`, and no `0 saved` summary. Independent `/root/terminal_attempt_ui_review` accepted the rendered terminal state with no findings.

- [x] The timer should be subtle and keep the focus on the Questions.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` places the `calm-status` timer in the Assessment Attempt header, outside the Question card.

- [x] For timed Coursework, the remaining time should stay visible while moving between Questions.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders `remainingMilliseconds` in the persistent header while keyed Question presentations change below it.

- [x] Submission status should be obvious and use plain language.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders "Submitting Assessment...", "Your answers were accepted", and plain-language save or submission errors from the submission state.

- [x] Present Question navigation as a compact horizontal row of numbered controls, with distinct
  current-Question and saved-status cues.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` provides numbered current/saved controls, width-adaptive first/last/range pagination, ellipses, and Previous/Next.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, supplied 2026-09-16 receipts `/private/tmp/ple-compact-student-navigation.md` and independent acceptance `/private/tmp/ple-compact-navigation-independent-review.md`: actual four-Question 1280/390px saved-response navigation and styled 250-Question harness keyboard traversal, first/last jumps, width adaptation, and reachable local scrolling at 200% enlargement. This is bounded Attempt-navigation evidence, not full Student/browser/theme acceptance.

- [x] For long Question sets, use forum-style pagination with Previous and Next controls, the first
  and last Question numbers, a range around the current Question, and ellipses for omitted ranges.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` provides numbered current/saved controls, width-adaptive first/last/range pagination, ellipses, and Previous/Next.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, supplied 2026-09-16 receipts `/private/tmp/ple-compact-student-navigation.md` and independent acceptance `/private/tmp/ple-compact-navigation-independent-review.md`: actual four-Question 1280/390px saved-response navigation and styled 250-Question harness keyboard traversal, first/last jumps, width adaptation, and reachable local scrolling at 200% enlargement. This is bounded Attempt-navigation evidence, not full Student/browser/theme acceptance.

- [x] Adapt the visible number range to the available width while keeping every Question reachable.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` provides numbered current/saved controls, width-adaptive first/last/range pagination, ellipses, and Previous/Next.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, supplied 2026-09-16 receipts `/private/tmp/ple-compact-student-navigation.md` and independent acceptance `/private/tmp/ple-compact-navigation-independent-review.md`: actual four-Question 1280/390px saved-response navigation and styled 250-Question harness keyboard traversal, first/last jumps, width adaptation, and reachable local scrolling at 200% enlargement. This is bounded Attempt-navigation evidence, not full Student/browser/theme acceptance.

- [x] Keep the Question prompt and response controls near the top of the working area. Give the
  title, timing summary, and Question navigation only the space needed to orient Students.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` provides numbered current/saved controls, width-adaptive first/last/range pagination, ellipses, and Previous/Next.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, supplied 2026-09-16 receipts `/private/tmp/ple-compact-student-navigation.md` and independent acceptance `/private/tmp/ple-compact-navigation-independent-review.md`: actual four-Question 1280/390px saved-response navigation and styled 250-Question harness keyboard traversal, first/last jumps, width adaptation, and reachable local scrolling at 200% enlargement. This is bounded Attempt-navigation evidence, not full Student/browser/theme acceptance.

- [x] Make the current Question, saved-response status, and keyboard-focused control visually distinct
  so Students can recognize where they are, what work is saved, and which action they will activate.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.css` `.is-current` gives the current Question a heavier weight and accent fill, and `.is-saved` adds a saved marker.
  - Evidence (source): `src/components/question_response_control_styles.ts` `QUESTION_RESPONSE_CONTROL_STYLES` paints `.format-status.saved` apart from an unsaved status.
  - Evidence (source): `src/components/question_response_controls/common.tsx` `Status` adds `saved` when the response is saved.
  - Evidence (source): `src/style.css` `:focus-visible` outlines the keyboard-focused control.

- [x] Label response actions by their effect, such as "Save response" and "Clear response", so Students
  can distinguish recording their work from changing it or submitting the whole Coursework.
  - Evidence (source): `src/components/question_response_controls/common.tsx` `Actions` renders the record action, and the Attempt page passes `Save response`.
  - Evidence (source): `src/components/question_response_controls/matching.tsx` `Clear response` is the visible change action; its accessible name is `Clear response for` the prompt. Ordering changes use `Move Ordering Item`.
  - Evidence (source): `src/pages/assessment_attempt_finish.ts` `submitAttemptActionLabel` names whole-Attempt submission `Submit Attempt`, and `src/pages/assessment_attempt_page.tsx` `submitAttemptActionLabel` is the Finish Attempt button.
  - Evidence (test): `tests/test_question_response_controls.mjs` `response actions distinguish Save response, Clear response, and Submit Attempt` renders `QuestionPresentationResponseControl` and calls `submitAttemptActionLabel`.

- [x] Group response feedback near the response controls and keep routine saved-status messages brief.
  - Evidence (source): `src/components/question_response_controls/common.tsx` `responseStatusMessage` places one saved-status sentence in `Status`, directly above the response actions.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `persistenceNotice` passes the Attempt save state into that status line.
  - Evidence (test): `tests/test_question_response_controls.mjs` `saved response status stays one brief message beside the response actions` renders `QuestionPresentationResponseControl` with `responseStatusMessage`.

#### Student Coursework review interface
- [x] Scores and feedback should appear where the Coursework settings allow them.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_history` applies the server-owned feedback-release decision before projecting scores and per-Question feedback; `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` renders only the released fields present in that projection.

- [x] Completed Coursework should remain easy to find and review.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` lists and links previous Attempts; `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` presents the selected Attempt's score and recorded work.

- [x] Group each reviewed Question's number, result, points, recorded response, and permitted feedback
  into a compact, clearly separated unit.
  - Evidence (source): `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` groups the Question number, result, points, recorded response, and released feedback in one unit.
  - Evidence (source): `src/pages/assessment_attempt_summary_styles.ts` `.attempt-summary .attempt-summary__question` separates that unit on the shipped summary content class.

### Sysadmin interface
- N/A Sysadmin interface work is LOW, LOW priority and can be done on an as-needed basis.
  - Reason: audited human-owned work priority and scheduling guidance, not a claim about implemented PLE behavior. The following Sysadmin capability requirements remain binding.

- [x] The Sysadmin interface should focus on system administration.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` labels its workspace "System administration" and manages Instructor Accounts.

- [ ] The Sysadmin menu should make Accounts, Instructors, Courses, and system configuration easy to find.
  - Reason: product decision still unclear
  - Question: Does system configuration need its own menu destination now, or is the menu complete while installation-wide settings remain an open inventory?
  - Mismatch: `src/pages/role_home_pages.tsx` `SysadminHomePage` reaches Instructor Accounts and installation Courses, and `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_ONE` includes `instructorAccounts`, `courses`, and `disciplines`, while no system-configuration destination exists. One reading adds that destination once system-wide settings are identified. The other treats Accounts, Instructors, and Courses as the findable menu until a setting exists.

- [x] Sysadmins should be able to find users quickly by name or email.
  - Evidence (source): `schemas/base_schema/50_functions/profile_media.sql` `list_instructor_account_avatar_summaries` matches a vetted display name or a normalized authentication email and returns only the closed Account summary.
  - Evidence (source): `crates/server/src/instructor_account.rs` `find_instructor_accounts` accepts that query in the request body and returns the same summary.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `findInstructorAccounts` submits the Sysadmin's name or email from the Instructor Accounts page.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `Sysadmins should be able to find users quickly by name or email.` drives the shipped page and client for Ada Lovelace and ada@university.edu, then shows the returned Account ID without the name or email. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Account lists should support searching, filtering, and scanning large numbers of users.
  - Evidence (source): `schemas/base_schema/50_functions/profile_media.sql` `list_instructor_account_avatar_summaries` filters one Account State and returns one page of 50, 100, or 250 closed Account summaries.
  - Evidence (source): `crates/server/src/instructor_account.rs` `find_instructor_accounts` accepts the state, page size, and cursor in the request body.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `instructor-account-state` filters the Instructor Account list and moves through pages of 50, 100, or 250.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `Account lists should support searching, filtering, and scanning large numbers of users.` searched Ada Lovelace, filtered to deactivated accounts, chose 100 accounts per page, and opened the next page on the shipped Instructor Accounts page. The list showed the returned Account ID without the name. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] User pages should clearly show role, account status, and other important administrative information.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `accountContent` shows Role, account state, and last successful sign-in on each Instructor Account row. The list is Instructor Accounts only, so the role is Instructor. Email and display name stay off the row.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `User pages should clearly show role, account status, and other important administrative information.` read Role Instructor, State Active, and the last successful sign-in for U7K3M2PA0 on the shipped Instructor Accounts page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Sysadmins create accounts and manage account access.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` provides Instructor Account creation, deactivation, and reactivation actions.

- [x] Sysadmins approve Instructors before they receive Instructor capabilities.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `createAccount` completes Instructor identity vetting and then creates the account with that decision id.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `require_completed_instructor_identity_vetting` rejects account creation unless that decision is completed for the same email, and `create_instructor_account` calls it before the account insert.
  - Evidence (test): `tests/test_instructor_account_decoder.mjs` `Sysadmins approve Instructors before they receive Instructor capabilities.` refused an account-creation request that omitted the vetting decision before any fetch, then drove the shipped Instructor Accounts page in headless Chromium. The page posted identity vetting and then account creation with the returned decision id. A rejected vetting decision sent no account-creation request. No Live Demo stack was started. No PostgreSQL proof was run.

- [ ] Instructor approval status should be easy to find and change.
  - Reason: product decision still unclear
  - Question: Is Instructor approval the completed pre-account vetting decision, with later access changed only by deactivate and reactivate, or a distinct Account status a Sysadmin can find and change after creation?
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` exposes active, deactivated, and closed lifecycle state, and `schemas/base_schema/20_tables/account.sql` `instructor_identity_vetting_decision` stores a completed vetting decision that is not an Account state. One reading keeps approval as that immutable decision and changes access only through deactivate and reactivate. The other adds a separate post-creation approval status. Human Guidance asks for a status that is easy to find and change, and it also says a Sysadmin approves an Instructor before that Account receives Instructor capabilities.

- [x] Sysadmins should be able to find and inspect Courses across the installation.
  - Evidence (source): `schemas/base_schema/50_functions/sysadmin_course_inspection.sql` `list_installation_courses` returns one page of installation Courses, and `src/pages/sysadmin_course_inspection_page.tsx` `SysadminCourseInspectionListPage` finds them from the Sysadmin home.
  - Evidence (test): `tests/test_sysadmin_course_inspection.mjs` `sysadmin installation course inspection shows instructor and status` renders the shipped list for an installation Course. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Course administration should show the Instructor and important Course status information.
  - Evidence (source): `schemas/base_schema/50_functions/sysadmin_course_inspection.sql` `load_installation_course` returns the active Instructor display names, activity, and retention state, and `src/pages/sysadmin_course_inspection_page.tsx` `SysadminCourseInspectionPage` shows those facts.
  - Evidence (test): `tests/test_sysadmin_course_inspection.mjs` `sysadmin installation course inspection shows instructor and status` renders the Instructor, activity, and retention on the shipped inspection page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Sysadmins should manage Courses through Sysadmin interfaces and capabilities.
  - Decision: Human Guidance assigns Sysadmins creation of a Course Instance for an Instructor who teaches it. Retention transitions stay with the background process.
  - Evidence (source): `src/components/sysadmin_course_creation.ts` `submitSysadminCourseCreation` creates an empty Course for the selected Instructor, and `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` assigns that Instructor when the actor is a Sysadmin.
  - Evidence (source): `src/pages/role_home_pages.tsx` `SysadminCourseCreation` is the Sysadmin home control for that creation.
  - Evidence (test): `tests/test_frontend_contract.mjs` `a Sysadmin creates a Course that an Instructor teaches` drives `submitSysadminCourseCreation` and records the assigned Instructor. No Live Demo stack was started. No PostgreSQL proof was run.

- [ ] System-wide settings should have their own area, separate from user and Course administration.
  - Reason: product decision still unclear
  - Question: Which implemented installation-wide settings must Sysadmins view or change, and which source-of-truth boundary owns each?
  - Mismatch: `src/ribbon/ribbon_catalog.ts` has no system-settings destination. The guidance could require a page for actual platform settings, or no page until implemented system-owned settings exist; current evidence cannot select between those readings.

- [x] Everyday navigation should emphasize frequently used administrative tasks.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `instructorAccounts` is a primary critical task while `supportRoster` is supporting normal priority.

- [ ] Rare installation and configuration tasks should remain available through secondary navigation.
  - Reason: product decision still unclear
  - Question: Does this require a secondary Ribbon row now, or does reachability from the Sysadmin home satisfy it while the complete Ribbon layout stays unlocked?
  - Mismatch: `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_TWO` has no Sysadmin destinations, and `src/pages/role_home_pages.tsx` `SysadminHomePage` still links Disciplines, Library activity, and scoped roster support. `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` already includes `instructorAccounts` and `disciplines` as primary tasks. One reading adds a secondary Ribbon row for rare tasks. The other keeps those tasks on the Sysadmin home until Human Guidance locks the complete Sysadmin Ribbon layout.

- [x] High-consequence administrative actions should have a visually distinct area.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `renderAccountBody` places deactivation in `instructor-account-consequence`, apart from ordinary account facts.
  - Evidence (source): `src/pages/instructor_accounts_page.css` `.instructor-account-consequence` gives that area its own border and fill.

- [x] Confirmation for destructive actions should clearly state what will happen.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `requestDeactivation` states that the Instructor Account is deactivated, current sessions end, and sign-in stays blocked until reactivation, and `deactivate` runs only after that confirmation.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `revoke_sessions_after_account_deactivation_or_closure` ends current sessions when the account is deactivated.

- [ ] The complete Sysadmin Ribbon task layout does not have a locked-in design yet.
  - Reason: HG: no locked-in design.
  - Mismatch: no complete Sysadmin Ribbon task layout can be verified until the design is locked.
