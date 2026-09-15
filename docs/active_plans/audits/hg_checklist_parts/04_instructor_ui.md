### Instructor interface

- [ ] The Instructor interface should make frequent teaching tasks fast and easy to find.
  - Mismatch: the main Instructor task areas still contain deferred destinations and no end-to-end usability evidence establishes this broad workflow claim.
- [ ] The Instructor menu has **Courses**, **Questions**, and **Assessments** in one dense top bar.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` labels the third tab "Assignments," not the required "Assessments."
- [x] Instructor Profile uses a generic user icon until the **Instructor** adds a Profile image.
  - Evidence (source): `src/features/profile_avatar/ribbon_account_avatar.tsx` `RibbonAccountAvatar` falls back to `RibbonIcon` `circle-user` when no Profile image or provided avatar exists.
- [ ] All required ribbon choices remain visible even when their collection is empty.
  - Mismatch: several required choices have `future` destinations in `src/ribbon/ribbon_catalog.ts` and are not admitted as usable controls.
- [x] A working navigation destination remains visible when its collection is empty.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` retains the courses route and renders an empty state.
- [x] A future or unavailable capability should not appear as a usable control until its workflow exists.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `RibbonDestination` distinguishes `route` from `future` destinations.
- [ ] Empty collection pages should explain what the collection is for and provide an obvious action to create or add the first item when the user can do so.
  - Mismatch: `src/pages/library_page.tsx` empty Question Library results offer no action to add or create a first item.
- [ ] Similar pages should place similar actions in consistent locations.
  - Mismatch: no cross-page layout contract or test verifies consistent action placement.
- [ ] Instructor pages should be composed around the teaching task rather than collections of padded components.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_live_page.tsx` presents Assignment-named tasks, and no behavior or rendered-layout evidence verifies this broad Instructor-interface judgment.
- [ ] Instructor Course and Assessment lists should be dense and easy to scan, more like a spreadsheet than cards.
  - Mismatch: `src/pages/course_list_page.tsx` has a dense Course Instance row, but no Assessment-named list exists; `src/pages/assignments_due_soon_page.tsx` still presents Assignments.
- [ ] Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_student_view_page.tsx` `AssessmentWorkspaceStudentViewPage` renders the server-authorized answer-free Assessment projection, loads only the manifest and selected Question, disables native response controls, and provides no Student Work, Assessment Attempt, submission, or grade action.
  - Evidence (source): `crates/server/src/assessment_student_view.rs` `assessment_student_view_router`, `crates/learning-data-access/src/postgres/assessment_student_view.rs` `PostgresInstructorStudentViewStore`, and `schemas/base_schema/assessment_student_view.sql` `load_instructor_student_view_question_source` implement the authorized no-write server, Store, and SQL boundaries.
  - Evidence (runtime): a fresh PostgreSQL proof exercised the real Store through the API roles with a nonempty Ready Asset rendition and verified read-only SQLSTATE `25006` plus zero writes to Student-state tables.
  - Evidence (test): an independently reviewed Chromium component proof with mock transport covered native and WeBWorK presentations, navigation, disabled controls, stale and error recovery, and no mutation requests; it was not connected or live-stack acceptance.
  - Mismatch: connected live-HTTP acceptance is still missing; the unchanged full server compile is blocked in the AWS dependency graph; and production iMathAS Student View integration remains deferred outside the pilot. Independent final server source review passed, but it does not establish runtime behavior.

#### Courses

- [ ] The **Courses** ribbon must include: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` declares all four labels, but three are `future` destinations rather than working ribbon destinations.
- [x] Course lists should support scanning and comparison without opening each Course.
  - Evidence (source): `src/pages/course_list_page.tsx` `CourseInstanceRow` renders each Course Instance name, term, theme, and open action in the list.
- [ ] The Course Editor should show the Course structure and its ordered Assessments without showing every Question at once.
  - Mismatch: `src/pages/course_instance_page.tsx` is an Assignment list rather than the required Course Editor and uses the retired Assignment product term.
- [ ] Selecting an Assessment in the Course Editor opens that Assessment for editing.
  - Mismatch: Course Instance rows link to Assignment routes; no Assessment-named Course Editor exists.
- [ ] Assessment content and Assessment properties should remain separate editing tasks.
  - Mismatch: the implemented separation is for the retired Assignment object in `src/pages/assignment_workspace/`, not the HG Assessment object.
- [ ] My Active Courses and My Inactive Courses should both be available from the Courses area.
  - Mismatch: both controls are `future` destinations in `src/ribbon/ribbon_catalog.ts`.

##### Blueprint Courses

- [x] **My Blueprint Courses** should emphasize reusable course design rather than teaching activity.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCoursesWorkspace` presents reusable Blueprint Course content and adoption information.
- [ ] **Search Public Blueprint Courses** helps Instructors find a Blueprint Course they already have in mind.
  - Mismatch: `searchPublicBlueprintCourses` is a `future` destination in `src/ribbon/ribbon_catalog.ts`.
- [ ] Public Blueprint Course search should support quickly narrowing a large collection.
  - Mismatch: no Public Blueprint Course search route or query controls exist.
- [x] A **Blueprint Course** should provide an obvious action for creating a **Course Instance** from it.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` renders "Create Course Instance from this Blueprint."
- [ ] Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor.
  - Mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` opens a Course Editor but names the selected editor a Blueprint Assignment editor, not a Blueprint Assessment Editor.
- [x] The Course Editor should show the Blueprint Course structure without editing every Question on one page.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` lists modules and assignments before selecting an editor.
- [ ] Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment.
  - Mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` uses `setSelectedAssignment` and a Blueprint Assignment editor rather than the required Blueprint Assessment.
- [ ] Only the selected Blueprint Assessment's Questions should appear in its editor.
  - Mismatch: the selected scope in `src/features/blueprint_course/blueprint_course_workspace.tsx` is a `BlueprintAssignmentContentEditor`, not a Blueprint Assessment editor.
- [ ] **Blueprint Assessment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assessment.
  - Mismatch: the implemented editor calls the product object a Blueprint Assignment, not a Blueprint Assessment.
- [ ] **Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
  - Mismatch: `src/features/blueprint_course/blueprint_assignment_content_editor.tsx` supplies reusable defaults but not the required separate Blueprint Assessment Properties Editor.
- [x] Blueprint Courses should not contain Assessment dates or relative Assessment schedules.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` describes reusable structure without deadlines or course delivery settings.
- [ ] Blueprint Courses follow the lifecycle **Private -> Public -> Archived**.
  - Mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` exposes only available/archive states, not the required Private/Public lifecycle.
- [ ] New and forked Blueprint Courses start **Private**.
  - Mismatch: no verified fork workflow or Private initial availability appears in the Blueprint Course UI.
- [ ] Private Blueprint Courses are visible only to their owner.
  - Mismatch: source evidence identifies `blueprint_course_owner` read access but does not verify the required Private visibility behavior.
- [ ] Instructors may develop and use Private Blueprint Courses without publishing them.
  - Mismatch: no Private availability workflow is implemented in the visible Blueprint Course controls.
- [ ] Making a Blueprint Course **Public** adds it to the shared Blueprint Course collection.
  - Mismatch: no Public transition or shared Public collection is implemented.
- [ ] A Public Blueprint Course with no adoptions may return to **Private**.
  - Mismatch: no Public-to-Private transition is implemented.
- [ ] A Public Blueprint Course with one or more adoptions remains **Public**.
  - Mismatch: no Public availability model is implemented.
- [x] Archived Blueprint Courses leave normal discovery but remain available where needed for history.
  - Evidence (source): `crates/server/src/blueprint_course.rs` `archive_blueprint` and discovery query comments retain historical pins while excluding archived discovery.
- [ ] Instructors may fork a Public Blueprint Course to continue development privately.
  - Mismatch: no Public Blueprint Course fork action or Private fork lifecycle is implemented.
- [ ] Blueprint Courses do not have a separate Draft state.
  - Mismatch: no source-level lifecycle test establishes the absence of a Draft state across Blueprint Course APIs and UI.

##### Course Instances

- [ ] **My Active Courses** should emphasize Course Instances the Instructor is currently teaching.
  - Mismatch: `myActiveCourses` is a `future` Ribbon destination.
- [ ] Active Course Instances should make upcoming Assessments and important course activity easy to find.
  - Mismatch: no active-only Course Instance view exists and the UI uses Assignment rather than Assessment.
- [ ] **My Inactive Courses** should keep past Course Instances available without competing with active Course Instances.
  - Mismatch: `myInactiveCourses` is a `future` Ribbon destination.
- [ ] Creating a Course Instance from a Blueprint Course preserves its Assessments, Questions, pools, and settings.
  - Mismatch: the creation path and `tests/e2e/e2e_live_demo_course_instance.sh` adopt Blueprint Assignments; they do not verify the required Assessment-named contents and settings contract.
- [ ] Assessments created from a Blueprint Course start unreleased with dates unset.
  - Mismatch: `tests/e2e/e2e_live_demo_course_instance.sh` verifies an adopted Assignment, not the required Assessment product behavior.
- [ ] A Course Instance represents one teaching period and remains Active for at most six months from
  creation.
  - Mismatch: `crates/question_model/src/course_term.rs` `CourseTerm::new` only requires an end date not before the start date; no implementation establishes a six-month Active limit from Course Instance creation.
- [ ] Course banners use a 5:1 aspect ratio.
  - Mismatch: `src/features/course_appearance/course_entry_identity.tsx` `COURSE_ENTRY_IDENTITY_STYLES` declares `aspect-ratio: 6 / 1`, not 5:1.
- [ ] 1280 by 256 pixels is the recommended Course banner authoring size.
  - Mismatch: the checked banner presentation in `src/features/course_appearance/course_entry_identity.tsx` uses a 1200-by-200 6:1 rendition; no 1280-by-256 authoring recommendation exists.
- [ ] Higher-resolution 5:1 Course banner images are supported.
  - Mismatch: the cited `tests/e2e/e2e_course_appearance.sh` does not contain `course_banner_upload` or an assertion that a higher-resolution 5:1 upload is accepted.
- [ ] PLE responsively scales Course banners while preserving their aspect ratio.
  - Mismatch: `tests/playwright/e2e/course_appearance_propagation.spec.ts` verifies upload, persistence, and visibility at one 1280px viewport, not responsive scaling or aspect-ratio preservation across viewports.
- [ ] Course banners appear as small centered banners rather than full-width page heroes.
  - Mismatch: `src/features/course_appearance/course_entry_identity.tsx` has a centered 6:1 entry banner, but no rendered evidence verifies the required visual comparison with a full-width page hero.
- [x] An Instructor can upload a Course banner and select a three-color theme.
  - Evidence (source): `src/pages/course_appearance_page.tsx` `AppearanceBannerEditor` contains `saveBanner`, which calls `uploadCourseBanner` and `setCourseBanner`; `AppearanceThemeEditor` selects a `COURSE_THEME_OPTIONS` theme.
  - Evidence (source): `src/features/course_appearance/course_theme_registry.ts` `ThemeAnchors` defines each theme's canvas, secondary, and accent colors.
  - Evidence (test): `tests/playwright/e2e/course_appearance_propagation.spec.ts` `Instructor saves both properties and enrolled Student receives only that Course appearance` independently saves, reloads, and displays a Course theme and banner.
- [ ] Course Instance Assessments have two editors:
  - Mismatch: the UI names the object Assignment, not Assessment.
  - [ ] **Assessment Question Editor**: Selects, adds, removes, and orders Questions in an Assessment.
    - Mismatch: the implemented `AssignmentWorkspaceQuestionsPage` is not an Assessment-named editor.
  - [ ] **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and what **Students** can see.
    - Mismatch: the implemented `AssignmentWorkspacePoliciesPage` is not an Assessment-named properties editor.

#### Questions

- [ ] The **Questions** ribbon must include: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library.
  - Mismatch: My Questions, Starred, and Watched are `future` destinations in `src/ribbon/ribbon_catalog.ts`.
- [ ] **My Questions** should make the Instructor's Published Questions easy to find and manage.
  - Mismatch: `myQuestions` is a `future` Ribbon destination.
- [x] **My Draft Questions** should emphasize Questions that still need work before publication.
  - Evidence (source): `src/pages/question_drafts_page.tsx` `QuestionDraftsPage` presents the current Instructor's drafts.
- [ ] **Starred** should provide a quick personal collection of Questions the Instructor wants to keep handy.
  - Mismatch: `starred` is a `future` Ribbon destination.
- [ ] **Watched** should help Instructors follow Questions where changes or activity matter to them.
  - Mismatch: `watched` is a `future` Ribbon destination.

##### Search Question Library

- [x] **Search Question Library** helps Instructors find specific Questions in a large library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` supplies the searchable published Question Library.
- [ ] Search should begin with a prominent search box, similar to Google Search.
  - Mismatch: `src/pages/library_page.tsx` places `Search published questions` before filters, but no rendered UX evidence establishes the required prominence or Google-like presentation.
- [ ] The initial Search page should stay simple and focus attention on entering a search.
  - Mismatch: `src/pages/library_page.tsx` presents seven filter controls alongside the initial search field.
- [x] Search should assume the Instructor has some idea what they want to find.
  - Evidence (source): `src/pages/library_page.tsx` search input placeholder `Title or concept` directs a known-item search.
- [x] Question Library search should work well with ordinary words by default.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` passes ordinary `search` text to the Question Library query.
- [x] Search results should switch to a dense, information-rich layout.
  - Evidence (source): `src/pages/library_page.tsx` `question-library-row` renders title, summary, authors, identifier, and action per result.
- [x] Results should make it easy to scan many Questions quickly.
  - Evidence (source): `src/pages/library_page.tsx` `questionLibraryBrowseVirtualWindow` virtualizes dense result rows.
- [x] Results should show the information needed to judge relevance without opening each Question.
  - Evidence (source): `src/pages/library_page.tsx` `question-library-row` shows title, summary, authors, and identifier.
- [x] Search results should support filters for narrowing the Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` supplies author, backend, tag, Question Type, license, Course-use, and capability filters.
- [x] Filters should update the current search rather than start a separate workflow.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` resets one `QuestionLibraryBrowseSession` with the updated query.
- [ ] Search should support Google-like syntax for more precise queries.
  - Mismatch: `src/pages/library_page_model.ts` exposes plain `search` text and no parsed advanced query syntax.
- [ ] Quoted text should search for an exact phrase.
  - Mismatch: no exact-phrase query parser or test exists.
- [ ] A minus sign should exclude matching terms.
  - Mismatch: no exclusion query parser or test exists.
- [ ] Search should support PubMed-like field tags such as `topic:genetics`.
  - Mismatch: no field-tag query parser or `topic` filter exists.
- [ ] Field tags should use PLE concepts and vocabulary.
  - Mismatch: field tags are not implemented.
- [ ] Useful fields may include subject, topic, tags, Question Type, and author.
  - Mismatch: the UI has tags, Question Type, and author filters but lacks subject and topic fields and field-tag search.
- [ ] Simple and advanced searches should use the same search box.
  - Mismatch: advanced search syntax is not implemented.
- [x] Instructors should not need to learn search syntax to use Search Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` exposes ordinary search and labeled filter controls without syntax requirements.
- [ ] The interface should make useful search syntax discoverable when needed.
  - Mismatch: no search syntax exists or is documented in the interface.
- [ ] Search syntax should help expert users quickly narrow a very large Question Library.
  - Mismatch: no advanced search syntax exists.
- [x] Search terms and active filters should remain visible while reviewing results.
  - Evidence (source): `src/pages/library_page.tsx` `query` signal remains bound to the search input and filter selects while rows render.
- [x] Clearing or changing part of a search should be quick.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` updates the search session on each input or selection change.
- [ ] Opening a result and returning should preserve the Instructor's search and position.
  - Mismatch: `src/pages/library_page.tsx` keeps query and scroll state only in the mounted component; no route-return persistence exists.

##### Browse Question Library

- [ ] **Browse Question Library** helps Instructors explore Questions without knowing what to search for.
  - Mismatch: `browseQuestionLibrary` and Search share the same `library` route with no distinct browse workflow.
- [ ] Browse should help Instructors understand what the Question Library contains.
  - Mismatch: no browse landing surface explains the library's content.
- [ ] Browse should emphasize subjects, topics, tags, Question Types, and other useful groupings.
  - Mismatch: `src/pages/library_page.tsx` has some filters but no subject/topic browse hierarchy.
- [ ] Browse should make moving from broad subjects to narrower topics easy.
  - Mismatch: no subject-to-topic browse hierarchy exists.
- [ ] Browse should show useful counts where they help Instructors choose where to explore.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` routes Browse and Search to the same undifferentiated Library page; count-bearing Search filters do not establish a Browse workflow.
- [ ] Browse results should use the same dense Question presentation used by Search where practical.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` maps Browse and Search to one undifferentiated route, so no Browse result presentation exists to compare with Search.
- [ ] Instructors should be able to move from browsing into a more focused search.
  - Mismatch: no separate Browse state or transition to focused Search exists.
- [ ] Search and Browse are different paths into the same **Question Library**.
  - Mismatch: both Ribbon controls route directly to the same undifferentiated `library` route.

#### Assessments

- [ ] The **Assessments** ribbon must include: Assessments Due Soon, My Assessment Templates.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` uses "Assignments Due Soon" and "My Assignment Templates"; templates are a `future` destination.
- [ ] **Assessments Due Soon** should emphasize Assessments that may need the Instructor's attention.
  - Mismatch: `src/pages/assignments_due_soon_page.tsx` implements the retired Assignment term rather than Assessments.
- [ ] Assessment lists should make Course, release status, due date, and other important state easy to scan.
  - Mismatch: `src/pages/assignments_due_soon_page.tsx` implements an Assignment list rather than an Assessment list.
- [ ] **My Assessment Templates** should emphasize reusable Assessment design rather than Course activity.
  - Mismatch: `assignmentTemplates` is a `future` Ribbon destination.
- [ ] Assessment editing has two editors:
  - Mismatch: current editors are Assignment-named.
  - [ ] **Assessment Question Editor**: Selects, adds, removes, and orders Questions.
    - Mismatch: `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx` implements an Assignment Questions editor.
  - [ ] **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and other Assessment settings.
    - Mismatch: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` implements Assignment Policies.
- [ ] The two Assessment editors should remain clearly distinct.
  - Mismatch: two Assignment editors exist, but the required Assessment terminology and editors do not.
- [ ] The Assessment Question Editor should make Question order easy to understand at a glance.
  - Mismatch: Question ordering is implemented for Assignment, not Assessment, editor terminology.
- [ ] Adding Questions should provide direct paths to Search and Browse Question Library.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx` provides available Questions but no direct Search and Browse Question Library paths.
- [ ] Instructors should be able to inspect a Question before adding it to an Assessment.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx` provides no Question inspection action before Add Question.
- [ ] Assessment Properties should group related settings so important settings are easy to find.
  - Mismatch: the implemented UI is Assignment Policies, not the required Assessment Properties surface.
- [ ] Instructors can randomize Question order for an Assessment.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` applies randomization to an Assignment, not an Assessment.
- [ ] Answer-choice randomization belongs to the Question, not the Assessment.
  - Mismatch: the implemented explanatory text uses Question and Assignment, not the required Assessment terminology.
- [ ] **Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches.
  - Mismatch: the route implements upcoming Assignments, not Assessments.
- [ ] Assessments Due Soon shows the Course and due time for each Assessment.
  - Mismatch: `src/pages/assignments_due_soon_page.tsx` displays Course and due time for Assignments, not Assessments.

#### High-consequence actions

- [ ] Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
  - Mismatch: the implemented UI names the first action "Unrelease assignment," not Assessment Unrelease.
- [ ] Danger Zone should be visually separate from ordinary editing actions.
  - Mismatch: `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` renders the separate `assessment-workspace-unrelease-danger-zone` section, but `src/pages/assessment_workspace/assessment_workspace.css` still styles the retired `assignment-workspace-unrelease-danger-zone` class, so the intended visual separation is not applied.
- [ ] Assessment Unrelease should explain that Student work will be deleted.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` explains Assignment unrelease, not Assessment Unrelease.
- [ ] Assessment Unrelease should require typing the Assessment title before confirmation.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` requires an Assignment title, not an Assessment title.
- [ ] Archive actions should explain the effect on shared availability and require a clear confirmation.
  - Mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` verifies that behavior only for Archive Blueprint Course; it does not establish the required behavior for every Archive action, including Archive Published Question.
- [x] Restore actions should use ordinary availability controls.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `restore` is presented under Course names and availability rather than the archive confirmation control.
