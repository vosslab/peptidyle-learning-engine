### Instructor interface
- [ ] The Instructor interface should make frequent teaching tasks fast and easy to find.
  - Mismatch: the main Instructor task areas still contain deferred destinations and no end-to-end usability evidence establishes this broad workflow claim.

- [x] Keep the teaching content central in authoring and inspection workflows, with metadata and supporting explanations arranged compactly around it.
  - Evidence (source): `src/pages/question_detail_page.tsx` `aria-label="Question prompt"` renders the prompt before the Question Description and the metadata strip.
  - Evidence (source): `src/pages/question_detail_page.css` `question-detail-description` keeps that explanation compact, with no card padding, fill, or shadow.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_workspace.tsx` `Student-facing prompt` stays in the authoring column, and Question Library metadata sits with the preview.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_styles.ts` `editor-grid` places the preview and library metadata beside the prompt.
  - Evidence (test): `tests/playwright/test_teaching_content_central.mjs` `TEACHING_CONTENT_SENTENCE` rendered the shipped Question inspection page and private Draft editor in headless Chromium at 1280 by 800. The prompt led the description and metadata, and the description had no card padding, fill, or shadow. The Draft preview and Question Description for Instructors sat beside the prompt. The check covers those two workflows and does not measure every authoring or inspection workflow. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Gradebook rows should identify Students by their Course roster names and Coursework by title, with IDs as supporting information where useful.
  - Evidence (source): `src/pages/gradebook_page.tsx` `gradebookRowHeader` shows the roster name with the roster ID in supporting text, and the Coursework cell shows the Assessment title.

- [x] The Instructor menu has **Courses**, **Questions**, and **Assessments** in one dense top bar.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `productAssessments` labels the third top-bar tab Assessments.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Instructor Product routes reserve owner-ordered task groups despite unavailable entries` checks the Courses, Questions, and Assessments tab labels.

- [x] Instructor Profile uses a generic user icon until the **Instructor** adds a Profile image.
  - Evidence (source): `src/features/profile_avatar/ribbon_account_avatar.tsx` `RibbonAccountAvatar` falls back to `RibbonIcon` `circle-user` when no Profile image or provided avatar exists.

- [x] All required Ribbon choices remain visible even when their collection is empty.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_TWO` declares each required Tier 2 choice without reading collection size.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `settled Instructor Tier 2 destinations and order stay fixed across deeper routes` keeps those choices on every Instructor route in the Tier 1 area.

- [x] Required Instructor Ribbon choices whose workflows are unavailable remain visible as unavailable, annotation-free controls. They become usable only when their workflow exists.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `UnavailableRibbonChoice` keeps the choice visible, disabled, and labeled without a navigation href.
  - Evidence (test): `tests/e2e/e2e_ribbon_app_component.mjs` `required Instructor choices remain visible without inventing unfinished routes` checks that unavailable choices stay visible and available choices keep their routes.

- [x] A working navigation destination remains visible when its collection is empty.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` retains the courses route and renders an empty state.

- [x] A future or unavailable capability should not appear as a usable control until its workflow exists.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `RibbonDestination` distinguishes `route` from `future` destinations.

- [x] Empty collection pages should explain what the collection is for and provide an obvious action to create or add the first item when the user can do so.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` explains the shared Published Question collection and shows Create a Draft Question only when mayMutateLibrary is true.

- [x] Similar pages should place similar actions in consistent locations.
  - Evidence (source): `src/components/record_list/record_list.tsx` `record-list__actions` places each record action after the title and facts.
  - Evidence (source): `src/pages/course_list_page.tsx` `Open Course` is the Course list open action in that shared slot.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `label: "Open"` is the Question Library open action in that same slot.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Similar pages should place similar actions in consistent locations.` rendered the shipped My Active Courses and Question Library pages in headless Chromium at 1280 by 800. Open Course and Open both sat in the shared actions slot, below the title and facts, with the same inset from the record content. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Instructor pages should be composed around the teaching task rather than collections of padded components.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_authoring.css` `assessment-editor-panel` keeps the subtle panel radius and removes the padding, fill, and shadow around the teaching task.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_authoring.css` `assessment-editor-actions` places the task buttons in a strip without a card.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace.css` `assessment-editor-policy-panel` keeps Assessment Properties in flat groups.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace.css` `assessment-create-method` names the create choice without a padded box.
  - Evidence (source): `src/pages/course_instance_page.tsx` `course-instance-page__assessments` puts the ordered Assessments ahead of Course administration.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `Ordered Assessment Entries` is the Question editor list ahead of adding Questions.
  - Evidence (test): `tests/playwright/test_teaching_task_pages.mjs` `TEACHING_TASK_SENTENCE` rendered the shipped Course editor, Create Assessment form, and Assessment Question editor in headless Chromium at 1280 by 800. The Assessment list leads the Course page. The create choice and Question editor sections have no card padding, fill, or shadow, and the Question editor panel keeps a subtle radius below the button. The check does not measure every Instructor page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Instructor lists and repeated records should be dense and easy to scan, more like a spreadsheet than cards.
  - Evidence (source): `src/components/record_list/record_table.tsx` `RecordTable` places each Gradebook and roster record on one table row under named columns.
  - Evidence (source): `src/components/record_list/record_list.css` `record-list__row--semantic` keeps each Question Library record a full-width row with a divider and no card radius.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Instructor lists and repeated records should be dense and easy to scan, more like a spreadsheet than cards.` rendered the shipped Gradebook, Course roster, and Question Library in headless Chromium at 1280 by 800, kept those records in one full-width column with square corners, no record shadow, and a hairline divider, and showed several Library titles in the list window. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_student_view_page.tsx` `AssessmentWorkspaceStudentViewPage` renders the answer-free cue, loads the manifest and selected Question, and disables the native response preview.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_student_view.sql` `load_instructor_student_view_question_source` is a stable read of one authorized Question source.
  - Evidence (source): `crates/server/src/assessment_student_view.rs` `assessment_student_view_router` registers only GET routes for the manifest, presentation, and document.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.` opened the shipped Student View in headless Chromium, showed the peptide-bond prompt and the answer-free cue, kept the response preview disabled and inert, and recorded only the workspace read plus the Student View manifest and Question reads. No Save response, Submit, or Start attempt control was present. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Instructor lists and repeated records should favor compact rows or tables with clear columns over cards or loosely concatenated text.
  - Evidence (source): `src/components/record_list/record_table.tsx` `scope="col"` names each Gradebook and roster column.
  - Evidence (source): `src/components/record_list/record_list.css` `repeat(auto-fit, minmax(min(100%, 8rem), 1fr))` places each Question Library fact in an equal column.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Instructor lists and repeated records should favor compact rows or tables with clear columns over cards or loosely concatenated text.` rendered the shipped Gradebook, Course roster, and Question Library in headless Chromium at 1280 by 800, lined each table cell up with its column header, and kept Library facts in equal columns that align across records. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] At 1280 x 800, Instructor pages should expose enough of the current workflow to minimize unnecessary scrolling.
  - Evidence (source): `src/pages/gradebook_page.tsx` `GradebookCoursePage` places the score download and the score table in the Gradebook workflow.
  - Evidence (source): `src/pages/course_roster_page.tsx` `current-roster-heading` places the roster table in the Students workflow.
  - Evidence (source): `src/pages/library_page.css` `grid-column: 2` places the Question Library result window beside the filters at laptop width.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `At 1280 x 800, Instructor pages should expose enough of the current workflow to minimize unnecessary scrolling.` rendered the shipped Gradebook, Course roster, and Question Library in headless Chromium at 1280 by 800 and kept each heading, first record, and workflow action inside that viewport. No Live Demo stack was started. No PostgreSQL proof was run.

#### Course interfaces
- [x] The **Courses** ribbon must include: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `myBlueprintCourses` is one of the four Courses-area route destinations.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Instructor Product routes reserve owner-ordered task groups despite unavailable entries` checks those four Courses labels.

- [x] Course lists should support scanning and comparison without opening each Course.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` puts the Course long name, classification, and term on each list row beside Open Course.

- [x] The Course Editor should show the Course structure and its ordered Assessments without showing every Question at once.
  - Evidence (source): `src/pages/course_instance_page.tsx` `CourseInstancePage` lists the Course and its Assessments in Course order.

- [x] Selecting an Assessment in the Course Editor opens that Assessment for editing.
  - Evidence (source): `src/pages/course_instance_page.tsx` `assessmentQuestionsPath` points Edit Assessment at that Assessment's Question editor.

- [x] Assessment content and Assessment properties should remain separate editing tasks.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `AssessmentWorkspaceQuestionsView` is the Question editor, and `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` is the Properties editor.

- [x] My Active Courses and My Inactive Courses should both be available from the Courses area.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `myActiveCourses` and `myInactiveCourses` are route destinations in the Courses area.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Instructor Product routes reserve owner-ordered task groups despite unavailable entries` checks both labels on the Instructor Courses row.

- [x] Course Discipline selection should provide a clear way to request a new Discipline when the needed
  Discipline is unavailable.
  - Evidence (source): `src/components/course_classification_fields.tsx` `DisciplineRequestControl` asks for a Discipline the Course selector does not offer.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `request_content_discipline` stores the requested name for an Instructor or Sysadmin.
  - Evidence (source): `src/pages/content_disciplines_page.tsx` `ContentDisciplinesPage` lists open requests with the requester Account ID.
  - Evidence (source): `src/components/discipline_request.ts` `createDisciplineFromRequest` calls createDiscipline and then resolveDisciplineRequest.
  - Evidence (test): `tests/test_frontend_contract.mjs` `Course Discipline selection requests a new Discipline without creating it` recorded trimmed Genetics through requestContentDiscipline.
  - Owner: Course interfaces (first occurrence).

##### Blueprint Course interface
- [x] **My Blueprint Courses** should emphasize reusable course design rather than teaching activity.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCoursesWorkspace` presents reusable Blueprint Course content and adoption information.

- [x] **Search Public Blueprint Courses** helps Instructors find relevant Blueprint Courses in a growing shared collection.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` searches public Blueprint Courses by name and classification.

- [x] Public Blueprint Course search should combine ordinary text search with shared classification
  filters beginning with Discipline and following Discipline -> Subject -> Topic -> Subtopic.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` keeps the name field and `BlueprintSearchClassification` on one search form.

- [x] Selecting a Discipline should limit Subject choices to Subjects associated with that Discipline.
  - Evidence (source): `src/pages/blueprint_course_search_classification.tsx` `BlueprintSearchClassification` loads Subject choices for the selected Discipline.

- [x] After selecting a Subject, Instructors should have an explicit option to include Blueprint Courses
  associated with that Subject across its other Disciplines.
  - Evidence (source): `src/pages/blueprint_course_search_classification.tsx` `BlueprintSearchClassification` shows Include this Subject across Disciplines only after a Subject is selected.

- [x] Tags should provide additional filters outside the hierarchy.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` places the Tag field outside `BlueprintSearchClassification`, and `schemas/base_schema/50_functions/blueprint_operations.sql` `list_blueprint_courses` matches that exact tag outside the Discipline predicates.
  - Evidence (test): `crates/server/src/blueprint_course/list.rs` `public_search_tag_binds_the_request_and_rejects_a_different_tag` binds the trimmed tag.

- [x] Search results should use a compact, information-rich layout that supports scanning and comparison.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchResultList` keeps public Blueprint results on the semantic record list.

- [ ] Results should show Course name, classification, author, institution, and useful usage or
  stewardship signals directly in the result list to support scanning and comparison.
  - Mismatch: `src/pages/blueprint_course_search_page.tsx` `publicBlueprintContent` shows the Course name, classification, adoptions, and students. `crates/question_model/src/blueprint_course.rs` `BlueprintCourseSummaryView` has no author or institution field.

- [ ] Public Blueprint Course search should support sorting by relevant fields such as Stars, Watches, Adoptions, Students who have taken the Course, and most recent edit.
  - Mismatch: `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` sorts by name, adoptions, and students. `crates/question_model/src/blueprint_course.rs` `BlueprintCourseSummaryView` has no stars, watches, or last-edit field.

- [x] Search terms, active filters, and the selected sort should remain visible while reviewing results.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` keeps the search form, applied name, tag, classification, and sort beside the results.

- [x] Clearing or changing part of a search should be quick.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` updates the search session on each input or selection change.
  - Owner: Blueprint Course interface (first occurrence).

- [x] Opening a result and returning should preserve the Instructor's search, filters, sort, and scroll
  position.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` restores the saved query, tag, classification, sort, cursor, and scroll.
  - Evidence (source): `src/pages/blueprint_course_search_return_state.ts` `saveBlueprintSearchReturnState` keeps that search snapshot for the same session.
  - Evidence (test): `tests/test_blueprint_search_return.mjs` `takeBlueprintSearchReturnState` returns the saved snapshot once to the same session.

- [x] A **Blueprint Course** should provide an obvious action for creating a **Course Instance** from it.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` renders "Create Course Instance from this Blueprint."
  - Evidence (runtime): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` is exercised by accepted C47 compiled-main browser proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.C6RwpH/public-search-browser.json`, which follows the existing detail action and preselects the matched Blueprint beyond the first 50 search rows. It does not submit or create a Course Instance.

##### Blueprint Course editing interface
- [x] Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_detail_workspace.tsx` `onToggleEditor` opens the Course Editor from the overview and clears any selected Assessment.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_detail_workspace.tsx` `selectAssessment` opens that Assessment's editor from the Course Editor list.

- [x] The Course Editor should show the Blueprint Course structure without editing every Question on one page.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` lists modules and Assessments before selecting an editor.

- [x] Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor` receives the selected Assessment content and renders its Questions and Properties tasks.
  - Evidence (runtime): accepted compiled-main, actual-loopback-HTTP evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.ay40bT/blueprint-properties-browser.json` opens one selected Assessment and exercises both tasks through `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor`.

- [x] Only the selected Blueprint Assessment's Questions should appear in its editor.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor` renders entries from its selected `content` input only.
  - Evidence (runtime): accepted compiled-main, actual-loopback-HTTP evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.ay40bT/blueprint-properties-browser.json` verifies one selected Assessment, its separated Questions task, and an unchanged sibling after ordinary Save and exact reload through `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor`.

- [x] **Blueprint Assessment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assessment.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor` selects through the Question picker, adds with confirmFixedQuestions, removes with removeEntry, and orders through its RecordSequence.

- [x] **Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `updateReusableEntryScoring` sets points and the scoring rule beside the attempt limit, late work, and Student feedback fields.

- [x] Blueprint Courses should not contain Assessment dates or relative Assessment schedules.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` describes reusable structure without Students, deadlines, or course delivery settings; the reusable Blueprint content model has no delivery-date fields.

##### Course Instance interface
- [x] **My Active Courses** should emphasize Course Instances the Instructor is currently teaching.
  - Evidence (source): `src/pages/role_home_pages.tsx` `InstructorHomePage` opens the Active Course list, and `src/pages/course_list_page.tsx` `TeachingCourseListPage` keeps that home to Active Course Instances.

- [x] Active Course Instances should make upcoming Assessments and important course activity easy to find.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` lists upcoming Assessment titles, Courses, and due times for the Courses the Instructor teaches, and `src/ribbon/ribbon_catalog.ts` `assessmentsDueSoon` opens that page from the Assessments ribbon.

- [x] **My Inactive Courses** should keep past Course Instances available without competing with active Course Instances.
  - Evidence (source): `src/pages/course_list_page.tsx` `InactiveCourseListPage` lists only Inactive Course Instances, separate from the Active home.

- [x] Creating a Course Instance from a Blueprint Course preserves its Assessments, Questions, pools, and settings.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `validate_course_blueprint_adoption` requires adopted Assessments, Questions, pools, and settings to match the exact Revision.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs` `assert_adoption_projection` checks that adopted Course against the sealed Revision.

- [x] Assessments created from a Blueprint Course start unreleased with dates unset.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `append_course_assessments` stores null available, due, and close instants and leaves the Assessment unreleased.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs` `assert_adoption_projection` checks the unreleased status and the three null dates.

- [x] A Course Instance represents one teaching period and remains Active for at most six months from
  creation.
  - Evidence (source): `crates/question_model/src/course_term.rs` `ensure_within_active_lifetime` rejects a term end after the UTC calendar date six months from creation and accepts that cutoff date.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` rejects a term end after that same UTC cutoff.
  - Evidence (source): `src/pages/course_list_page.tsx` `createCourseInstance` refuses that end date before creating the Course.
  - Evidence (test): `crates/question_model/src/course_term.rs` `active_lifetime_includes_the_cutoff_date_and_rejects_the_next_day` accepts the cutoff date, including a term longer than six months that ends on it, and rejects the next day.

- [x] Course banners use a 5:1 aspect ratio.
  - Evidence (source): `src/features/course_appearance/course_entry_banner.tsx` `aspect-ratio: 5 / 1` sizes the Course entry banner frame.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `the shipped course entry banner keeps a 5:1 ratio as its width changes` measures that rendered frame.

- [x] 1280 by 256 pixels is the recommended Course banner authoring size.
  - Evidence (source): `src/pages/course_appearance_page.tsx` `AppearanceBannerEditor` tells the Instructor that 1280 by 256 pixels is recommended.

- [x] Higher-resolution 5:1 Course banner images are supported.
  - Evidence (source): `crates/objects/src/image_validation.rs` `verify_course_banner_still_image` accepts an exact 5:1 source with no minimum size.
  - Evidence (source): `crates/server/src/course_appearance/banner.rs` `stage_banner_upload` stages that verified source, and promotion scales it to the delivery rendition.
  - Evidence (test): `crates/objects/src/image_validation.rs` `course_banner_accepts_a_higher_resolution_five_to_one_source` accepts a source larger than the delivery rendition and scales it to that rendition.

- [x] PLE responsively scales Course banners while preserving their aspect ratio.
  - Evidence (source): `src/features/course_appearance/course_entry_banner.tsx` `CourseEntryBanner` frames the banner with `aspect-ratio: 5 / 1` and a viewport-bounded inline size.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `the shipped course entry banner keeps a 5:1 ratio as its width changes` measures the frame at three viewport widths.

- [x] Course banners appear as small centered banners rather than full-width page heroes.
  - Evidence (source): `src/features/course_appearance/course_entry_banner.tsx` `course-entry-banner-frame` caps the banner and centers it with `margin-inline: auto`.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `the shipped course entry banner keeps a 5:1 ratio as its width changes` checks that a wide viewport leaves the banner narrower than the page and centered.

- [x] An Instructor can upload a Course banner and select a Course Theme from the fixed theme catalog.
  - Evidence (source): `src/pages/course_appearance_page.tsx` `saveBanner` uploads the chosen Course banner from the Instructor appearance page.
  - Evidence (source): `src/appearance/theme_chooser.tsx` `THEME_OPTIONS` offers every theme in the fixed catalog.
  - Evidence (test): `tests/playwright/course_appearance_m8_evidence.mjs` `bannerUploadCalls` saves a banner after the Instructor selects Ocean and Magma.

- [x] Course Instance Assessments have two editors:
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_overview_page.tsx` `Assessment Question Editor` links to the Question editor and `Assessment Properties Editor` links to the properties editor.

  - [x] **Assessment Question Editor**: Selects, adds, removes, and orders Questions in an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `Assessment Question Editor` is the Question editor title.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `appendAvailableFixedQuestion` adds a selected Question, `removeAssessmentEntry` removes one Entry, and `moveAssessmentEntry` reorders Entries.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `appendAvailableFixedQuestion` adds a selected Published Question.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `removeAssessmentEntry` removes only the chosen Entry.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `moveAssessmentEntry` changes Entry order.

  - [x] **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and what **Students** can see.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `Assessment Properties Editor` edits dates, attempt limit, late work, and Student feedback.
  - Evidence (source): `src/pages/assessment_workspace/assessment_fixed_question_points_editor.tsx` `AssessmentFixedQuestionPointsEditor` edits fixed-Question points on that page.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `assessmentPolicySaveInput` keeps dates and saves the attempt limit, late-work rule, and Student feedback release.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `withFixedQuestionPointValues` changes only the chosen fixed-Question points.

#### Question interface
- [x] The **Questions** ribbon must include: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `RIBBON_TASK_CATALOG` includes those six Questions-area labels.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Instructor Product routes reserve owner-ordered task groups despite unavailable entries` checks those six Questions labels.

- [x] **My Questions** should make the Instructor's Published Questions easy to find and manage.
  - Evidence (source): `src/pages/my_questions_page.tsx` `MyQuestionsPage` lists the signed-in Instructor's Published Questions, links each one to its Question page, and pages them with RecordPageControls.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `myQuestions` opens that list from the Questions ribbon.
  - Evidence (test): `tests/test_library_classification_search.mjs` `My Questions continues the authored library page` loads the next authored page at 50, 100, or 250 and renders Previous, Next, and Records per page.

- [x] **My Draft Questions** should emphasize Questions that still need work before publication.
  - Evidence (source): `src/pages/question_drafts_page.tsx` `QuestionDraftsPage` presents the current Instructor's drafts.

- [x] **Starred** should provide a quick personal collection of Questions the Instructor wants to keep handy.
  - Evidence (source): `src/pages/starred_questions_page.tsx` `StarredQuestionsPage` lists the signed-in Instructor's Starred Questions and links each one to its Question page.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `list_current_starred_questions` returns that Instructor's Stars, newest first.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `starred` opens that collection from the Questions ribbon.

- [x] **Watched** should help Instructors follow Questions where changes or activity matter to them.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `LibraryWatchNotificationsPage` lists changes and stewardship activity for Published Questions the Instructor watches.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `watched` opens that inbox from the Questions ribbon.
  - Evidence (test): `tests/test_library_watch_notification_client.mjs` `getLibraryWatchNotifications` returns Revision, fork, and stewardship activity for a watched Question.

- [x] Published Questions should offer a **Create Pool from Question** action.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionPoolFromQuestionControl` offers Create Pool from Question on a Published Question.

- [x] Pool creation should show the starting Question and its Discipline and Subject alongside the
  Pool Title field.
  - Evidence (source): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` shows the starting Question title, Discipline, and Subject beside Pool Title.

- [x] Creating the Pool includes the starting Question and uses its Discipline and Subject.
  - Evidence (source): `src/components/question_pool_create_model.ts` `questionPoolMemberTuples` places the starting Question first and requires its Discipline and Subject.
  - Evidence (test): `tests/test_question_pool_source_binding.mjs` `questionPoolMemberTuples` keeps the starting Revision first.

- [x] Adding Questions to a Pool should begin with Question Library results filtered to the Pool's
  Discipline and Subject.
  - Evidence (source): `src/components/question_pool_create_model.ts` `questionPoolSourcePickerRepository` keeps Question Library rows from the starting Discipline and Subject.
  - Evidence (test): `tests/test_question_pool_source_binding.mjs` `questionPoolSourcePickerRepository` requests the starting Subject and drops a different Discipline.

##### Search Question Library interface
- [x] **Search Question Library** helps Instructors find specific Questions in a large library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` supplies the searchable published Question Library.

- [x] Search should begin with a prominent search box, similar to Google Search.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` starts in `initial` state and applies `question-library-controls-initial` to the single `Search published questions` entry.
  - Evidence (runtime): one-time accepted compiled-browser exercise of `src/pages/library_page.tsx` `LibraryPage` confirmed idle Search makes no fetch and shows neither filters nor result rows; the temporary harness and landing screenshot were removed after the accepted proof.

- [x] The initial Search page should stay simple and focus attention on entering a search.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders filters and results only outside `initial` state, while `changeQuery` begins the current Search after input.
  - Evidence (runtime): one-time accepted compiled-browser exercise of `src/pages/library_page.tsx` `LibraryPage` confirmed the initial single-entry landing, then filters and 50 loaded results after entering `genetics`; the temporary harness and screenshots were removed after the accepted proof.

- [x] Search should assume the Instructor has some idea what they want to find.
  - Evidence (source): `src/pages/library_page.tsx` search input placeholder `Title or concept` directs a known-item search.

- [x] Question Library search should work well with ordinary words by default.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` passes ordinary `search` text to the Question Library query.

- [x] Instructors should not need to learn search syntax to use Search Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` exposes ordinary search and labeled filter controls without syntax requirements.

- [x] Search results should switch to a dense, information-rich layout.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` shows the published Question list only after Search leaves initial.
  - Evidence (source): `src/components/record_list/record_list.tsx` `RecordList` keeps those results on the semantic list unless a caller asks for gallery.

- [x] Results should make it easy to scan many Questions quickly.
  - Evidence (source): `src/pages/library_browse_record_list.css` `library-browse-record-list__window` keeps the current result page in one scroll region.

- [x] Results should show the information needed to judge relevance without opening each Question.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `questionDetails` shows authors, Discipline, Bloom, format when present, and the Question ID on each result.

- [x] Search terms and active filters should remain visible while reviewing results.
  - Evidence (source): `src/pages/library_page.tsx` `query` signal remains bound to the search input and filter selects while rows render.

- [x] Clearing or changing part of a search should be quick.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` updates the search session on each input or selection change.
  - Owner: Blueprint Course interface (first occurrence).

- [ ] Should opening a result and returning preserve the Instructor's search and position.
  - Reason: product decision still unclear
  - Question: Must Question Library restore the Instructor's search and scroll after opening a result, or is that still an open design choice?
  - Mismatch: `src/pages/library_page_model.ts` `takeQuestionLibraryReturnState` already restores one saved query and scroll position. One reading treats that as required. The other keeps this sentence as an open question beside the hover-preview and new-tab note, so Human Guidance has not locked the requirement.

  - [ ] we should offer some hover preview and open items in a new browser tab by default
    - Reason: product decision still unclear
    - Question: Should a Question Library result show a hover preview and open in a new browser tab by default?
    - Mismatch: Question links open in the same tab and have no hover preview. One reading requires both. The other treats the sentence as an unsettled design note rather than a requirement.

- [ ] Advanced Search considerations:
  - Reason: product decision still unclear
  - Question: Does this bullet require a separate advanced-search mode, or does it only introduce the comparison notes that follow?
  - Mismatch: Question Library has one search box plus filters. One reading requires another advanced-search mode. The other treats the bullet as a heading for the notes below.

  - [ ] Simple and advanced searches could use the same search box, but we should seriously consider
    advanced search versus simple search interfaces forms.
    - Reason: product decision still unclear
    - Question: Should simple and advanced Question Library search share one box, or use separate forms?
    - Mismatch: `src/pages/library_page.tsx` `LibraryPage` uses one search box. One reading keeps that box. The other asks for a separate advanced form. Human Guidance says to consider both.

  - [ ] The interface should be minimal, show options by priority, not overwhelming to new users;
    - Reason: product decision still unclear
    - Question: Does this require a new minimal advanced-search form, or does the existing simple Search page already satisfy it?
    - Mismatch: The initial Search page is one search box. One reading asks for a further prioritized advanced form. The other treats the sentence as a design note under Advanced Search considerations.

  - [ ] Movie Lens as a tiered filter system https://movielens.org/explore/
    - Reason: product decision still unclear
    - Question: Must Question Library filters copy the MovieLens tiered explore layout?
    - Mismatch: No MovieLens-style explore layout is shipped. One reading requires that layout. The other treats the link as a comparison note, not a requirement to copy the product.

  - [ ] IMDB advanced search page is wel designed, https://www.imdb.com/search/title/ but questions
    would not be displayed as movie posters
    - Reason: product decision still unclear
    - Question: Must Question Library search copy the IMDb advanced title search, while keeping Questions off poster cards?
    - Mismatch: Results use a semantic list rather than posters. One reading requires an IMDb-style advanced form. The other treats the sentence as a comparison note.

  - [ ] Google advanced image search is more user friendly design https://www.google.com/advanced_search
    - Reason: product decision still unclear
    - Question: Must Question Library search copy Google advanced search?
    - Mismatch: No Google advanced-search form is shipped. One reading requires that form. The other treats the link as a comparison note.

  - [ ] Pubmed is clean, but not obvious to use https://pubmed.ncbi.nlm.nih.gov/advanced/
    - Reason: product decision still unclear
    - Question: Must Question Library search copy the PubMed advanced-search page?
    - Mismatch: No PubMed advanced form is shipped. One reading requires that page. The other treats the link as a comparison note.

  - [ ] Ebay is dated, but perhaps a useful comparison https://www.ebay.com/sch/ebayadvsearch
    - Reason: product decision still unclear
    - Question: Must Question Library search copy eBay advanced search?
    - Mismatch: No eBay advanced-search form is shipped. One reading requires that form. The other treats the link as a comparison note.

  - [ ] Should Question IDs have a preview image/movie poster style?
    - Reason: product decision still unclear
    - Question: Should a Question ID in search results include a preview image, or stay text?
    - Mismatch: `src/components/copyable_question_id.tsx` `CopyableQuestionId` shows the text Question ID. One reading adds a poster image. The other keeps the text ID.

##### Search Question Library filters
- [x] Search results should support filters for narrowing the Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` supplies author, backend, tag, Question Type, license, and capability filters.

- [x] Classification browsing and filtering should begin with Discipline and follow the shared
  Discipline -> Subject -> Topic -> Subtopic hierarchy.
  - Evidence (source): `src/components/library_classification_search.tsx` `LibraryClassificationSearch` offers Discipline, then Subject, Topic, and Subtopic.

- [x] Tags should provide additional filters outside the hierarchy.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` places the Tag field outside `BlueprintSearchClassification`, and `schemas/base_schema/50_functions/blueprint_operations.sql` `list_blueprint_courses` matches that exact tag outside the Discipline predicates.
  - Evidence (test): `crates/server/src/blueprint_course/list.rs` `public_search_tag_binds_the_request_and_rejects_a_different_tag` binds the trimmed tag.
  - Owner: Blueprint Course interface (first occurrence).

- [x] Selecting a Discipline should limit Subject choices to Subjects associated with that Discipline.
  - Evidence (source): `src/components/library_classification_search.tsx` `LibraryClassificationSearch` loads Subject choices for the selected Discipline.
  - Owner: Blueprint Course interface (first occurrence).

- [x] After selecting a Subject, Instructors should have an explicit option to include Library Objects
  associated with that Subject across its other Disciplines.
  - Evidence (source): `src/components/library_classification_search.tsx` `LibraryClassificationSearch` shows Include this Subject across Disciplines only after a Subject is selected.

- [x] Filters should update the current search rather than start a separate workflow.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` resets one `QuestionLibraryBrowseSession` with the updated query.

##### Search Question Library syntax
- [x] Search should support Google-like syntax for more precise queries.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `pub(super) struct QuestionTextQuery` accepts ordinary AND words, quoted phrases, minus exclusions, PLE field tags, and exact Question IDs with filters through its parser.
  - Evidence (runtime): accepted one-time private PostgreSQL and actual-server HTTP proof exercised `crates/server/src/question_library.rs` `search_questions` with ordinary AND words, a quoted phrase, minus exclusion, and all five PLE fields under an active vetted Instructor; anonymous and Student requests remained concealed and responses were `no-store`. The temporary proof is retained outside Git through documentation acceptance.

- [x] Quoted text should search for an exact phrase.
  - Evidence (source): `crates/server/src/library_search_terms.rs` `term_value` retains quoted text as one exact phrase term.
  - Evidence (runtime): historical C58 actual-server HTTP proof returned only `Alpha enzyme kinetics` for the quoted phrase `"enzyme kinetics"`; current locator `crates/server/src/library_search_terms.rs` `term_value` records the extracted behavior.

- [x] A minus sign should exclude matching terms.
  - Evidence (source): `crates/server/src/library_search_terms.rs` `exclusion_prefix` records a leading minus as an excluded search term.
  - Evidence (runtime): historical C58 actual-server HTTP proof returned `Alpha enzyme kinetics` for `enzyme -inhibitor` while excluding the matching inhibitor Question; current locator `crates/server/src/library_search_terms.rs` `exclusion_prefix` records the extracted behavior.

- [x] Search should support PubMed-like field syntax such as `discipline:biology` and
  `subject:genetics`.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` parses discipline and subject prefixes into store terms.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `pubmed_style_fields_round_trip_through_the_store_terms` keeps `discipline:biology` and `subject:genetics` from the query text.

- [x] Classification field examples include `topic:"chromosomal inheritance"` and `tags:review`.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` parses topic and tags prefixes into store terms.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `pubmed_style_fields_round_trip_through_the_store_terms` keeps `topic:"chromosomal inheritance"` and `tags:review` from the query text.

- [x] A Subtopic field example is `subtopic:"x-linked recessive crosses"`.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` parses the subtopic prefix into a store term.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `pubmed_style_fields_round_trip_through_the_store_terms` keeps `subtopic:"x-linked recessive crosses"` from the query text.

- [x] Search fields should use PLE concepts and vocabulary.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` accepts discipline, subject, topic, subtopic, tags, type, and author.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `pubmed_style_fields_round_trip_through_the_store_terms` renders those prefixes back from the store terms.

- [x] Useful fields may include Discipline, Subject, Topic, Subtopic, Tags, Question Type, and author.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` maps type and author onto Question Type and author store fields.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `pubmed_style_fields_round_trip_through_the_store_terms` keeps `type:"multiple choice"` and `author:Ada` with the classification fields.

- [x] The interface should make useful search syntax discoverable when needed.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders `question-library-search-tips` as a native disclosure beside the ordinary Search box with words, quotes, minus, PLE fields, and examples.
  - Evidence (runtime): accepted corrected desktop `src/pages/library_page.tsx` `LibraryPage` component proof opened Search tips without obscuring filters or bulk controls; the full `./check_codebase.sh` gate passed.

- [ ] Search syntax should help expert users quickly narrow a very large Question Library.
  - Evidence (runtime): accepted C58 actual-server HTTP evidence proves the grammar through the production Store and route across a bounded 69-Question fixture.
  - Verification pending: the bounded proof does not establish usability or performance for a very large production Question Library.

##### Browse Question Library interface
- [x] **Browse Question Library** helps Instructors explore Questions without knowing what to search for.
  - Evidence (source): `src/route_contract.ts` `libraryBrowse` is a distinct Instructor route and `src/pages/library_page.tsx` `LibraryPage` provides an overview-first grouped Browse mode without requiring Search text.
  - Evidence (runtime): accepted private full-app browser evidence exercised `src/main.tsx` `render` against the actual server through overview-first Browse, Biology, Enzymes, and focused Search without starting from Search text; every actual search response was `no-store`.

- [x] Browse should help Instructors understand what the Question Library contains.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` explains that counts cover all authorized matching Questions and presents subject, topic, tag, and Question Type groups.
  - Evidence (runtime): accepted C60 component evidence rendered `src/pages/library_page.tsx` `LibraryPage` overview groups; actual-server HTTP evidence exercised `crates/server/src/question_library.rs` `search_questions` over the full authorized matched snapshot.

- [x] Browse should begin with Discipline and make moving through Subject, Topic, and Subtopic easy.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` places `LibraryClassificationSearch` at the start of Browse.

- [x] Selecting a Discipline limits browsing to Subjects associated with that Discipline.
  - Evidence (source): `src/components/library_classification_search.tsx` `LibraryClassificationSearch` loads Subject choices for the selected Discipline.

- [x] Browse should also offer Tags, Question Types, and other useful groupings.
  - Evidence (source): `src/pages/library_browse_controls.tsx` `LibraryBrowseControls` offers Subject, Topic, Tags, and Question Types groups.

- [x] Browse should show useful counts where they help Instructors choose where to explore.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` group choices render their server-owned counts and state that counts cover all authorized matches, not only loaded rows.
  - Evidence (runtime): accepted actual-server HTTP evidence exercised `crates/server/src/question_library.rs` `search_questions` with `page_size=1` yet returned Biology topic counts of Enzymes 2 and Metabolism 1 over all three matching Questions.

- [x] Browse results should use the same dense Question presentation used by Search where practical.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` gives Search and Browse the same virtualized `question-library-row` result renderer.
  - Evidence (runtime): accepted private full-app browser evidence rendered `src/pages/library_page.tsx` `LibraryPage` at 1280 by 800 with two shared dense Question rows, and root manager visual review accepted the initial Browse, narrowed Browse, and Search handoff screenshots.

- [x] Instructors should be able to move from browsing into a more focused search.
  - Evidence (source): `src/pages/library_page.tsx` `searchWithinResultsPath` serializes exact subject, topic, tag, and Question Type filters into the Search route.
  - Evidence (runtime): accepted routed component evidence exercised `src/pages/library_page.tsx` `LibraryPage` and preserved Biochemistry and Enzymes visibly and in subsequent repository requests through text editing and detail return, even when response facets omitted selected values.

- [x] Search and Browse are different paths into the same **Question Library**.
  - Evidence (source): `src/route_contract.ts` `ROUTE_CONTRACT` defines `library` and `libraryBrowse` as distinct Instructor routes backed by the same repository and `LibraryPage` row implementation; the Ribbon Browse task targets `libraryBrowse`.
  - Evidence (test): `tests/test_ribbon_route_contract.mjs` `declared routes select only Ribbon topology and task areas that exist` and focused Ribbon, screenshot-manifest, and picker checks passed after the distinct Browse route was added.

#### Assessment interface
- [x] The **Assessments** ribbon must include: Assessments Due Soon, My Assessment Templates.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `assessmentsDueSoon` and `assessmentTemplates` are admitted route destinations with the required labels.
  - Evidence (runtime): accepted independent `src/ribbon/app_ribbon.tsx` `AppRibbon` and `src/ribbon/ribbon_contract.ts` `deriveRibbonModel` proof verified both actual Ribbon choices and routes.

- [x] **Assessments Due Soon** should emphasize Assessments that may need the Instructor's attention.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` presents upcoming deadlines across Courses the Instructor teaches.
  - Evidence (runtime): accepted private full-app evidence exercised `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` against actual HTTP and visibly emphasized the seven-day Account-zone window, release state, and Due group for each upcoming Assessment.

- [x] Assessment lists should make Course, release status, due date, and other important state easy to scan.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `DueSoonAssessmentRow` presents each Assessment with its Course, release state, and Account-zone Due; `src/pages/course_instance_page.tsx` `AssessmentRow` presents title, release state, and readable Account-zone Due under the Course heading.
  - Evidence (runtime): accepted private actual-server and exact-main browser proof exercised `src/pages/course_instance_page.tsx` `CourseInstancePage` alongside Due Soon. The first owned Course retained three readable rows at 1280px and 720px in a Los Angeles browser with a Chicago Account zone; a second owned Course showed three Assessment rows with Released/Unreleased states and distinct Due values matched row-by-row to its actual 200/`no-store` Assessment-list response. The Due Soon list separately showed Course identity on each cross-Course row. The fixture used privileged projection state for Released rows, not the release workflow.

- [x] **My Assessment Templates** should emphasize reusable Assessment design rather than Course activity.
  - Evidence (source): `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface` foregrounds reusable Assessment settings and excludes Questions, Pools, and Course dates.
  - Evidence (runtime): accepted independent `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface` proof verified the heading, lede, legend, and normal Ribbon route without claiming HTTP, CRUD, or copy workflow acceptance.

- [x] Assessment editing has two editors:
  - Evidence (source): `src/route_contract.ts` `assessmentWorkspaceQuestions` and `assessmentWorkspacePolicies` declare distinct Assessment Question and Properties routes; the Ribbon tasks and breadcrumb use the same names.
  - Evidence (runtime): accepted private exact-main browser evidence navigated from `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` into the independently rendered Properties Editor.

  - [x] **Assessment Question Editor**: Selects, adds, removes, and orders Questions.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` provides Add, Move, Remove, and Save controls.
  - Evidence (runtime): accepted private actual-HTTP and exact-main browser evidence exercised `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage`, adding two exact Questions, moving, removing, re-adding, saving, and reloading them in the persisted visible order.

  - [x] **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and other Assessment settings.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` provides dates, instructions, Attempt/time limits, late work, order, disclosure, and focused fixed-Question point-value editing through `AssessmentFixedQuestionPointsEditor`.
  - Evidence (runtime): accepted private actual-HTTP and exact-main browser evidence exercised `src/pages/assessment_workspace/assessment_fixed_question_points_editor.tsx` `AssessmentFixedQuestionPointsEditor`: it saved `2.5` and `1` for two ordered exact Questions, reloaded the persisted values, preserved the other full-Assessment fields, exercised Cancel and Stay/Discard, and recovered from a real concurrent-write conflict by explicitly reloading and discarding the retained point draft.

- [x] The two Assessment editors should remain clearly distinct.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` and the Properties page have separate canonical routes, headings, Ribbon tasks, state models, and save operations.
  - Evidence (runtime): accepted private exact-main browser evidence showed `src/ribbon/ribbon_catalog.ts` `assessmentPolicies` as a distinct selected task, with Question order and Properties instructions each surviving their own actual-HTTP reload.

- [x] The Assessment Question Editor should make Question order easy to understand at a glance.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` renders the ordered entries as one numbered list with adjacent Move and Remove controls.
  - Evidence (runtime): accepted private actual-HTTP and exact-main browser evidence exercised `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` with two visibly numbered Questions, changed their order, and reloaded the persisted order; root manager visual review and independent review accepted the rendered order.

- [x] Adding Questions should provide direct paths to Search and Browse Question Library.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` now renders direct Search and Browse Question Library links inside Available published Questions.
  - Evidence (runtime): accepted private actual-HTTP and exact-main browser evidence exercised `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` through both direct paths and browser Back; the unsaved-changes guard preserved a title draft on Stay and required deliberate Discard before navigation.

- [x] Instructors should be able to inspect a Question before adding it to an Assessment.
  - Evidence (source): `src/features/question_picker/question_picker_model.ts` `inspectQuestionPickerRow` returns the current selection and the result Revision without adding the Question.
  - Evidence (source): `src/features/question_picker/question_picker.tsx` `inspectQuestionPickerRow` opens that inspection inside the picker, and Confirm adds only the selection.
  - Evidence (source): `src/features/question_picker/question_picker_inspection.tsx` `QuestionPickerInspection` shows the answer-free prompt and states that inspection does not add the Question.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `getQuestionRevision` loads that inspection for the Assessment Question Editor.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `loadQuestionInspection` passes the same inspection into the Assessment picker and the Pool picker.
  - Evidence (source): `src/api/decoders/question_library.ts` `decodeQuestionDetails` rejects source and grading fields.
  - Evidence (test): `tests/test_question_picker.mjs` `inspectQuestionPickerRow keeps the current selection and QuestionPickerInspection shows the prompt without adding the Question` kept 7K3M-79QP selected, rendered Enzyme active site, and withheld Add selected Questions and Add pinned Question.

- [x] Instructors should be able to quickly add questions by Question ID to an assessment in bulk.
  - Evidence (source): `src/pages/assessment_workspace/assessment_question_id_batch.ts` `parseQuestionIdBatch` accepts a paste of canonical Question IDs and rejects any token that is not a Question ID before lookup.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `addQuestionsById` resolves each ID with `resolveQuestion` and adds only an available current Revision, within the 250 Question limit.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `Add by Question ID` is the paste control beside Choose published Questions.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `Question ID paste adds the canonical batch and withholds invalid or missing IDs` collapsed a repeated `7K3M-79QP`, withheld missing `2R5X-E7YA`, refused `not-an-id` before lookup, and refused a paste larger than the remaining capacity.

- [x] Assessment Properties should group related settings so important settings are easy to find.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` groups Assessment and delivery controls separately from Student feedback, and the owning CSS uses a two-column desktop grid that collapses to one column below 60rem.
  - Evidence (runtime): accepted private exact-main browser evidence rendered `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` at 1280 by 800; computed-style checks verified the two-column desktop and one-column 720px layouts, restored panel/control styling, and persisted edited instructions through actual HTTP and reload.

- [x] Present timing settings in familiar units such as minutes, with explicit units and clear
  meanings for optional or unlimited values.
  - Evidence (source): `src/assessment_duration.ts` `assessmentDurationDefaultDescription` states the calculated default in minutes.
  - Evidence (source): `src/assessment_duration.ts` `OPTIONAL_DURATION_OVERRIDE_GUIDANCE` names a blank duration override, `NO_OPENING_TIME_GUIDANCE` names a blank opening time, `NO_CLOSING_TIME_GUIDANCE` names a blank close, and `UNLIMITED_ATTEMPTS_GUIDANCE` names a blank Attempt limit.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` shows those meanings beside the timing controls. A Quiz or Exam keeps one Attempt.
  - Evidence (test): `tests/test_assessment_duration.mjs` `timing settings name minutes, a blank override, no closing time, and unlimited Attempts` checked the minute default and those blank meanings, and checked that the properties page renders the guidance constants.

- [x] Instructors can randomize Question order for an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `updateOrder` binds the current Assessment Properties checkbox to authored or shuffled Question order. The Student start transaction persists that rule with the Attempt and shuffles the complete fixed-and-Pool issued vector only for `shuffled`.
  - Evidence (runtime): accepted private actual-HTTP evidence exercised `crates/learning-data-access/src/postgres/assessment_delivery_start.rs` `start_current_assessment_attempt`: it persisted authored and shuffled rules, observed a concurrent Instructor save blocked behind the Student-start lock, issued exact fixed-and-Pool pins, and resumed the immutable shuffled Attempt after a current-rule edit. Accepted exact-main browser evidence saved and reloaded `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`'s visible **Randomize question order** checkbox through actual HTTP.

- [x] Answer-choice randomization belongs to the Question, not the Assessment.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_model.ts` `setChoiceRandomization` and the strict Question codec own `randomizeChoices`; `crates/adapters/ple/src/question_json/source_document.rs` `compile_choices` compiles it to `NativeChoiceOrder`, and `crates/question_model/src/presentation/builder.rs` `pending_items` applies its nonce-derived choice permutation. The closed Assessment activity rules contain Question-order policy but no answer-choice override.
  - Evidence (runtime): accepted exact-main browser evidence preserved `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`'s explicit Question-owned choice-order explanation while saving and reloading the independent Assessment Question-order rule. This verifies the ownership boundary without claiming a runtime matrix of every native choice permutation.

- [x] **Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` calls `listAssessmentsDueSoon` and states its across-Courses scope.
  - Evidence (runtime): accepted private actual-server evidence exercised `schemas/base_schema/50_functions/assessment_operations.sql` `list_assessments_due_soon` across two owned Courses and one outsider Course; each Instructor saw only their own Course rows, while anonymous and Student requests received the same concealed response.

- [x] Assessments Due Soon shows the Course and due time for each Assessment.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `dueSoonContent` shows the Course long name and the formatted due time on each row.

#### Assessment type appearance
- [x] Each Assessment Type has its own PLE-defined Font Awesome icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` assigns one required Font Awesome icon name and bundled glyph to every canonical Type.

- [x] Assessment Type icons remain consistent across PLE themes.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` owns the theme-independent Type-to-icon mapping.

- [x] Each Assessment Type also has its own theme-defined color.
  - Evidence (source): `src/styles/assessment_types.css` `--ple-assessment-type-regular-assignment` defines the five semantic Type color properties at the root and Course-theme scope.

- [x] Themes may change Assessment Type colors but preserve the meaning of each Type.
  - Evidence (source): `src/styles/assessment_types.css` `html[data-display-mode="dark"]` changes the five Type color values.
  - Evidence (source): `src/assessment_type_presentation.ts` `assessmentTypePresentation` keeps the Type icon and label when those colors change.

- [x] Assessment Type should never be communicated by color alone.
  - Evidence (source): `src/components/record_list/record_list.tsx` `RecordFactView` shows the Type icon and label inside the colored Type fact.

- [x] Icons and labels should remain sufficient to identify the Assessment Type without color.
  - Evidence (source): `src/assessment_type_presentation.ts` `assessmentTypePresentation` returns the canonical Type label and bundled icon metadata independently of color.

- [x] **Weekly Assignment** uses the Font Awesome `pen-to-square` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Weekly Assignment to `pen-to-square`.

- [x] **Unit Review Assignment** uses the Font Awesome `arrows-spin` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Unit Review Assignment to `arrows-spin`.

- [x] **Bonus Assignment** uses the Font Awesome `star` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Bonus Assignment to `star` and the genuine bundled glyph.

- [x] **Quiz** uses the Font Awesome `circle-question` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Quiz to `circle-question` and the genuine bundled glyph.

- [x] **Exam** uses the Font Awesome `file-signature` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Exam to `file-signature` and the genuine bundled glyph.

#### High-consequence actions
- [x] Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionArchiveControl`, `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`, and `src/features/blueprint_course/blueprint_course_lifecycle_controls.tsx` `BlueprintCourseLifecycleControls` place those three actions in Danger Zone controls.

- [x] Danger Zone should be visually separate from ordinary editing actions.
  - Evidence (source): `src/pages/question_detail_page.css` `question-archive-danger-zone`, `src/pages/assessment_workspace/assessment_workspace.css` `assessment-workspace-unrelease-danger-zone`, and `src/features/blueprint_course/blueprint_course.css` `blueprint-course-archive-danger-zone` each use the danger border and danger-tinted background.

- [x] Assessment Unrelease should explain that Student work will be deleted.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `assessment-unrelease-confirmation-help` says Unrelease permanently deletes the represented Student Work, and the action is labeled "Unrelease and delete Student Work."

- [x] Assessment Unrelease should require typing the Assessment title before confirmation.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `confirmationTitle` disables Unrelease unless the entered value exactly matches the current Assessment title, while `schemas/base_schema/50_functions/unrelease.sql` `ple_api.unrelease_assessment` independently rejects a nonmatching confirmation title.

- [x] Archive actions should explain the effect on shared availability and require a clear confirmation.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionArchiveControl` explains that archiving removes the Published Question from new selection and disables confirmation until the exact title, and `src/features/blueprint_course/blueprint_course_lifecycle_controls.tsx` `BlueprintCourseLifecycleControls` does the same for the Blueprint Course long name.

- [x] Restore actions should use ordinary availability controls.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_lifecycle_controls.tsx` `BlueprintCourseLifecycleControls` presents Restore alongside ordinary availability actions, without the Archive name-confirmation input; `src/features/blueprint_course/blueprint_course_detail_workspace.tsx` `restore` performs the existing authorized availability change.
