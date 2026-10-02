## Interface design
### General interface design
- [ ] Design around what users need to find and do.
  - Mismatch: No repository-wide behavioral or usability evidence establishes this broad design outcome.

- [x] Important information should stand out from supporting information.
  - Evidence (source): `src/components/record_list/record_list.css` `.record-list__title` gives the record title the accent color and leaves the facts muted.
  - Evidence (source): `src/components/record_list/record_family.css` `.record-table thead th` gives each column header a heavier face and a distinct background.
  - Evidence (source): `src/components/record_list/record_family.css` `.record-table tbody th small` keeps the supporting id lighter than the row name.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Important information should stand out from supporting information.` rendered the shipped Gradebook, Course roster, My Active Courses, and Question Library in headless Chromium at 1280 by 800. Titles and column headers stood out from facts, cells, and supporting ids by weight or color. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Related information should be visually grouped and aligned.
  - Evidence (source): `src/components/record_list/record_list.css` `.record-list__facts` groups each record's related facts in one aligned grid.
  - Evidence (source): `src/components/record_list/record_list.tsx` `record-list__facts` places those facts together under the title.
  - Evidence (source): `src/components/record_list/record_table.tsx` `record-table` keeps each record's related cells on one row.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Related information should be visually grouped and aligned.` rendered the shipped Gradebook, Course roster, My Active Courses, and Question Library in headless Chromium at 1280 by 800. Related cells shared a row and lined up with their headers. Related facts stayed in one group below the title and above the actions, aligned within that group and across repeated records. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Similar pages should place similar controls in consistent locations.
  - Evidence (source): `src/components/record_list/record_sort_control.tsx` `RecordSortControl` places the order label above its select.
  - Evidence (source): `src/pages/library_page.tsx` `Order results` is the Question Library sort control above the result window.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `Sort Public Blueprint Courses` is the Blueprint search sort control above its results.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Similar pages should place similar controls in consistent locations.` rendered the shipped Question Library and Public Blueprint Course search in headless Chromium at 1280 by 800. Each sort control sat above its results, aligned to that result region, with the label above the select and the same select inset. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] A page describes what each record shows and what the user can do. Shared record and page components own the markup, spacing, and reflow, so the same facts and actions stay readable when the page gets narrower.
  - Evidence (source): `src/pages/course_list_page.tsx` `courseContent` names each Course, its classification, its Term, and Open Course.
  - Evidence (source): `src/components/record_list/record_list.tsx` `RecordList` renders that description in the shared semantic record row.
  - Evidence (source): `src/components/record_list/record_list.css` `record-list--semantic` lays the shared list out as one column so its facts and actions remain in the row when the page is narrower.
  - Evidence (test): `tests/playwright/record_list_contracts.mjs` `checkSemanticContent` kept the classification fact, title, facts, and actions visible at a 393px width.

- [x] Use headings and action labels that reflect the current state and next useful step.
  - Evidence (source): `src/pages/student_coursework_presentation.ts` `studentCourseworkDisplay` maps a resumable Attempt to Resume and In progress, a completed Attempt to Review and Completed, and upcoming Coursework to Open and Upcoming.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `assessmentContent` puts the Coursework title, that status, and that action on each row.
  - Evidence (test): `tests/playwright/student_course_entry_m6_evidence.mjs` `Use headings and action labels that reflect the current state and next useful step.` rendered the shipped Course landing in headless Chromium and showed Protein structure practice as In progress with Resume Weekly Assignment, Bonus protein challenge as Completed with Review Bonus Assignment, and Peptide quiz as Upcoming with Open Quiz. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Match feedback wording and visual emphasis to the outcome: success, information, warning, or
  error. Make the result and any next action easy to recognize.
  - Evidence (source): `src/pages/course_roster_page.tsx` `Course access was removed` states the successful roster result, and `Reload and try again` states the error's next action.
  - Evidence (source): `src/components/record_list/record_family.css` `record-collection__state--loading` gives collection loading its own information emphasis, separate from `record-collection__state--error`.
  - Evidence (source): `src/style.css` `confirmation-dialog` uses the warning emphasis, and `attempt-error` uses the danger emphasis.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `Match feedback wording and visual emphasis to the outcome: success, information, warning, or error. Make the result and any next action easy to recognize.` opened the shipped Course roster in headless Chromium. Loading used the information emphasis, the removal confirmation used the warning emphasis and named Keep it or Revoke course access, the successful removal used the success emphasis, and the refused removal used the danger emphasis with Reload and try again. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Primary actions should be easy to find and appear near the content or workflow they affect.
  - Evidence (source): `src/pages/gradebook_page.tsx` `Download CSV` places the score download with the Gradebook table.
  - Evidence (source): `src/pages/course_roster_page.tsx` `Remove course access` places the roster action in the student row.
  - Evidence (source): `src/pages/course_list_page.tsx` `Open Course` places the Course action in the Course record.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `label: "Open"` places the Question action in the Question record.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Primary actions should be easy to find and appear near the content or workflow they affect.` rendered the shipped Gradebook, Course roster, My Active Courses, and Question Library in headless Chromium at 1280 by 800. Each named action stayed in that viewport, inside its record or within the export group above the score table. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Identify the object and relevant context before an action that changes membership or stored
  settings, so users can recognize what they are accepting or changing.
  - Evidence (source): `src/pages/roster_confirmation_dialog.tsx` `Remove course access for` names the student, and the dialog body includes that student's roster ID, before Course access changes.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `instructor-deactivation-copy` names the Instructor Account id before deactivation.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `confirmationTitle` requires the current Assessment title before Unrelease.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `Identify the object and relevant context before an action that changes membership or stored settings, so users can recognize what they are accepting or changing.` opened the shipped Course roster in headless Chromium, showed Morgan Lee and Avery Thompson with their roster IDs before any revoke request, left the roster unchanged on Keep it, and revoked RU-001 only after confirmation. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use concise helper text near the control it explains. Present shared explanations once per
  relevant group and keep the main task information easy to scan.
  - Evidence (source): `src/pages/gradebook_page.tsx` `Export Assessment points` explains the Gradebook download once, beside those actions.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `question-library-bulk-help` explains the bulk actions once, inside that toolbar.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Use concise helper text near the control it explains. Present shared explanations once per relevant group and keep the main task information easy to scan.` rendered the shipped Gradebook and Question Library in headless Chromium at 1280 by 800. Each explanation appeared once, next to its control, while the score table and the first Question stayed in view. No Live Demo stack was started. No PostgreSQL proof was run.

- [ ] Avoid scattering related actions across page headers, menus, navigation, and content areas.
  - Mismatch: Current top-bar Sign Out contradicts the specified Profile-menu location.

- [ ] Dream big on the UI. Choose one visual philosophy and carry it through the entire interface.
  - Mismatch: No repository evidence can verify this whole-product qualitative outcome.

- [x] Students should have no upload capabilities. Instructor-created content should use text boxes.
  - Evidence (source): `src/features/profile_avatar/profile_avatar_role.ts` `profileRoleMayManageImage` shows a Profile image upload only to an Instructor or Sysadmin.
  - Evidence (source): `crates/server/src/profile_avatar.rs` `replace_profile_image` refuses every other role before reading an image.
  - Evidence (source): `crates/server/src/draft_question_images.rs` `instructor_session_hash` admits only an Instructor session before a Draft Question image upload.
  - Evidence (source): `crates/server/src/course_appearance.rs` `instructor_session_hash` admits only an Instructor session before a Course Banner upload.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_workspace.tsx` `Student-facing prompt` is a text box for Instructor-created Question content.
  - Evidence (source): `src/pages/course_roster_page.tsx` `importRoster` records Course roster rows from a text box.
  - Evidence (test): `tests/test_ribbon_route_contract.mjs` `Students should have no upload capabilities. Instructor-created content should use text boxes` found file inputs only for the Course Banner, Profile image, and HOTSPOT image, refused those Instructor routes and the Profile image control for a Student, and kept the Question prompt and roster import as text boxes.

- [x] Buttons should look intentionally designed rather than like native browser controls.
  - Evidence (source): `src/style.css` `:where(button)` sets appearance to none and gives each button a border, corner radius, background, and type weight.
  - Evidence (source): `src/ribbon/app_ribbon.css` `ple-app-ribbon__profile` and `ple-app-ribbon__profile-menu-item` keep a designed border and corner radius and do not set appearance.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Buttons should look intentionally designed rather than like native browser controls.` rendered the shipped Ribbon with those stylesheets in headless Chromium. Profile and Sign out computed appearance none and a non-zero corner radius. No Live Demo stack was started. No PostgreSQL proof was run.

### Rounded rectangles preference
- [x] Rounded rectangles are preferred for all interface objects, especially buttons, input fields, cards, avatars, tags, and interactive controls.
  - Evidence (source): `src/style.css` `:where(button)` rounds buttons, the text input rule rounds fields, `.course-card` rounds a card, and `.calm-status` rounds the status tag.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon__profile` rounds the Profile avatar.
  - Evidence (source): `src/components/question_response_control_styles.ts` `choice-card` rounds an answer choice.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Rounded rectangles are preferred for all interface objects, especially buttons, input fields, cards, avatars, tags, and interactive controls.` measured those named shipped objects in headless Chromium. The button, input, card, avatar, and answer choice were above 0px and below 16px. The status tag kept its compact pill. Structural regions stay square under the later structural rule. No Live Demo stack was started. No PostgreSQL proof was run.

- N/A Rounded corners generally feel softer, friendlier, and more contemporary.
  - Reason: supporting descriptive rationale, not independently closable; it remains binding design context for the rounded-object requirement.

- N/A Rounding also helps users visually distinguish discrete objects from the surrounding page.
  - Reason: supporting descriptive rationale, not independently closable; it remains binding design context for the rounded-object requirement.

- [x] Use corner radius to reinforce interface hierarchy.
  - Evidence (source): `src/style.css` `--ple-radius-surface` is smaller than `--ple-radius-control`, and `--ple-radius-control` is smaller than `--ple-radius-inset`. `body` sets no corner radius.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon` sets no corner radius.
  - Evidence (source): `src/features/question_picker/question_picker.css` `.question-picker-dialog` uses `--ple-radius-inset`.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Use corner radius to reinforce interface hierarchy.` measured the page body and Ribbon at 0px, then a course card, a button, and the Question picker dialog in increasing order, with the dialog below 16px, in headless Chromium. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Interactive and self-contained objects should generally be more rounded than structural containers.
  - Evidence (source): `src/style.css` `:where(button)` and the text input rule use the control radius, `.confirmation-dialog` uses that same radius, and `.course-card` uses the surface radius.
  - Evidence (source): `src/components/question_response_control_styles.ts` `choice-card` uses the control radius for an answer choice.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Interactive and self-contained objects should generally be more rounded than structural containers.` measured a shipped button, input, answer choice, confirmation dialog, and course card above 0px and below 16px after the page body, main region, Ribbon, tab navigation, and breadcrumb prelude measured 0px. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use moderate rounding for buttons, input fields, answer choices, dialogs, and similar interactive controls.
  - Evidence (source): `src/style.css` `:where(button)` and the text input rule use `--ple-radius-control`, and `.confirmation-dialog` uses that same control radius.
  - Evidence (source): `src/components/question_response_control_styles.ts` `choice-card` uses the control radius for an answer choice.
  - Evidence (source): `src/features/question_picker/question_picker.css` `.question-picker-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/features/question_pool_picker/question_pool_picker.css` `.question-pool-picker-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/pages/course_instance_page.css` `.course-instance-blueprint-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/features/blueprint_course/blueprint_course.css` `.blueprint-course-create-dialog` uses `--ple-radius-inset`.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Use moderate rounding for buttons, input fields, answer choices, dialogs, and similar interactive controls.` measured those named shipped controls in headless Chromium above 4px and below 16px. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use subtle rounding for cards, tables, panels, and other content containers.
  - Evidence (source): `src/style.css` `.course-card` and `.question-card` use `--ple-radius-surface`, which is smaller than `--ple-radius-control`.
  - Evidence (source): `src/components/record_list/record_family.css` `.record-table__scroll` uses `--ple-radius-surface` for the table container.
  - Evidence (source): `src/pages/question_statistics_panel.css` `.question-statistics-panel` uses `--ple-radius-surface`.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_authoring.css` `.assessment-editor-panel` uses `--ple-radius-surface`.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Use subtle rounding for cards, tables, panels, and other content containers.` measured those shipped containers in headless Chromium above 0px and below the button radius. Dialogs, avatars, and compact media keep the inset radius. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Keep large page regions, navigation bars, breadcrumbs, and other structural layout elements square or nearly square.
  - Evidence (source): `src/style.css` `body` and the main content region set the page surface without a corner radius.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon` is the navigation bar and sets no corner radius. Tab links on that bar keep the control radius.
  - Evidence (source): `src/ribbon/app_ribbon_density.css` `.ple-shell__breadcrumb-prelude` is the breadcrumb trail and sets no corner radius.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Keep large page regions, navigation bars, breadcrumbs, and other structural layout elements square or nearly square.` measured the shipped Ribbon, its tab navigation, a breadcrumb prelude using the shipped class, body, and main at 0px on every corner in headless Chromium. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Pills and fully rounded shapes should be reserved for compact objects such as tags, badges, timers, and avatars.
  - Evidence (source): `src/style.css` `.calm-status` is the fully rounded status badge.
  - Evidence (source): `src/pages/library_page.css` `.question-library-browse-active` wraps the fully rounded active-filter tag.
  - Evidence (source): `src/components/question_response_control_styles.ts` `QUESTION_RESPONSE_CONTROL_STYLES` fully rounds the choice number and the status spinner.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Pills and fully rounded shapes should be reserved for compact objects such as tags, badges, timers, and avatars.` inventories every fully rounded border radius under src and measures those four compact elements in headless Chromium. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Apply corner radii consistently to objects that serve the same purpose.
  - Evidence (source): `src/style.css` `.primary-action` and `.quiet-action` share `--ple-radius-control` with `:where(button)`.
  - Evidence (source): `src/style.css` `input:not([type="radio"])` shares `--ple-radius-control` with select and textarea.
  - Evidence (source): `src/style.css` `.course-card` and `.question-card` share `--ple-radius-surface`.
  - Evidence (source): `src/features/question_picker/question_picker.css` `.question-picker-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/features/question_pool_picker/question_pool_picker.css` `.question-pool-picker-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/pages/course_instance_page.css` `.course-instance-blueprint-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/features/blueprint_course/blueprint_course.css` `.blueprint-course-create-dialog` uses `--ple-radius-inset`.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Apply corner radii consistently to objects that serve the same purpose.` measured three action buttons at one radius, three text fields at one radius, two cards at one radius, and four task dialogs at one larger radius in headless Chromium. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use the application's typography, spacing, corner radius, borders, and interaction states consistently.
  - Evidence (source): `src/style.css` `font-family: inherit` keeps buttons and text fields on the application face. `:where(button)` sets the shared control size, padding, border, radius, and hover. `:focus-visible` sets the shared focus ring.
  - Evidence (source): `src/style.css` `input:not([type="radio"])` shares field padding, border, and radius with select and textarea.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Use the application's typography, spacing, corner radius, borders, and interaction states consistently.` measured plain, primary, quiet, and disabled buttons, plus input, select, and textarea, in headless Chromium with the shipped grass light tokens. They shared the application font and a 1px solid border. Each group shared padding and radius. The disabled button used a not-allowed cursor, keyboard focus showed a 2px solid ring, and hover changed the button border. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Primary, secondary, and low-emphasis actions should be visually distinct.
  - Evidence (source): `src/style.css` `:where(button)` is the unclassified secondary face, `.primary-action` is the filled primary face, and `.quiet-action` is the transparent low-emphasis face.
  - Evidence (source): `src/components/unsaved_changes_guard.tsx` `Stay and keep editing` is the low-emphasis action, the unlabeled save button is the secondary face, and `Discard and continue` is the primary action.
  - Evidence (source): `src/appearance/theme_registry.ts` `themeStyle` supplies the grass light tokens used to measure those faces.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Primary, secondary, and low-emphasis actions should be visually distinct.` rendered the shipped unsaved-changes dialog in headless Chromium. The quiet, unclassified, and primary buttons computed three different backgrounds, and the quiet border differed from the secondary border. No Live Demo stack was started. No PostgreSQL proof was run.

### Information density and layout
- [x] Design Instructor and **Sysadmin** workflows for laptop browsers, using a 1280 by 800 viewport
  as the layout target.
  - Evidence (source): `tests/playwright/ui_corpus_manifest.ts` `RIBBON_RESPONSIVE_PROFILES` and `SYSADMIN_DESKTOP_CONTEXT_OPTIONS` declare 1280 by 800 desktop contexts for both staff roles.
  - Evidence (test): `tests/playwright/ribbon_responsive_evidence.mjs` `assertResponsiveRows` verifies the Instructor desktop shell and `assertSysadminDesktopRibbon` verifies the Sysadmin Ribbon has no overflow with Instructor Accounts and Scoped Support visible.
  - Evidence (source): `src/pages/role_home_pages.tsx` `SysadminHomePage` presents the backed Instructor Accounts and Scoped Support operations reached by the checked Sysadmin desktop model.
  - Decision: retained viewport-target evidence supports this equivalent design-target rewrite, not whole-product usability or all staff workflows.

- [x] PLE often presents large collections where users need to find a few relevant items.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders the Question Library collection surface.

- [ ] Optimize large collections for scanning, searching, filtering, and comparison.
  - Mismatch: The broad all-collection outcome lacks complete implementation evidence.

- [x] Show enough useful information at once to support comparison without excessive scrolling.
  - Evidence (source): `src/pages/library_page.css` `question-library-bulk-toolbar` keeps the Question Library bulk strip compact and gives the result column the remaining width at the 1280 by 800 layout.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `comparableLibraryRows` rendered the shipped Question Library and Course roster in headless Chromium at 1280 by 800. Two Library records stayed fully in that viewport with their titles and facts, and two roster rows stayed fully in that viewport. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Search and filters should help users quickly narrow large collections.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` applies search text, author, backend, tag, classification, Question type, license, course use, and capability changes to the current Question Library query.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` excludes a Question that misses the requested text, author, tag, classification, type, or license.
  - Evidence (source): `crates/learning-data-access/src/postgres/question_library/search.rs` `search_published_question_library_entries` calls that shipped PostgreSQL function.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/question_library.rs` `nonmatching_question_id` adds one outsider beside 65 matching Questions. `question_library_search_filters_and_pages_in_postgresql` runs `PostgresQuestionLibraryStore` on disposable PostgreSQL. The shared filters return those 65 Questions and omit the outsider. The outsider's tag returns only that Question. Excluding the matching text keeps the outsider. This proof is 66 Questions. It does not time a 13,000 Question library.

- [x] Dense pages should remain easy to scan.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` keeps each Published Question title and its facts on one record row.
  - Evidence (source): `src/components/record_list/record_list.css` `record-list__facts` places those facts in a grid on the row.
  - Evidence (source): `src/pages/course_roster_page.tsx` `CourseRosterPage` keeps each Student, state, and action on one roster row.
  - Evidence (test): `tests/playwright/test_dense_page_scan.mjs` `DENSE_PAGE_SENTENCE` rendered the shipped Question Library and Course roster in headless Chromium at 1280 by 800. Two Library records stayed in the viewport with their titles and facts on one line. Two roster rows stayed in the viewport with aligned Student and state columns. The rows were square, unshadowed, and separated by a divider. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Treat screen space as a limited resource. Prefer useful information over decorative whitespace.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` places each Published Question in the result region.
  - Evidence (source): `src/pages/library_page.css` `library-browse-record-list__window` keeps desktop result rows on short block padding so the record text uses the row.
  - Evidence (source): `src/pages/course_roster_page.tsx` `CourseRosterPage` renders the Current roster in `src/pages/course_roster_page.css` `roster-section`, which sets no padding and no reserved block under the table.
  - Evidence (test): `tests/playwright/test_screen_space.mjs` `SCREEN_SPACE_SENTENCE` rendered the shipped Question Library and Course roster in headless Chromium at 1280 by 800. The result region had no padding and its visible area was filled by records. Each visible record's title, description, facts, and action were taller than the row padding, and the space between those parts stayed within 16 pixels. The roster section ended at the table, and each Student cell's text was taller than its padding. Leftover viewport under the short roster is the end of the page. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use spacing to separate meaningful groups rather than simply making pages spacious. Large gaps should communicate a meaningful change in section or task.
  - Evidence (source): `src/pages/library_page.tsx` `library-page__filters` stacks Question Library filters apart from `src/pages/library_page.css` `library-page__results`, so the result window does not open a hole between filter groups.
  - Evidence (source): `src/pages/course_roster_page.css` `roster-tools` separates the import tools from the Current roster.
  - Evidence (test): `tests/playwright/test_group_spacing.mjs` `GROUP_SPACING_SENTENCE` rendered the shipped Question Library and Course roster in headless Chromium at 1280 by 800. Filter groups stayed 8 to 16 pixels apart, and the space inside one record stayed within that. Filters and results stayed 8 to 16 pixels apart. The result stack stayed within 24 pixels, and the result region stayed inside the viewport. Roster rows met at a divider, and the import tools followed the roster across a larger gap of at most 32 pixels. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Prefer alignment, typography, and dividers over unnecessary cards, boxes, borders, and nested containers.
  - Evidence (source): `src/pages/library_browse_controls.tsx` `LibraryBrowseControls` names each browse group with a heading.
  - Evidence (source): `src/pages/library_page.css` `question-library-browse-groups` separates those groups with a bottom divider and gives the choices no card, border, or nested box.
  - Evidence (test): `tests/playwright/test_alignment_dividers.mjs` `ALIGNMENT_DIVIDER_SENTENCE` rendered the shipped Question Library browse groups in headless Chromium at 1280 by 800. Subjects, Tags, and Question Types shared one column. Each heading aligned with its group. Each group closed with a one-pixel divider and had no fill, corner radius, shadow, or side border. The choice list inside each group was not another box. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Cards and rounded containers should earn their space by representing a distinct object or interaction, not merely grouping nearby content.
  - Evidence (source): `src/pages/library_page.css` `question-library-controls` leaves Classification filters and Bloom filters unpadded, square, unfilled, and unshadowed. `src/pages/library_page.css` `question-library-bloom-report` does the same for the count lists. `src/pages/library_page.css` `question-library-bulk-toolbar` does the same for the action strip.
  - Evidence (source): `src/pages/library_page.css` `question-library-facet-choices` keeps a border and control radius on each choice, and `src/pages/library_browse_controls.tsx` `question-library-browse-active` keeps the selected Tag as its own pill.
  - Evidence (test): `tests/playwright/test_distinct_object_cards.mjs` `DISTINCT_OBJECT_SENTENCE` rendered the shipped Question Library browse page in headless Chromium at 1280 by 800. Classification filters, Bloom filters, the Bloom count lists, and the bulk-action strip grouped more than one field or button and had no border, radius, shadow, or fill. One facet button kept its border and radius, and the Tag pill kept its radius. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Avoid the modern dashboard style of large rounded cards, generous padding, and isolated islands of content.
  - Evidence (source): `src/pages/assessment_templates_page.css` `assessment-template-overview` places Your Templates and the editor in one workspace without card fill, radius, or shadow. `src/pages/assessment_templates_page.css` `assessment-template-editor-empty` leaves Choose a Template unpadded and square.
  - Evidence (source): `src/pages/library_page.css` `question-library-controls-initial` keeps the opening search on the page without hero padding. `src/pages/library_page.css` `question-library-search-tips` leaves the tip text without a rounded shadow box. `src/pages/library_browse_record_list.css` `library-browse-record-list__window` keeps the result rows in a scroll region without a floating card.
  - Evidence (test): `tests/playwright/test_dashboard_islands.mjs` `DASHBOARD_SENTENCE` rendered the shipped Template workspace, opening Question Library search, and Question Library browse page in headless Chromium at 1280 by 800. The Template columns shared one top edge and stayed within 24 pixels, with no padding, radius, shadow, or fill. Choose a Template was flat. The opening search had no block padding, and the opened search tips were flat text. The result region was at least 300 pixels tall, held two records, and had no card fill or shadow. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use horizontal and vertical space efficiently without crowding information together. Related information should form clearly readable rows, columns, or groups.
  - Evidence (source): `src/pages/assessment_templates_page.css` `assessment-template-editor-empty` separates Choose a Template from its message. `src/pages/assessment_templates_page.css` `assessment-template-overview` stacks Your Templates as one column beside the editor.
  - Evidence (source): `src/pages/library_page.css` `question-library-controls` keeps each classification name off its field. `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` stacks a Question title, description, facts, and action. `src/components/library_bloom_discovery.tsx` `question-library-bloom-report` places each count beside its name.
  - Evidence (test): `tests/playwright/test_readable_groups.mjs` `READABLE_GROUP_SENTENCE` rendered the shipped Template workspace and Question Library browse page in headless Chromium at 1280 by 800. The Template message sat 4 to 16 pixels under its heading, the overview stacked in one column, and the editor stayed 8 to 24 pixels to its right. A classification name sat 4 to 16 pixels above its field. A Question record stacked its parts in that same range, and at least two facts shared a row. A Bloom count sat at least 8 pixels from its name, and the two count lists stayed 4 to 16 pixels apart. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Size controls and content regions for their contents and task. Avoid unnecessarily tall panels, empty states, Question previews, and other fixed-height regions.
  - Evidence (source): `src/pages/library_page.css` `library-browse-record-list__window` keeps the desktop Question Library result region inside the viewport instead of a 65vh panel measured from the top of the screen.
  - Evidence (source): `src/pages/assessment_templates_page.css` `assessment-template-editor-empty` gives the Template empty state no reserved block size and no trailing message margin.
  - Evidence (source): `src/components/opaque_webwork_preview_frame.tsx` `OpaqueWebworkPreviewFrame` sizes the Question preview from a resize report between 160 and 1200 pixels.
  - Evidence (test): `tests/playwright/test_region_sizing.mjs` `REGION_SENTENCE` rendered those shipped surfaces in headless Chromium at 1280 by 800. The result region stayed inside the viewport and held two records. The empty state was shorter than 160 pixels with at most 24 pixels under its message. The preview ignored a 2000 pixel report and adopted 420 pixels. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Keep the visual design compact, flat, information dense, and consistent across PLE.
  - Evidence (source): `src/pages/library_page.css` `question-pool-create-review-grid` and `src/components/question_pool_create_dialog.css` `question-pool-create-review-grid` use the same flat groups as Question Library browse.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` and `src/pages/course_roster_page.tsx` `CourseRosterPage` keep collection rows compact and unshadowed, with the shared action height.
  - Evidence (test): `tests/playwright/test_visual_consistency.mjs` `VISUAL_CONSISTENCY_SENTENCE` rendered the shipped Question Library, Course roster, and Question Pool review in headless Chromium at 1280 by 800. Two Library records and two roster rows stayed in view, shared one font, and had no corner radius or shadow. Their action heights matched within 2 pixels. The Pool review groups matched the browse groups: no padding, fill, radius, or shadow, in two columns 8 to 24 pixels apart. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use compact rows, restrained corner rounding, and controls sized to their task.
  - Evidence (source): `src/style.css` `.instructor-list__row` uses `--ple-radius-surface` and `--ple-list-row-min-block-size`.
  - Evidence (source): `src/style.css` `:where(button)` uses `--ple-control-min-height` for the shared control size.
  - Evidence (source): `src/components/record_list/record_list.css` `.record-list__row` uses compact block padding and a divider, and sets no corner radius.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Use compact rows, restrained corner rounding, and controls sized to their task.` measured an instructor row and a record row in headless Chromium. Each row was at least as tall as its button and shorter than three button heights. The instructor row radius was above 0px and below the button radius. The record row radius was below the button radius. Both buttons shared one height. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Present short labels and values in aligned rows or compact grids, adapting to stacked groups
  when the available width requires them.
  - Evidence (source): `src/pages/question_statistics_panel.tsx` `QuestionStatisticsPanel` renders Blank rate, Answered rate, Correct rate, Partial rate, Incorrect rate, and Mean credit as short labels with their values.
  - Evidence (source): `src/pages/question_statistics_panel.css` `question-statistics-measures` places those pairs in two columns and stacks them to one column below 40rem.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Present short labels and values in aligned rows or compact grids, adapting to stacked groups when the available width requires them.` rendered QuestionStatisticsPanel in headless Chromium, measured two aligned columns at 1280px, then one stacked column at 480px. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Give each object one clear title within its list entry. Group its metadata and actions beneath
  or alongside that title.
  - Evidence (source): `src/pages/course_list_page.tsx` `courseContent` gives each Course one title, its classification and Term, and Open Course.
  - Evidence (source): `src/components/record_list/record_list.tsx` `RecordSemanticContent` places that one title, then the record's facts, then its actions.
  - Evidence (source): `src/components/record_list/record_table.tsx` `RecordTable` gives each record one row header and places the other columns in that same row.
  - Evidence (test): `tests/playwright/record_list_contracts.mjs` `checkAlignment` kept one title per list entry and grouped its metadata and actions beneath that title.
  - Evidence (test): `tests/playwright/record_list_contracts.mjs` `checkRecordFamily` kept one row-header title with its metadata and action in the same row, and one review heading with its note and action.

- [x] Preserve readable text and reachable controls as users enlarge text or zoom the page.
  - Evidence (source): `src/style.css` `font-size: 100%` lets the document root follow the user's text size, and Ribbon labels and buttons use rem sizes that grow with that root.
  - Evidence (source): `src/index.html` `initial-scale=1.0` leaves page zoom available.
  - Evidence (test): `tests/playwright/ribbon_narrow_routed_shell.mjs` `Preserve readable text and reachable controls as users enlarge text or zoom the page.` rendered the shipped Ribbon in headless Chromium, measured the Courses label and the outside button at a 200% root and at zoom 2, and kept each text run visible and each control reachable. No Live Demo stack was started. No PostgreSQL proof was run.

### Interaction design
- [x] Use progressive disclosure to keep common tasks compact while making supporting details easy
  to find.
  - Evidence (source): `src/style.css` `details:not([open]) > :not(summary)` hides supporting content until its disclosure is opened.
  - Evidence (source): `src/pages/course_roster_page.tsx` `roster-tools` keeps the Roster tools summary available and places the import explanation inside that disclosure.
  - Evidence (test): `tests/playwright/ribbon_narrow_routed_shell.mjs` `Use progressive disclosure to keep common tasks compact while making supporting details easy to find.` rendered the Roster tools disclosure in headless Chromium, kept the summary visible while the import explanation had no box, then opened the section and showed that explanation. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use tooltips for brief supplementary explanations, available on hover and keyboard focus.
  - Evidence (source): `src/style.css` `attr(title)` shows the Profile control and display-mode switch titles on hover and keyboard focus, and hides the Profile tip while its menu is open.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `title="Profile"` explains the icon Profile button, and `src/appearance/display_mode_toggle.tsx` `Switch to ` explains the display-mode glyph.
  - Evidence (test): `tests/playwright/ribbon_narrow_routed_shell.mjs` `Use tooltips for brief supplementary explanations, available on hover and keyboard focus.` rendered the shipped Ribbon Profile button in headless Chromium, showed its title below the button on hover, showed that same title on keyboard focus, and hid it at rest and while the Profile menu was open. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use clearly labeled expandable sections with chevrons for longer details and secondary settings,
  supporting keyboard, pointer, and touch interaction.
  - Evidence (source): `src/style.css` `details > summary::before` draws one disclosure chevron, and `details[open] > summary::before` turns that chevron when the section is open.
  - Evidence (source): `src/pages/course_roster_page.tsx` `roster-tools` labels the secondary roster section Roster tools.
  - Evidence (test): `tests/playwright/ribbon_narrow_routed_shell.mjs` `Use clearly labeled expandable sections with chevrons for longer details and secondary settings, supporting keyboard, pointer, and touch interaction.` rendered that Roster tools disclosure in headless Chromium, opened it from the keyboard, closed it with a pointer click, and opened it again from a touch tap. The chevron stayed visible and changed direction while open. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Keep essential information, primary actions, and current status visible in the main interface.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__user-role` shows the signed-in role in the Ribbon.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `aria-current` marks the selected Ribbon tab, and the Ribbon tabs are the primary actions.
  - Evidence (test): `tests/playwright/ribbon_narrow_routed_shell.mjs` `Keep essential information, primary actions, and current status visible in the main interface.` rendered the shipped Ribbon in headless Chromium at 1280 by 800. The Instructor role, every Ribbon tab, and the current tab were visible in that bar while the Profile menu was closed. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use drag-and-drop where it makes reordering faster and more natural.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `RecordSequence` orders Assessment Entries through the shared record movement.
  - Evidence (source): `src/components/record_list/record_sequence.tsx` `completeDrop` accepts a native drop only when the dragged record and the target are distinct enabled records in the current order.
  - Evidence (source): `src/components/record_list/record_list_reorder.tsx` `RecordMoveControls` renders the native drag control for that movement.
  - Evidence (test): `tests/test_record_list_reorder.mjs` `native drag moves Alpha onto Charlie` moves Alpha onto Charlie, refuses a disabled or same-record drop, and renders the enabled drag control.

- [x] Reordering must also have a precise keyboard-accessible method.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `moveEntry` and `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `move` back their labelled native-button Move earlier and Move later controls.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_choice_list.tsx` `onMoveChoice`, `src/features/ple_question_json_authoring/question_json_multiple_answer_editor.tsx` `onMoveChoice`, `src/features/ple_question_json_authoring/question_json_multi_fill_in_editor.tsx` `onMoveBlank`, `src/features/ple_question_json_authoring/question_json_matching_editor.tsx` `onMoveItem`, and `src/features/ple_question_json_authoring/question_json_ordering_editor.tsx` `onMoveItem` give every native JSON reorderer the same precise buttons.
  - Evidence (test): `tests/test_blueprint_course_model.mjs` `reusable entries preserve fixed and Question Pool interleaving`, `tests/test_ple_question_json_editor_model.mjs` `choice edits retain semantic IDs and enforce choices and correct-answer invariants`, `tests/test_ple_question_json_multiple_answer_editor.mjs` `multiple-answer text edits and reordering retain choice IDs and exact correct IDs`, and `tests/test_ple_question_json_multi_fill_ordering_authoring.mjs` `ORDER treats Ordering Items as the source of truth and derives correctOrder after movement` protect the stable reorder results.
  - Decision: The one-time seven-surface keyboard-control inventory passed and was removed rather than becoming a permanent implementation-inventory test. Drag-and-drop is checked on its own bullet.

- [x] UUIDs should never appear in visible content, navigation URLs, or copyable links.
  - Evidence (test): `tests/test_frontend_contract.mjs` `UUIDs should never appear in visible content, navigation URLs, or copyable links.` rendered Discipline: Name unavailable, built the Blueprint search description as Name unavailable, titled the retained Attempt Quiz: Attempt 2 with roster RU-001, and showed Impact notice activity for 7K3M-79QP. That visible text omitted aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee. The activity link kept that UUID only in its route fragment. No Live Demo stack was started. No PostgreSQL proof was run.
  - Evidence (source): `src/components/course_classification_summary.tsx` `vocabularyName` returns Name unavailable when the vocabulary item is missing.
  - Evidence (source): `src/pages/blueprint_course_search_classification.tsx` `blueprintClassificationDescription` uses Name unavailable for a missing classification name.
  - Evidence (source): `src/components/course_student_work_recovery.tsx` `archivedAttemptVisibleFacts` titles the Attempt number and roster ID.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `notificationContent` keeps the activity UUID in the route fragment.
  - Evidence (source): `src/components/copyable_question_id.tsx` `CopyableQuestionId` copies only a canonical Question ID.
  - Evidence (source): `src/pages/profile_account_id.tsx` `validateCanonicalPublicId` copies only a canonical Account ID.
  - Evidence (test): `tests/test_public_navigation.mjs` `objects without a public ID use their UUID in routes and reject a secondary Id` accepts the UUID route for an Attempt, Draft, and proposal and rejects a second Id.
  - Decision: The later rule uses a UUID Id in routes and JSON for an object without a public ID. Visible text and clipboard copies stay free of that UUID.

### Role badges
- [x] The role badge is always in the upper left, just left of the logo.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__user-role` is the first control in `ple-app-ribbon__context-identity`, immediately before `ple-app-ribbon__brand`.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon__user-role` stays displayed at phone width for Student, Instructor, and Sysadmin.

- [x] **Sysadmin** uses tomato red as its role color.
  - Evidence (source): `src/styles/user_role.css` `[data-user-role="sysadmin"]` defines `--ple-role-accent: #ff6347`.

- [x] **Instructor** uses teal green as its role color.
  - Evidence (source): `src/styles/user_role.css` `[data-user-role="instructor"]` defines `--ple-role-accent: #168575`.

- [x] **Student** uses lavender purple as its role color.
  - Evidence (source): `src/styles/user_role.css` `[data-user-role="student"]` defines `--ple-role-accent: #8861b5`.

- [x] The student role badge in mobile is shortened.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__user-role` shows the Student role label.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon__user-role[data-user-role="student"]` keeps that badge visible and narrower than the desktop reservation on a phone.

- [x] Role colors should be used consistently in role labels and other appropriate interface cues.
  - Evidence (source): `src/styles/user_role.css` `.live-demo-persona-action[data-user-role]` and `.ple-app-ribbon__user-role[data-user-role]` consume the shared role tokens.

- [x] Demo role selection should clearly state both the user's role and name.
  - Evidence (test): `tests/playwright/e2e_live_demo_authoring_browser.mjs` selects `Assume the role of Instructor Dr. Elena Rivera`.

### Themes
- [x] The interface uses a fixed set of visually distinct biome and habitat themes.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` checks the closed Theme catalog, habitat names, and distinct canvases.
  - Evidence (source): `src/appearance/theme_registry.ts` `THEME_REGISTRY` is that fixed catalog.

- [x] Each theme has coordinated Light and Dark appearances.
  - Evidence (source): `src/appearance/theme_registry.ts` `THEME_REGISTRY` stores a light and dark palette for every Theme.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` reads both appearances.

- [x] An Instructor has a personal theme for global Instructor pages and independently controls the theme assigned to each Course.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `updatePersonalTheme` saves the Instructor personal theme.
  - Evidence (source): `src/appearance/profile_appearance.tsx` `savePersonalTheme` offers that control only to an Instructor.
  - Evidence (source): `crates/server/src/course_appearance.rs` `update_theme` persists the Course theme for an Instructor.

- [x] Course pages use the Course theme. Global Instructor pages use the Instructor's personal theme.
  - Evidence (source): `src/appearance/appearance_rules.ts` `resolveTheme` selects the Course theme when a page has one and the personal theme on global Instructor pages.
  - Evidence (test): `tests/test_appearance_rules.mjs` `Course pages use their Course Theme` covers a Course page.
  - Evidence (test): `tests/test_appearance_rules.mjs` `global Instructor pages use the Instructor personal Theme` covers a global Instructor page.

- [x] Light/Dark is a separate user preference that applies across all themes and pages.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `updateDisplayModePreference` saves display mode without a theme.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `dataset.displayMode` applies that one preference on the document.

- [x] Light/Dark has only two selectable values: Light and Dark. When no explicit preference is saved, follow the browser setting.
  - Evidence (source): `generated/api/DisplayMode.ts` `DISPLAY_MODE_VALUES` lists light and dark.
  - Evidence (test): `tests/test_appearance_rules.mjs` `an unset display preference follows the browser` uses the browser value when no preference is saved.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `prefers-color-scheme` reads the browser setting.

- [x] Changing Light/Dark must not change the selected theme. Changing a theme must not change Light/Dark.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `updateDisplayModePreference` writes only the display-mode preference.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `updateInstructorPersonalTheme` writes only the personal theme.
  - Evidence (source): `crates/server/src/course_appearance.rs` `update_theme` writes only the Course theme.

- [x] Themes should affect major page surfaces so each theme is visually distinct across the whole interface.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `themeStyle` applies the resolved theme tokens to the document root.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` checks page, surface, secondary, highlight, and accent colors.

- [x] Each Light or Dark theme appearance is defined by five source colors: Canvas, Surface, Secondary, Accent, and Highlight.
  - Evidence (source): `src/appearance/theme_registry.ts` `ThemePalette` names canvas, surface, secondary, accent, and highlight.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` compares those five colors with the palette document.

- [x] Theme colors should remain accessible in their actual interface uses.
  - Evidence (source): `src/appearance/theme_registry.ts` `themeStyle` publishes the shared tokens used by the shell, Ribbon, and result surfaces.

- [x] Light themes should use clearly light page backgrounds. Dark themes should use clearly dark page backgrounds.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` requires every light canvas to stay clearly light and every dark canvas clearly dark.
  - Evidence (source): `src/appearance/theme_registry.ts` `themeStyle` publishes that canvas as the page background.

- [x] Check text, controls, borders, and interaction states against their actual rendered backgrounds.
  - Evidence (source): `src/ribbon/app_ribbon.css` `background-color` fills the selected Ribbon tab without clearing its pending gradient.

- [x] Apply the contrast requirements for text, controls, and other semantic uses in
  [BIOME_THEME_PALETTES.md](/docs/BIOME_THEME_PALETTES.md) to rendered components in both Light
  and Dark modes, including gradients and state backgrounds.
  - Evidence (source): `src/style.css` `brand-mark` paints its label on an accent gradient whose stops stay readable.

- [x] Pair color cues with text, icons, or shapes so selection, focus, saved status, and results remain
  recognizable across themes and color-vision differences.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `is-saved` marks a saved Question with a check and a thick edge.
  - Evidence (source): `src/components/student_feedback_panel.tsx` `studentFeedbackAnnouncement` states Correct or Not quite in words.

- [x] Theme IDs are durable. Changing a theme's display name or colors should not require a new ID.
  - Evidence (source): `src/appearance/theme_registry.ts` `grass` remains the Theme id while its display name is Grassland.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` checks that id and name pair.

- [x] Follow `docs/BIOME_THEME_PALETTES.md` for theme names, palettes, accessibility, and implementation.
  - Evidence (source): `src/appearance/theme_registry.ts` `THEME_REGISTRY` stores the palette document's habitat names and five source colors.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` compares those names and colors with the palette document.

### Typography
- [x] Use [Atkinson Hyperlegible Next](https://www.brailleinstitute.org/freefont/) as the main PLE font.
  - Evidence (source): `src/style.css` `:root` sets `Atkinson Hyperlegible Next` as the first font family.
  - Evidence (runtime): `src/style.css` `:root` was confirmed by a current authorized Student WeBWorK iframe probe at 1440 and 390 CSS pixels: `document.fonts` was loaded and computed `Atkinson Hyperlegible Next` first on both the body and its visible input. Receipt: `/private/tmp/ple-resumed-types-20260916.md`.

- [x] Use [Atkinson Hyperlegible Mono](https://www.brailleinstitute.org/freefont/) for code and other monospace text.
  - Evidence (source): `src/styles/browser_fonts.css` `--ple-font-mono` loads the local Atkinson Hyperlegible Mono face for code and monospace text.

- [x] Prefer the official Braille Institute font files and include the needed weights locally with PLE.
  - Evidence (source): `src/styles/browser_fonts.css` `@font-face` loads local Atkinson Hyperlegible Next variable font files.

- [x] When a narrow font is needed, use `IBM Plex Sans Condensed` for long unbreakable strings such as URLs.
  - Evidence (source): `src/styles/browser_fonts.css` `--ple-font-narrow` declares the local `IBM Plex Sans Condensed` face and applies it only to the PLE Question JSON editor's Citation URL input; `pipeline/build.mjs` `BROWSER_FONT_BUNDLES` copies and verifies that same-origin asset.
  - Evidence (runtime): `src/features/ple_question_json_authoring/question_json_editor_styles.ts` `PLE_QUESTION_JSON_EDITOR_STYLES`: a one-time Chromium fixture imported the actual injected editor CSS against production-built local font assets on 2026-09-15; at 360 and 1280 CSS pixels in light and dark OS preferences it requested the local font, computed the narrow family on the Citation URL input, and retained normal input value, horizontal-scroll, and overflow behavior.

- [x] With `IBM Plex Sans Condensed`, try `font-variant-numeric: slashed-zero` to better distinguish `0` from `O`.
  - Evidence (source): `src/styles/browser_fonts.css` `font-variant-numeric: slashed-zero` applies it to that narrow Citation URL input; `src/assets/fonts/ibm_plex_sans_condensed/provenance.txt` records the locally retained IBM Plex Sans Condensed Regular asset, OFL provenance, and its verified OpenType `zero` GSUB feature.
  - Evidence (runtime): `src/styles/browser_fonts.css` `font-variant-numeric: slashed-zero`: that same one-time Chromium fixture computed `slashed-zero` on the local IBM face without a fallback; it was removed rather than retained as a permanent implementation-coupled test.

- [x] Question Backend-rendered content may use its own fonts when needed for correct display.
  - Evidence (source): `src/styles/ple_embed.css` `Question-authored font rules remain in charge` keeps the readable baseline and leaves later Question, math, and icon families in effect.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `Question Backend-rendered content may use its own fonts when needed for correct display` kept Atkinson on ordinary text and plain fields, STIX Two Text on the later Question rule, MathJax_Main on the math element, and Question Widget on the classed answer field.

### Ribbon and page layout
- [x] The top Ribbon is the persistent navigation area for signed-in PLE pages.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` renders `AppRibbon` inside the persistent shell.

- [x] The Ribbon should remain in the same location and use the same overall structure while navigating.
  - Evidence (test): `tests/playwright/ribbon_responsive_evidence.mjs` `assertResponsiveRows` verifies declared Ribbon rows across route-model changes.

- [x] Navigation choices should remain in predictable locations as users move between related pages.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` and `RIBBON_TASK_CATALOG` own fixed navigation identities.

- [x] Each Tier 1 choice has one Tier 2 set. If Tier 1 stays the same, Tier 2 stays the same. Opening a Course, Assessment, Question, or other item does not change the Tier 2 choices or their order.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_TWO` declares one Tier 2 set for each Tier 1 area, independent of the open route.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `settled Instructor Tier 2 destinations and order stay fixed across deeper routes` compares Instructor rows for the same Tier 1 area.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Tier 2 choices and order stay fixed across routes and Course context` compares Student rows while opening Course and Coursework routes.

- [x] Changing a Ribbon selection should change the content below the Ribbon without moving the main content area up or down.
  - Evidence (source): `src/application_shell.tsx` `ContentRegion` keeps `BreadcrumbPrelude` in place and swaps the routed page when `pathname` changes.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `shell geometry drifts` compares breadcrumb and main-content tops across signed-in routes.

- [x] Ribbon rows should keep their space when needed so changing selections does not make the content area jump.
  - Evidence (source): `src/ribbon/app_ribbon.css` `--ple-ribbon-reserved-task-size` keeps the task row in the Ribbon grid.

- [x] Page actions should appear near the content they affect rather than changing the Ribbon layout.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` renders only catalog navigation and Sign Out in `AppRibbon`; task content stays in `ApplicationShell` content.

- [x] See **User top bar** and **Breadcrumbs** for the persistent elements that make up the top of the page.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` composes `AppRibbon` and `BreadcrumbPrelude`.

### User top bar interface
- [x] All signed-in users share the same top-left logo/account and top-right profile bar layout.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `AppRibbon` renders the shared leading `ple-app-ribbon__context-identity` logo/account block and shared trailing `ple-app-ribbon__profile-endcap` Profile control for every role model; `src/ribbon/app_ribbon.css` `ple-app-ribbon__profile-endcap` pins the Profile control at the inline end, while Student narrow rules adapt only the middle navigation arrangement.

- [x] The top bar remains in a consistent location as users navigate.
  - Evidence (test): `tests/playwright/ribbon_responsive_evidence.mjs` `assertResponsiveRows` measures the persistent top row across model changes.

- [x] The PLE logo and product name appear at the upper left and link to the user's home dashboard.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__brand` is the upper-left Peptidyle home link immediately after the role badge.

- [x] Each User Role has its own home dashboard and navigation.
  - Evidence (source): `src/pages/role_home_pages.tsx` `InstructorHomePage` opens the Instructor dashboard, `StudentHomePage` opens Student Coursework, and `SysadminHomePage` opens system administration.
  - Evidence (source): `src/route_contract.ts` `userRoleHomeRouteId` selects a different home route for each User Role.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_ONE` declares separate Tier 1 navigation for each User Role.

- [x] User Role appears once next to the PLE name.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__user-role` occurs once in the shared identity block beside `ple-app-ribbon__brand`.

- [x] Role-specific navigation appears between the product identity and Profile.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` places `ple-app-ribbon__tabs` after identity and before account controls.

- [x] Profile appears at the far right as an icon-only avatar.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__profile-endcap` renders the shared icon-only Profile button after the navigation region; `src/ribbon/app_ribbon.css` `ple-app-ribbon__profile-endcap` anchors that endcap at the inline end.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `every signed-in User Role has one accessible generic Profile end control` verifies the one accessible, text-free Profile control for Student, Instructor, and Sysadmin.
  - Decision: A one-time real-shell probe verified the isolated Profile control at 1280 and 320 CSS pixels with a coarse pointer, including thumbnail-request isolation; it was removed rather than retained as a permanent browser test.

- [x] Clicking the Profile avatar opens the Profile menu.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `openProfileMenu` controls the Profile trigger's `profileMenuOpen` state and renders the labelled `ple-profile-menu` menu.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Ribbon Profile menu contract: PASS` protects pointer and keyboard opening, focus, dismissal, and action dispatch.

- [x] I want the Profile menu to contain Profile and Sign Out.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-profile-menu` renders the Profile link and the Sign out action.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Ribbon Profile menu contract: PASS` checks that the open menu contains those two commands.

- [x] I want Profile to be the only page that names the Account's time zone; other pages should show times
  without repeating the zone name.
  - Evidence (source): `src/pages/profile_page.tsx` `profile-time-zone-heading` names the Account time zone.
  - Evidence (source): `src/format_datetime.ts` `createDisplayDateTimeFormatter` formats a time without a zone name.
  - Evidence (test): `tests/test_pending_invitations_model.mjs` `invitation expiry uses the explicit viewer zone` checks that the shown expiry omits the zone name.

- [x] Sign Out belongs in the Profile menu rather than the main top bar.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `data-ribbon-action={props.model.context.signOutAction.id}` renders Sign Out as a Profile-menu item and closes that menu after dispatch.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Ribbon Profile menu contract: PASS` verifies no top-bar Sign Out button and one dispatched Profile-menu Sign Out action.

- [x] See **Ribbon and page layout** for the overall navigation and page-position rules.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` is the shared shell that composes the top bar and content region.

### Profile avatar interface
- [x] Every Account is randomly assigned an avatar from the PLE avatar gallery when the Account is created.
  - Evidence (source): `schemas/base_schema/50_functions/profile_media.sql` `record_initial_account_avatar` chooses one selectable gallery avatar when an Account row is inserted.

- [x] The same avatar gallery collection is available to all User Roles.
  - Evidence (source): `src/pages/profile_page.tsx` `ProvidedAvatarPicker` presents the gallery for the signed-in Account.
  - Evidence (source): `src/features/profile_avatar/provided_avatar_picker.tsx` `selectableEntries` reads the one selectable catalog.

- [ ] The current avatar or Profile image appears consistently anywhere PLE represents that user.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Profile avatar interface boundary.
  - Owner: 03_shell.md / Profile avatar interface (first occurrence; identical requirement and status).

#### Student avatars
- [x] **Students** select avatars from the PLE-provided avatar gallery collection and cannot upload Profile images.
  - Evidence (source): `src/pages/profile_page.tsx` `ProvidedAvatarPicker` is the Student gallery, and `profileRoleMayManageImage` keeps image upload off that page.
  - Evidence (source): `src/features/profile_avatar/profile_avatar_role.ts` `profileRoleMayManageImage` admits image controls for Instructor and Sysadmin only.

- [x] Student avatar selection should be visual and playful.
  - Evidence (source): `src/features/profile_avatar/provided_avatar_picker.tsx` `RecordListImageBrowser` shows the gallery, and the introduction says the choice is playful.
  - Evidence (source): `src/components/record_list/record_list.css` `.record-list--gallery .record-list__media` gives each avatar a visual tile.

- [x] All avatars in the gallery are available for selection.
  - Evidence (source): `src/features/profile_avatar/provided_avatar_picker.tsx` `selectableEntries` offers each selectable catalog avatar as a radio.

- [x] Students may select another avatar at any time.
  - Evidence (source): `src/pages/profile_page.tsx` `selectProvidedAvatar` saves a new provided avatar from the Profile gallery.

#### Instructor and Sysadmin Profile images
- [x] **Instructors** and **Sysadmins** share the same Profile backend and functionality.
  - Evidence (source): `schemas/base_schema/50_functions/profile_media.sql` `prepare_account_profile_image` admits an Instructor or a Sysadmin to the same image preparation.
  - Evidence (source): `src/pages/profile_page.tsx` `ProfilePage` is the one Profile surface for every signed-in role.

- [x] **Instructors** and **Sysadmins** may select from the PLE avatar gallery or upload their own Profile image.
  - Evidence (source): `src/pages/profile_page.tsx` `StaffAvatarSettings` is shown when `profileRoleMayManageImage` is true.

- [x] Image upload accepts any aspect ratio with a minimum of 128 pixels in both dimensions.
  - Evidence (source): `src/api/decoders/profile_avatar.ts` `decodeProfileImageCropInput` requires both source sides to be at least 128 pixels.
  - Evidence (source): `src/features/profile_avatar/profile_image_crop.ts` `decodeProfileImage` applies that same minimum to the decoded image.
  - Evidence (test): `tests/test_profile_image_crop.mjs` `Profile crop preserves a square and reaches the chosen edges at each zoom` accepts wide and tall sources whose shorter side is 128 pixels.
  - Evidence (test): `tests/test_profile_image_crop.mjs` `Profile crop rejects undersized, oversized, and invalid crop inputs` rejects a side below that minimum.

- [x] After upload, Instructors and Sysadmins can position and crop the image within a square Profile preview.
  - Evidence (source): `src/features/profile_avatar/staff_avatar_settings.tsx` `PROFILE_IMAGE_SIDE_PIXELS` sizes the Profile preview canvas to a square.
  - Evidence (source): `src/features/profile_avatar/profile_image_crop.ts` `profileImageCrop` returns one square region from the chosen position and zoom.
  - Evidence (test): `tests/test_profile_image_crop.mjs` `Profile crop preserves a square and reaches the chosen edges at each zoom` moves that square to the source edges.

- [x] Instructors and Sysadmins may replace their Profile image or select a provided avatar at any time.
  - Evidence (source): `src/pages/profile_page.tsx` `selectProvidedAvatar` saves a gallery avatar from Profile while a Profile image can already be current.
  - Evidence (source): `src/features/profile_avatar/staff_avatar_settings.tsx` `replaceImage` submits a new Profile image through the signed-in Account.

- [ ] The current avatar or Profile image appears consistently anywhere PLE represents that user.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Profile avatar interface boundary.
  - Owner: 03_shell.md / Profile avatar interface (first occurrence; identical requirement and status).

### Breadcrumbs interface
- [x] All signed-in users have a permanent breadcrumb row below the top Ribbon.
  - Evidence (source): `src/application_shell.tsx` `BreadcrumbPrelude` is the first region inside `ContentRegion`, under `AppearanceRibbon`.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-ribbon-shell-grid` keeps the Ribbon in the first shell row.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `shellGeometry` requires the breadcrumb row on every signed-in shell materialization.

- [x] The breadcrumb row remains in the same location and keeps the same space as users navigate.
  - Evidence (source): `src/ribbon/app_ribbon_density.css` `.ple-shell__breadcrumb-prelude` reserves one fixed block size for the breadcrumb row.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `shell geometry drifts` fails when that row or the main content moves between signed-in routes.

- [x] Breadcrumbs show the path from the user's home dashboard to the current page.
  - Evidence (source): `src/ribbon/ribbon_contract.ts` `breadcrumbsFor` starts each signed-in trail at the role home.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `every signed-in route reserves linked breadcrumbs rooted at its role home` checks that the first crumb is the role home and the last crumb is the current route.

- [x] Each breadcrumb level links back to its corresponding page.
  - Evidence (source): `src/application_shell.tsx` `BreadcrumbPrelude` renders every crumb through `breadcrumb.href`.
  - Evidence (source): `src/ribbon/ribbon_contract.ts` `RibbonBreadcrumbModel` requires `href` on every crumb.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `every signed-in route reserves linked breadcrumbs rooted at its role home` checks that each crumb href is a declared route.

- [x] Breadcrumbs use human-readable names rather than internal identifiers.
  - Evidence (source): `src/application_shell.tsx` `BreadcrumbPrelude` renders `RibbonBreadcrumbModel.label`, not an ID.

- [x] Course and Assessment breadcrumbs preserve the current Course context.
  - Evidence (source): `src/ribbon/route_scope_controller.ts` `createRouteScopeController` resolves route labels while retaining course scope.

- [x] Keeping the breadcrumb row in place prevents the main content from moving up or down as breadcrumb depth changes.
  - Evidence (test): `tests/playwright/ribbon_shell_evidence.mjs` `label resolution preserves the reserved breadcrumb-prelude geometry` verifies stable shell geometry through deferred resolution.

- [x] See **Ribbon and page layout** for the overall page-position rules.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` composes the Ribbon and the breadcrumb row in one shell.
