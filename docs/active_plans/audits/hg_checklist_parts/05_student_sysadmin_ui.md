### Student interface

#### General Student interface

- [x] The Student interface should focus on current Courses, Coursework, and work that needs attention.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` lists current courses; `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` lists assigned work.
- [ ] **Coursework** is the Student-facing collective term for Regular Assignments, Practice Question
  Assignments, Bonus Assignments, Quizzes, and Exams.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` labels the collective section and its loading and empty states "Coursework," while each item receives one canonical Type from the typed projection.
  - Mismatch: the actual PostgreSQL landing Store proved the same closed Type column with Regular Assignment and the compiled SolidJS/mock-API browser evidence was accepted, but the connected Student HTTP workflow was not run; this broad Student-facing terminology row remains runtime-unverified.
- [ ] Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentCard` renders the specific Type label and uses it in the individual item's open action; `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` uses it in start, resume, and availability language.
  - Mismatch: the actual PostgreSQL access and landing Stores projected the specific Type and accepted compiled SolidJS/mock-API browser evidence rendered it, but the connected Student HTTP workflow and other Student-facing surfaces were not exercised; this broad interface row remains open.
- [x] The Student Ribbon should use familiar Student language rather than internal PLE terms such as Assessment.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` labels the Student-only tab "Coursework" and `RIBBON_TASK_CATALOG` labels its return task "Back to Coursework"; `src/ribbon/ribbon_contract.ts` uses "Before you start" when the individual Coursework title is not yet available, "Attempt" for the Student Attempt task area and current breadcrumb, and "Attempt history" for the history breadcrumb. Instructor Assessment labels remain separate.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Coursework navigation retains collective labels` and its canonical breadcrumb projections passed with the focused 15-test Ribbon contract lane.
  - Evidence (runtime): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` and `src/ribbon/ribbon_contract.ts` `deriveRibbonModel` were exercised by accepted isolated actual-server/exact-main Student browser receipts `/private/tmp/ple-course-empty-artifacts.r7S1T6` and `/private/tmp/ple-course-empty-artifacts.ONrLSK`. They showed the Coursework Ribbon tab, "Before you start" overview breadcrumb, specific Practice Question Assignment Type, and the submitted Attempt history route/breadcrumb after a real native Student Attempt. The source catalog keeps the return task "Back to Coursework"; the complete Student Ribbon task layout remains separately unlocked.
- [x] The Student interface should make the next useful action easy to find.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentCard` presents the primary "Open Assessment" action.
- [ ] The Student menu is simpler than the Instructor menu.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` `RIBBON_TASK_CATALOG` does not define a complete Student menu for comparison.
- [ ] Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.
  - Mismatch: needs runtime evidence for the four required Student viewport classes; `tests/playwright/student_course_entry_m6_evidence.mjs` does not cover them.
- [ ] Student layouts should adapt smoothly at intermediate widths, with readable long titles and
  controls that wrap or rearrange in the task's reading order.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Every Student browser action should be usable with the keyboard alone.
  - Mismatch: needs keyboard-only journey evidence; `src/pages/assignment_attempt_page.tsx` has keyboard-operable controls but no complete Student journey test.
- [x] Student pages should use names meaningful to Students.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` uses "Your courses" and "Open assigned work".
- [ ] Student navigation and pages should contain only Student interfaces and capabilities.
  - Mismatch: `src/route_contract.ts` `studentCourseLanding` restricts that one route to Students, but source inspection is not evidence that every Student navigation and page exposes only Student capabilities; the required authorization/runtime check has not been recorded.
- [x] Student content entry should use the response controls provided by Questions and other Student activities.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` passes the current Question presentation's response format to `QuestionPresentationResponseControl`.
- [ ] Students should have no upload capabilities. Instructor-created content should use text boxes.
  - Mismatch: Student upload denial is not sufficient to verify the universal Instructor text-box requirement.
  - Owner: Interface design > General interface design (first identical Human Guidance occurrence).
- [ ] The complete Student Ribbon task layout does not have a locked-in design yet.
  - Reason: HG: no locked-in design.
  - Mismatch: no complete Student Ribbon task layout can be verified until the design is locked.

#### Student Course and Coursework interface

- [x] Students enrolled in one active Course should go directly into that Course.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` redirects the one-entry `courses()` result to its Course reference.
- [x] Students should be able to see their active Courses and Coursework from the main navigation.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` provides the current-Course index; `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` provides its work.
- [ ] Course invitations should show the Course name and relevant Instructor and term information
  before the Student accepts the invitation.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [x] Course pages should make upcoming, available, completed, and missed Coursework easy to distinguish.
  - Evidence (source): `src/pages/student_coursework_presentation.ts` `studentCourseworkDisplay` maps the server-owned start decision, completion, and resumability to upcoming, available, in-progress, completed, or missed learner states.
  - Evidence (test): `tests/test_student_coursework_presentation.mjs` `Coursework display distinguishes resumable, non-resumable unfinished, and completed work` exercises the pure state projection.
  - Evidence (runtime): `crates/learning-data-access/tests/assessment_access_postgres.rs` `access_reader_projects_one_authoritative_decision_and_effective_policy` passed on a fresh PostgreSQL 17 database, proving the landing Store projects scheduled, expired unfinished, active resumable, Attempt-limit-reached resumable, and late-work-refused resumable states; accepted compiled SolidJS/mock-API browser evidence proved their visible presentation. This does not claim a connected HTTP-server run.
- [x] Coursework lists should make due dates, Type, and completion status easy to scan.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentCard` presents visible Type, due, completion, and state fields from the typed Student projection.
  - Evidence (runtime): `crates/learning-data-access/tests/assessment_access_postgres.rs` `access_reader_projects_one_authoritative_decision_and_effective_policy` passed on a fresh PostgreSQL 17 database and projected the Regular Assignment Type, due/availability facts, completion, and resumability through the actual landing Store; accepted compiled SolidJS/mock-API browser evidence proved the fields are scannable. This does not claim a connected HTTP-server run.
- [ ] Keep Coursework entries compact in height so Students can scan several items at once.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Keep essential Coursework information and the main action visible, with fuller access and timing
  details available through progressive disclosure.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- N/A Coursework lists may provide filters for **Regular Assignments**, **Practice Question Assignments**,
  **Bonus Assignments**, **Quizzes**, and **Exams**.
  - Reason: Optional permission does not require current product behavior.
- [x] Each Coursework item should clearly show its Assessment Type using its label and Type icon.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentCard` always renders `typePresentation().label` beside the guaranteed bundled `typePresentation().icon`; semantic Type color is supplementary.
- [x] Before starting Coursework, Students should see its title, Type, Question count, points possible,
  time limit, and previous Attempts.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` presents the title, specific Type, Question count, points possible, time limit, and previous Attempts before start.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `StudentAssessmentStartFacts` owns the compact Question, points, and time-limit facts.
  - Evidence (runtime): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` was exercised by accepted isolated actual-server/exact-main Student proof `/private/tmp/ple-course-empty-artifacts.JCFLm9` after a real roster import/claim. It opened a Released direct Practice Assessment before Start and showed title, Practice Type, one Question, one point, the exact one-hour limit, and an explicit zero-previous-Attempt state; an Unreleased sibling was omitted and an outsider received 404. A separate accepted native Student HTTP/browser run `/private/tmp/ple-course-empty-artifacts.ONrLSK` whole-submitted a real graded 1/1 Attempt, then reopened the overview before starting another. The same six facts included an actual "Previous attempts" Attempt 1 Submitted link; its clicked history showed recorded PKU response and 1/1 score. The overview's previous-Attempt score is optional under the current DTO, so this row does not require that optional value or claim every Student viewport.
- [ ] Present the "Before you start" settings as a compact summary. Keep each label beside its value
  in aligned rows, using a compact grid when width permits.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Group Question count and points together, and group availability, deadlines, and Attempt rules
  into clearly readable sections with concise spacing.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Express unset or unlimited settings in Student language, such as "No closing time" or
  "Unlimited Attempts", and show the time zone once beside the timing group.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Keep the start action close to this summary so Students can review the rules and begin with
  minimal scrolling.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.

#### Student Coursework interface

- [x] Students see one Question at a time while completing Coursework.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders one keyed current presentation in one `article.question-card`.
- [x] While completing Coursework, navigation should provide access to every Question and its saved
  status, with direct jumps between Questions.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` renders every position, saved-status label, and position button.
  - Evidence (test): `tests/test_student_assessment_attempt_navigation.mjs` `Student Question navigation renders ordered, answer-free states with one current Question`.
- [x] Leaving a Question and returning should preserve its saved response.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `activatePosition` saves before changing position; `src/pages/assessment_attempt_page.tsx` `loadPresentation` restores the persisted `savedResponse` when the Student returns.
- [ ] The current Question and overall progress should remain easy to see.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` owns the visible current/total Question and saved-count summary; `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders this component without the retired duplicate eyebrow.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, accepted supplied `/private/tmp/ple-compact-student-navigation.md` and independent `/private/tmp/ple-compact-navigation-independent-review.md` show visible current/total progress at 1280 and 390 pixels.
  - Verification pending: supplied parent actual submitted R-4 navigation shows disabled controls with misleading `Question - of 4` and `0 saved`, despite one retained correct MATCH response in history. Active-Attempt current/progress proof remains accepted; truthful submitted-state summary needs the separately queued source correction and rendered verification. The five bounded active navigation closures are unchanged.
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
- [ ] Use consistent PLE styling for native Question navigation and response actions, with readable
  labels and clear selected, saved, and keyboard-focus states.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [x] Style Question navigation controls with PLE typography, deliberate spacing, restrained corner
  rounding, and theme-aware borders and backgrounds.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` provides numbered current/saved controls, width-adaptive first/last/range pagination, ellipses, and Previous/Next.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, supplied 2026-09-16 receipts `/private/tmp/ple-compact-student-navigation.md` and independent acceptance `/private/tmp/ple-compact-navigation-independent-review.md`: actual four-Question 1280/390px saved-response navigation and styled 250-Question harness keyboard traversal, first/last jumps, width adaptation, and reachable local scrolling at 200% enlargement. This is bounded Attempt-navigation evidence, not full Student/browser/theme acceptance.
- [ ] Group response feedback near the response controls and keep routine saved-status messages brief.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.

#### Student Coursework review interface

- [x] Scores and feedback should appear where the Coursework settings allow them.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_history` applies the server-owned feedback-release decision before projecting scores and per-Question feedback; `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` renders only the released fields present in that projection.
- [x] Completed Coursework should remain easy to find and review.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` lists and links previous Attempts; `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` presents the selected Attempt's score and recorded work.
- [ ] Group each reviewed Question's number, result, points, recorded response, and permitted feedback
  into a compact, clearly separated unit.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.

### Sysadmin interface

- N/A Sysadmin interface work is LOW, LOW priority and can be done on an as-needed basis.
  - Reason: audited human-owned work priority and scheduling guidance, not a claim about implemented PLE behavior. The following Sysadmin capability requirements remain binding.
- [x] The Sysadmin interface should focus on system administration.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` labels its workspace "System administration" and manages Instructor Accounts.
- [ ] The Sysadmin menu should make Accounts, Instructors, Courses, and system configuration easy to find.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` defines Instructor Accounts and Scoped Support only; it has no Sysadmin Courses or system configuration destinations.
- [ ] Sysadmins should be able to find users quickly by name or email.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` has no name or email search.
- [ ] Account lists should support searching, filtering, and scanning large numbers of users.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` lists all Instructor Accounts without search, filters, or large-list pagination.
- [ ] User pages should clearly show role, account status, and other important administrative information.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` displays only Instructor Account reference, state, and sign-in time; no user detail page exists.
- [x] Sysadmins create accounts and manage account access.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` provides Instructor Account creation, deactivation, and reactivation actions.
- [ ] Sysadmins approve Instructors before they receive Instructor capabilities.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `createAccount` creates an active Instructor Account directly; there is no approval state.
- [ ] Instructor approval status should be easy to find and change.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` exposes active/deactivated/closed account lifecycle state, but no Instructor approval status exists to find or change.
- [ ] Sysadmins should be able to find and inspect Courses across the installation.
  - Mismatch: `src/pages/support_roster_page.tsx` `SupportRosterPage` inspects only one Instructor-issued exact-capability roster, not installation-wide Courses.
- [ ] Course administration should show the Instructor and important Course status information.
  - Mismatch: no Sysadmin Course administration page or Course status projection exists in `src/pages/`.
- [ ] Sysadmins should manage Courses through Sysadmin interfaces and capabilities.
  - Mismatch: `src/route_contract.ts` has no Sysadmin Course-management route.
- [ ] System-wide settings should have their own area, separate from user and Course administration.
  - Reason: product decision still unclear
  - Question: Which implemented installation-wide settings must Sysadmins view or change, and which source-of-truth boundary owns each?
  - Mismatch: `src/ribbon/ribbon_catalog.ts` has no system-settings destination. The guidance could require a page for actual platform settings, or no page until implemented system-owned settings exist; current evidence cannot select between those readings.
- [x] Everyday navigation should emphasize frequently used administrative tasks.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `instructorAccounts` is a primary critical task while `supportRoster` is supporting normal priority.
- [ ] Rare installation and configuration tasks should remain available through secondary navigation.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` contains only the primary Instructor Accounts Sysadmin destination; no rare installation or configuration task is available through secondary Sysadmin navigation.
- [ ] High-consequence administrative actions should have a visually distinct area.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `Deactivate Instructor Account` uses the ordinary `quiet-action` styling with no distinct high-consequence area.
- [ ] Confirmation for destructive actions should clearly state what will happen.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `deactivate` executes immediately after a reason is entered; no confirmation step states the consequence.
- [ ] The complete Sysadmin Ribbon task layout does not have a locked-in design yet.
  - Reason: HG: no locked-in design.
  - Mismatch: no complete Sysadmin Ribbon task layout can be verified until the design is locked.
