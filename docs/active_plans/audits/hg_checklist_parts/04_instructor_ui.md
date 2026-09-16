### Instructor interface

- [ ] The Instructor interface should make frequent teaching tasks fast and easy to find.
  - Mismatch: the main Instructor task areas still contain deferred destinations and no end-to-end usability evidence establishes this broad workflow claim.
- [ ] Keep the teaching content central in authoring and inspection workflows, with metadata and
  supporting explanations arranged compactly around it.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [ ] Gradebook rows should identify Students by their Course roster names and Coursework by title,
  with reference IDs as supporting information where useful.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
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
- [ ] Instructor lists and repeated records should be dense and easy to scan, more like a spreadsheet than cards.
  - Verification pending: Current source includes compact Course rows in `src/pages/course_list_page.tsx` and grid/panel layout in `src/pages/assessment_templates_page.css`; this new or expanded requirement lacks a scoped rendered audit across the affected pages at 1280 x 800. Existing local layouts do not establish the whole requirement.
- [ ] Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_student_view_page.tsx` `AssessmentWorkspaceStudentViewPage` renders the server-authorized answer-free Assessment projection, loads only the manifest and selected Question, disables native response controls, and provides no Student Work, Assessment Attempt, submission, or grade action.
  - Evidence (source): `crates/server/src/assessment_student_view.rs` `assessment_student_view_router`, `crates/learning-data-access/src/postgres/assessment_student_view.rs` `PostgresInstructorStudentViewStore`, and `schemas/base_schema/assessment_student_view.sql` `load_instructor_student_view_question_source` implement the authorized no-write server, Store, and SQL boundaries.
  - Evidence (runtime): a fresh PostgreSQL proof exercised the real Store through the API roles with a nonempty Ready Asset rendition and verified read-only SQLSTATE `25006` plus zero writes to Student-state tables.
  - Evidence (test): an independently reviewed Chromium component proof with mock transport covered native and WeBWorK presentations, navigation, disabled controls, stale and error recovery, and no mutation requests; it was not connected or live-stack acceptance.
  - Mismatch: connected live-HTTP acceptance is still missing; the unchanged full server compile is blocked in the AWS dependency graph; and production iMathAS Student View integration remains deferred outside the pilot. Independent final server source review passed, but it does not establish runtime behavior.
- [ ] Instructor lists and repeated records should favor compact rows or tables with clear columns over cards or loosely concatenated text.
  - Verification pending: Current source includes compact Course rows in `src/pages/course_list_page.tsx` and grid/panel layout in `src/pages/assessment_templates_page.css`; this new or expanded requirement lacks a scoped rendered audit across the affected pages at 1280 x 800. Existing local layouts do not establish the whole requirement.
- [ ] At 1280 x 800, Instructor pages should expose enough of the current workflow to minimize unnecessary scrolling.
  - Verification pending: Current source includes compact Course rows in `src/pages/course_list_page.tsx` and grid/panel layout in `src/pages/assessment_templates_page.css`; this new or expanded requirement lacks a scoped rendered audit across the affected pages at 1280 x 800. Existing local layouts do not establish the whole requirement.

#### Course interfaces

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

##### Blueprint Course interface

- [x] **My Blueprint Courses** should emphasize reusable course design rather than teaching activity.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCoursesWorkspace` presents reusable Blueprint Course content and adoption information.
- [x] **Search Public Blueprint Courses** helps Instructors find a Blueprint Course they already have in mind.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` submits the Instructor's name query to the Public-Blueprint list and opens the selected exact Blueprint detail.
  - Evidence (runtime): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` is exercised by accepted C47 actual HTTP and compiled-main browser proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.C6RwpH/public-search-result.json` and `public-search-browser.json`, using the Courses ribbon, submitted exact name search, and existing detail. It proves the isolated privileged-session workflow only; no ordinary login, TLS, full accessibility, or Course creation is claimed.
- [x] Public Blueprint Course search should support quickly narrowing a large collection.
  - Evidence (source): `crates/server/src/blueprint_course/list.rs` `list_blueprints` applies the literal Public query before paging, and `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` preserves the submitted query through continuation and resets it after an empty result.
  - Evidence (runtime): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` and `crates/server/src/blueprint_course/list.rs` `list_blueprints` are exercised by accepted C47 actual HTTP and compiled-main browser proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.C6RwpH/public-search-result.json` and `public-search-browser.json`, proving Public-only literal `%`, `_`, and backslash matching, a query-bound cursor, 51 matching records across the real 50-row continuation, and empty-search reset with unchanged `ple_data`.
- [x] A **Blueprint Course** should provide an obvious action for creating a **Course Instance** from it.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` renders "Create Course Instance from this Blueprint."
  - Evidence (runtime): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` is exercised by accepted C47 compiled-main browser proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.C6RwpH/public-search-browser.json`, which follows the existing detail action and preselects the matched Blueprint beyond the first 50 search rows. It does not submit or create a Course Instance.

##### Blueprint Course editing interface

- [ ] Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor.
  - Verification pending: re-audit `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` and the authorized Blueprint editor/lifecycle operation for this exact behavior. The former compact-Course-row 1280 x 800 note was unrelated and is removed; no lifecycle or content-editor proof follows from it.
- [x] The Course Editor should show the Blueprint Course structure without editing every Question on one page.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` lists modules and Assessments before selecting an editor.
- [x] Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor` receives the selected Assessment content and renders its Questions and Properties tasks.
  - Evidence (runtime): accepted compiled-main, actual-loopback-HTTP evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.ay40bT/blueprint-properties-browser.json` opens one selected Assessment and exercises both tasks through `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor`.
- [x] Only the selected Blueprint Assessment's Questions should appear in its editor.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor` renders entries from its selected `content` input only.
  - Evidence (runtime): accepted compiled-main, actual-loopback-HTTP evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.ay40bT/blueprint-properties-browser.json` verifies one selected Assessment, its separated Questions task, and an unchanged sibling after ordinary Save and exact reload through `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor`.
- [ ] **Blueprint Assessment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assessment.
  - Verification pending: re-audit `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` and the authorized Blueprint editor/lifecycle operation for this exact behavior. The former compact-Course-row 1280 x 800 note was unrelated and is removed; no lifecycle or content-editor proof follows from it.
- [ ] **Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
  - Verification pending: accepted compiled-main, actual-loopback-HTTP evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.ay40bT/blueprint-properties-browser.json` verifies shared local drafts across tasks, one ordinary PUT Save, exact reload, and all six Student-feedback controls. It does not exercise the full scoring, attempt, and late-work activity-control scope.
- [x] Blueprint Courses should not contain Assessment dates or relative Assessment schedules.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` describes reusable structure without Students, deadlines, or course delivery settings; the reusable Blueprint content model has no delivery-date fields.

##### Course Instance interface

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

#### Question interface

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
- [x] Search results should switch to a dense, information-rich layout.
  - Evidence (source): `src/pages/library_page.tsx` `question-library-row` renders title, summary, authors, identifier, and action per result.
- [x] Results should make it easy to scan many Questions quickly.
  - Evidence (source): `src/pages/library_page.tsx` `questionLibraryBrowseVirtualWindow` virtualizes dense result rows.
- [x] Results should show the information needed to judge relevance without opening each Question.
  - Evidence (source): `src/pages/library_page.tsx` `question-library-row` shows title, summary, authors, and identifier.
- [x] Search results should support filters for narrowing the Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` supplies author, backend, tag, Question Type, license, and capability filters.
- [x] Filters should update the current search rather than start a separate workflow.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` resets one `QuestionLibraryBrowseSession` with the updated query.
- [x] Search should support Google-like syntax for more precise queries.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `pub(super) struct QuestionTextQuery` accepts ordinary AND words, quoted phrases, minus exclusions, PLE field tags, and exact Question IDs with filters through its parser.
  - Evidence (runtime): accepted one-time private PostgreSQL 17 and actual-server HTTP proof exercised `crates/server/src/question_library.rs` `search_questions` with ordinary AND words, a quoted phrase, minus exclusion, and all five PLE fields under an active vetted Instructor; anonymous and Student requests remained concealed and responses were `no-store`. The temporary proof is retained outside Git through documentation acceptance.
- [x] Quoted text should search for an exact phrase.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `term_value` retains quoted text as one exact phrase term.
  - Evidence (runtime): the accepted C58 actual-server HTTP proof exercised `crates/server/src/question_library/search_query.rs` `term_value` and returned only `Alpha enzyme kinetics` for the quoted phrase `"enzyme kinetics"`.
- [x] A minus sign should exclude matching terms.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `exclusion_prefix` records a leading minus as an excluded search term.
  - Evidence (runtime): the accepted C58 actual-server HTTP proof exercised `crates/server/src/question_library/search_query.rs` `exclusion_prefix` and returned `Alpha enzyme kinetics` for `enzyme -inhibitor` while excluding the matching inhibitor Question.
- [x] Search should support PubMed-like field tags such as `topic:genetics`.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `field_prefix` recognizes `topic` and `SearchTerm::matches` applies it to Question metadata.
  - Evidence (runtime): the accepted C58 actual-server HTTP proof exercised `crates/server/src/question_library/search_query.rs` `field_prefix`, composed `subject:`, `topic:`, `tags:`, `type:`, and `author:` in one request, and returned the exact expected Question.
- [x] Field tags should use PLE concepts and vocabulary.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` limits field tags to PLE terms: `subject`, `topic`, `tags`, `type`, and `author`.
  - Evidence (runtime): the accepted C58 actual-server HTTP proof exercised `crates/server/src/question_library/search_query.rs` `SearchField` with only the closed PLE field vocabulary and preserved the production route's strict query boundary.
- [x] Useful fields may include subject, topic, tags, Question Type, and author.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `SearchField` and `SearchTerm::matches` support `subject`, `topic`, `tags`, `type`, and `author`.
  - Evidence (runtime): the accepted C58 actual-server HTTP proof exercised `crates/server/src/question_library/search_query.rs` `impl SearchTerm` for subject, topic, tags, Question Type, and author together through the ordinary `text` query parameter.
- [x] Simple and advanced searches should use the same search box.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` provides one Search input, and `crates/server/src/question_library.rs` passes its optional `text` query value to `QuestionTextQuery::parse` before matching.
  - Evidence (runtime): accepted C59 component proof exercised `src/pages/library_page.tsx` `LibraryPage` and confirmed the visible Search box and its normal-flow Search tips.
  - Evidence (runtime): the accepted C58 component and actual-server HTTP evidence exercised `src/pages/library_page.tsx` `LibraryPage` and `crates/server/src/question_library.rs` `search_questions` through the same visible Search input and `text` transport for ordinary words and advanced grammar.
- [x] Instructors should not need to learn search syntax to use Search Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` exposes ordinary search and labeled filter controls without syntax requirements.
- [x] The interface should make useful search syntax discoverable when needed.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders `question-library-search-tips` as a native disclosure beside the ordinary Search box with words, quotes, minus, PLE fields, and examples.
  - Evidence (runtime): accepted corrected desktop `src/pages/library_page.tsx` `LibraryPage` component proof opened Search tips without obscuring filters or bulk controls; the full `./check_codebase.sh` gate passed.
- [ ] Search syntax should help expert users quickly narrow a very large Question Library.
  - Evidence (runtime): accepted C58 actual-server HTTP evidence proves the grammar through the production Store and route across a bounded 69-Question fixture.
  - Verification pending: the bounded proof does not establish usability or performance for a very large production Question Library.
- [x] Search terms and active filters should remain visible while reviewing results.
  - Evidence (source): `src/pages/library_page.tsx` `query` signal remains bound to the search input and filter selects while rows render.
- [x] Clearing or changing part of a search should be quick.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` updates the search session on each input or selection change.
- [x] Opening a result and returning should preserve the Instructor's search and position.
  - Evidence (source): `src/pages/library_page_model.ts` `saveQuestionLibraryReturnState` and `takeQuestionLibraryReturnState` retain one session-bound, single-use in-document snapshot; `src/pages/library_page.tsx` `LibraryPage` restores its query, server-validated loaded rows, filters, and clamped scroll position.
  - Evidence (runtime): one-time accepted compiled-browser exercise of `src/pages/library_page.tsx` `LibraryPage` restored `genetics`, the `ple` filter, 80 loaded rows, and exact virtual-list scroll position through visible detail return and browser Back; a changed session returned to the empty landing. The temporary harness and screenshots were removed after the accepted proof.

##### Browse Question Library interface

- [x] **Browse Question Library** helps Instructors explore Questions without knowing what to search for.
  - Evidence (source): `src/route_contract.ts` `libraryBrowse` is a distinct Instructor route and `src/pages/library_page.tsx` `LibraryPage` provides an overview-first grouped Browse mode without requiring Search text.
  - Evidence (runtime): accepted private full-app browser evidence exercised `src/main.tsx` `render` against the actual server through overview-first Browse, Biology, Enzymes, and focused Search without starting from Search text; every actual search response was `no-store`.
- [x] Browse should help Instructors understand what the Question Library contains.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` explains that counts cover all authorized matching Questions and presents subject, topic, tag, and Question Type groups.
  - Evidence (runtime): accepted C60 component evidence rendered `src/pages/library_page.tsx` `LibraryPage` overview groups; actual-server HTTP evidence exercised `crates/server/src/question_library.rs` `search_questions` over the full authorized matched snapshot.
- [x] Browse should emphasize subjects, topics, tags, Question Types, and other useful groupings.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders dedicated Subjects, Topics, Tags, and Question Types groups with explicit free-text truncation notices.
  - Evidence (runtime): accepted component and actual-server HTTP evidence covered `crates/server/src/question_library/facets.rs` `facets` for all four groups, including truthful 64-value truncation flags.
- [x] Browse should make moving from broad subjects to narrower topics easy.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` clears the prior topic on subject selection and requests exact subject filtering before presenting `Topics in this subject`.
  - Evidence (runtime): accepted routed component evidence exercised `src/pages/library_page.tsx` `changeQuery`, selected Biochemistry then Enzymes, and preserved both exact filters.
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
- [ ] Instructors should be able to inspect a Question before adding it to an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `questionRevisionInspectionPath` addresses the candidate's exact Revision, and `src/api/client.ts` `getQuestionRevisionDetails` exposes the strict client operation. The server resolves the Instructor-authorized private source and checksum through the existing opaque WeBWorK adapter, which supplies a hardened iframe.
  - Evidence (runtime): accepted private PostgreSQL 17/MinIO and unchanged-renderer HTTP evidence returned preview 200 with hardened headers and concealed missing Revision, Student, and anonymous requests. Exact-main browser evidence visibly rendered the prompt and five choices. This isolated preview path left all five Student Work counts at zero before and after: Assessment Attempts, Question Attempts, saved responses, submissions, and grading results.
  - Verification pending: the renderer JavaScript dereferences `window.frameElement.id` when the hardened sandbox has no same-origin frame element, then errors before focus, popover, and parent telemetry. Do not loosen the iframe sandbox or rewrite the sibling renderer HTML. Successful embed behavior and return without losing Assessment state remain required for closure.
- [x] Assessment Properties should group related settings so important settings are easy to find.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` groups Assessment and delivery controls separately from Student feedback, and the owning CSS uses a two-column desktop grid that collapses to one column below 60rem.
  - Evidence (runtime): accepted private exact-main browser evidence rendered `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` at 1280 by 800; computed-style checks verified the two-column desktop and one-column 720px layouts, restored panel/control styling, and persisted edited instructions through actual HTTP and reload.
- [ ] Present timing settings in familiar units such as minutes, with explicit units and clear
  meanings for optional or unlimited values.
  - Verification pending: expanded requirement needs scoped source and rendered workflow proof against its full current wording. Bounded Course/theme/MATCH/Attempt-navigation receipts do not establish this broader interface contract.
- [x] Instructors can randomize Question order for an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `updateOrder` binds the current Assessment Properties checkbox to authored or shuffled Question order. The Student start transaction persists that rule with the Attempt and shuffles the complete fixed-and-Pool issued vector only for `shuffled`.
  - Evidence (runtime): accepted private actual-HTTP evidence exercised `crates/learning-data-access/src/postgres/assessment_delivery_start.rs` `start_current_assessment_attempt`: it persisted authored and shuffled rules, observed a concurrent Instructor save blocked behind the Student-start lock, issued exact fixed-and-Pool pins, and resumed the immutable shuffled Attempt after a current-rule edit. Accepted exact-main browser evidence saved and reloaded `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`'s visible **Randomize question order** checkbox through actual HTTP.
- [x] Answer-choice randomization belongs to the Question, not the Assessment.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_model.ts` `setChoiceRandomization` and the strict Question codec own `randomizeChoices`; `crates/adapters/ple/src/question_json/source_document.rs` `compile_choices` compiles it to `NativeChoiceOrder`, and `crates/question_model/src/presentation/builder.rs` `pending_items` applies its nonce-derived choice permutation. The closed Assessment activity rules contain Question-order policy but no answer-choice override.
  - Evidence (runtime): accepted exact-main browser evidence preserved `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`'s explicit Question-owned choice-order explanation while saving and reloading the independent Assessment Question-order rule. This verifies the ownership boundary without claiming a runtime matrix of every native choice permutation.
- [x] **Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` calls `listAssessmentsDueSoon` and states its across-Courses scope.
  - Evidence (runtime): accepted private actual-server evidence exercised `schemas/base_schema/assessment_operations.sql` `list_assessments_due_soon` across two owned Courses and one outsider Course; each Instructor saw only their own Course rows, while anonymous and Student requests received the same concealed response.
- [x] Assessments Due Soon shows the Course and due time for each Assessment.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `DueSoonAssessmentRow` renders `courseLongName` and a formatted due time.
  - Evidence (runtime): accepted private full-app evidence exercised `src/pages/assessments_due_soon_page.tsx` `formatDueTime`; both visible Course names and Due values matched the actual HTTP instants formatted in the response's Account time zone.

#### Assessment type appearance

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

#### High-consequence actions

- [ ] Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
  - Mismatch: `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` implements Assessment Unrelease and `src/features/blueprint_course/blueprint_course_lifecycle_controls.tsx` implements Archive Blueprint Course, but no current interface exposes Archive Published Question in a Danger Zone; the browser API in `src/api/question_availability.ts` alone is not an Instructor workflow.
- [x] Danger Zone should be visually separate from ordinary editing actions.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `assessment-workspace-unrelease-danger-zone` is the sole current Danger Zone consumer, and `src/pages/assessment_workspace/assessment_workspace.css` applies the distinct danger border, background, spacing, and responsive layout to that exact class while preserving its shared `assessment-editor-field` child.
  - Evidence (test): an independently reviewed temporary Chromium proof rendered `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `assessment-workspace-unrelease-danger-zone` with current CSS and verified the destructive panel remained visually distinct with a readable confirmation action at 1280px and 600px; it was component evidence, not connected-app acceptance.
- [x] Assessment Unrelease should explain that Student work will be deleted.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `assessment-unrelease-confirmation-help` says Unrelease permanently deletes the represented Student Work, and the action is labeled "Unrelease and delete Student Work."
- [x] Assessment Unrelease should require typing the Assessment title before confirmation.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `confirmationTitle` disables Unrelease unless the entered value exactly matches the current Assessment title, while `schemas/base_schema/unrelease.sql` `ple_api.unrelease_assessment` independently rejects a nonmatching confirmation title.
- [ ] Archive actions should explain the effect on shared availability and require a clear confirmation.
  - Mismatch: `src/features/blueprint_course/blueprint_course_lifecycle_controls.tsx` `Archive Blueprint Course` explains removal from new selection and requires the long name, but no current Archive Published Question interface provides the corresponding explanation and confirmation; `src/api/question_availability.ts` `archiveQuestion` is only a browser transport contract.
- [x] Restore actions should use ordinary availability controls.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `restore` is presented under Course names and availability rather than the archive confirmation control.
