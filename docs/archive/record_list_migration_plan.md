# Plan: Record presentation migration

## Context

The September 23 record-presentation audit supersedes the former `7 converted + 47 remaining`
arithmetic. It found that four entries treated as collections are native selectors or one selected
preview, that Gradebook and ordered Assessment entries need semantics beyond a flat list, and that
the prior inventory omitted current Student Course Progress, Attempt History, and Response Stats
collections. Current source also has recovery Attempt choices and discussion Impact notices that
must move with their enclosing files.

This is a frontend presentation migration. It preserves each page's fetching, filtering,
selection, editing, pagination, saving, authorization, Question controls, and specialized Question
navigation. The plan converts genuine repeated-record presentations through a small component
family. It does not turn native form controls, fixed comparisons, or Question content into record
collections.

## Verification approach

Start source work after inventory and component contracts are clear. Run focused checks for the
changed component or page, then run the fast UI checks and complete repository gate at integration.
If a check fails, fix or revert the affected bounded change and rerun that check. Split a separate
plan only when the failure reveals work outside this migration. Browser checks and populated
captures follow the implementation they verify. A running Live Demo rebuilds only a stale bundle
and does not establish rendered acceptance.

## Objectives

- Use a shared component whose native semantics match the demonstrated collection task.
- Keep Student Coursework compact and Course-identifying; keep long reviews readable; retain
  table headers and ordered or nested relationships.
- Give every collection a component, whole-file owner, and completion evidence in the ledger.
- Close this plan only after its ledger reflects current source and rendered evidence.

Human Guidance requires compact scan rows, dense Instructor tables with clear columns, Course
context across Student global views, precise keyboard access for reordering, and review units that
keep a Question's number, result, recorded response, and permitted feedback together.

## Shared presentation contracts

Pages own domain and workflow state. The component family owns collection markup, responsive
presentation, structural alignment, and loading, empty, and error states.

| Component                                   | Native structure and use                                                                                                                                                                                                                                                                                                           | Caller retains                                                           |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `RecordList<Row>`                           | Flat compact scans, actions, result rows, and galleries. It accepts `rows`, `recordId`, `regions`, exported `RecordCollectionState` (`ready`, `loading`, or `error`) as `state`, `ariaLabel`, and exported `RecordCollectionEmptyState` as `emptyState`.                                                                           | Search, filtering, selection, paging, windowing, and persistence.        |
| `RecordSequence<Row>`                       | `<ol>/<li>` records. It accepts `rows`, `recordId`, `regions`, the exported `state` and `emptyState` contracts, and `ariaLabel`.                                                                                                                                                                                                   | Move and remove controls, focus restoration, and saved order.            |
| `RecordTable<Row>`                          | A native scrolling `<table>` with explicit column headers, one row-header column, and cell renderers. It accepts `rows`, `rowId`, `columns`, `rowHeader`, the exported `state` and `emptyState` contracts, and `ariaLabel`. Columns may declare CSS widths and logical text alignment; shared styling fills and scrolls the table. | Grade calculations, roster actions, sorting, and data loading.           |
| `RecordOutlineList` and `RecordOutlineItem` | Composable nested native lists for parent and child records. `RecordOutlineList` accepts the exported `state` and `emptyState` contracts, `ariaLabel`, and `children`; `RecordOutlineItem` accepts `recordId` and `children`.                                                                                                      | Selection, structural edits, source/destination choice, and persistence. |
| `RecordDetailList<Row>`                     | Expanded entries whose caller renders an article, form, nested content, or paired comparison without compact-row hiding. It accepts `rows`, `recordId`, `renderRecord`, the exported `state` and `emptyState` contracts, and `ariaLabel`.                                                                                          | Review, form, discussion, and comparison behavior.                       |

`RecordListReorder` remains a composition for keyboard move controls and announcements. It does
not persist order. New components accept no page-specific props. `RecordCollectionState` and
`RecordCollectionEmptyState` are exported once for every component: `state` presents `ready`,
`loading`, or `error`, while `emptyState` presents the ready collection with no records.

## Row-by-row ledger

The ledger replaces a fixed total. Each row is a current collection surface, not a statement that a
source file contains only one collection. "Proof retained" means an existing implementation keeps
its current component. "Move" means the named whole file moves all coupled presentation surfaces
owned by that lane. Evidence is required before the row becomes complete.

| Collection and source                                                                 | Presentation                                                 | Whole-file owner     | Completion evidence                                                                     |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------ | -------------------- | --------------------------------------------------------------------------------------- |
| Question Drafts - `question_drafts_page.tsx`                                          | `RecordList` (proof retained)                                | L: Library/Blueprint | Existing focused browser proof; fresh populated row capture.                            |
| Library Question results - `library_browse_rows.tsx`                                  | `RecordList` (proof retained)                                | L: Library/Blueprint | Existing windowing proof; search and row capture.                                       |
| Course Assessment actions - `course_instance_page.tsx`                                | `RecordList` (proof retained)                                | L: Instructor        | Existing action-row proof; populated Course capture.                                    |
| Student Coursework - `student_course_landing_page.tsx`                                | `RecordList` (proof retained)                                | L: Student           | Existing four-viewport proof; compact identity, Course, state, and action check.        |
| Provided avatar catalog - `provided_avatar_picker.tsx`                                | `RecordList` (proof retained)                                | L: Student           | Existing gallery/list selection proof.                                                  |
| Student Courses - `student_courses_page.tsx`                                          | `RecordList` gallery/list                                    | L: Student           | Populated Course gallery at laptop and phone widths.                                    |
| Student Course Invitations - `student_course_invitations_page.tsx`                    | `RecordList` gallery/list                                    | L: Student           | Course, Instructor, term, and action stay visible at narrow width.                      |
| Account pending invitations - `account_pending_invitations_page.tsx`                  | `RecordList`                                                 | L: Student           | Invitation state and next action capture.                                               |
| Student Scores - `student_course_grades_page.tsx`                                     | `RecordList`, Course identified per item                     | L: Student           | Multi-Course populated laptop and phone capture.                                        |
| Student Course Progress - `student_course_progress_page.tsx`                          | Course-grouped `RecordList`                                  | L: Student           | Released score and "Score not released" row capture.                                    |
| Student Attempt History - `student_course_attempt_history_page.tsx`                   | Course-grouped `RecordList`                                  | L: Student           | Independent per-Course pagination and Course identification check.                      |
| Student Response Stats - `student_course_response_stats_page.tsx`                     | Course-grouped `RecordDetailList`                            | L: Student           | Disclosed counts, duration, review action, and narrow-width capture.                    |
| Assessment overview prior Attempts - `assessment_overview_page.tsx`                   | `RecordList`                                                 | L: Student           | Prior-Attempt identity and review link capture.                                         |
| Instructor Course lists - `course_list_page.tsx`                                      | `RecordList`                                                 | L: Instructor        | Active and inactive Course scans retain dates, classification, and action.              |
| Assessments Due Soon - `assessments_due_soon_page.tsx`                                | `RecordList`                                                 | L: Instructor        | Temporary populated capture; Course, state, due time, action check.                     |
| Assessment Templates - `assessment_templates_page.tsx`                                | Selectable `RecordList`                                      | L: Instructor        | Selection, editor dirty-state guarding, and return focus check.                         |
| Question statistics by revision - `question_statistics_panel.tsx`                     | `RecordList`                                                 | L: Library/Blueprint | Revision and metric remain coupled in a populated panel.                                |
| Question use by Course - `question_statistics_panel.tsx`                              | `RecordList`                                                 | L: Library/Blueprint | Course identity and usage metric capture.                                               |
| Pool discovery results - `library_pool_discovery.tsx`                                 | `RecordList`                                                 | S: Sequence          | Search/facet state, result row, and selected detail capture.                            |
| QuestionPicker search results - `question_picker.tsx`                                 | Selectable `RecordList`                                      | S: Sequence          | Filter, selection, and exact Question Revision check.                                   |
| Available Assessment Questions - `assessment_workspace_questions_view.tsx`            | `RecordList`                                                 | S: Sequence          | Inspect/Add actions retain exact Question Revision identity.                            |
| Fixed Question points - `assessment_fixed_question_points_editor.tsx`                 | `RecordList` with caller-owned inputs                        | S: Sequence          | Input validation and identity alignment check.                                          |
| Course Blueprint update Assessments - `course_blueprint_update_review.tsx`            | `RecordList`                                                 | L: Instructor        | Proposed Assessment action rows and confirmation flow.                                  |
| Public Blueprint Courses - `blueprint_course_search_page.tsx`                         | `RecordList`                                                 | L: Library/Blueprint | Visible search, filters, sort, and populated results.                                   |
| My Blueprint Courses - `blueprint_courses_workspace.tsx`                              | `RecordList`                                                 | L: Library/Blueprint | Metadata scan, sorting, and selection check.                                            |
| Question Pool picker results - `question_pool_picker.tsx`                             | Selectable `RecordList`                                      | S: Sequence          | Radio selection and detail loading capture.                                             |
| Blueprint history revisions - `blueprint_history.tsx`                                 | `RecordList`                                                 | O: Outline           | Revision selection and history-state capture.                                           |
| Blueprint Watch activity - `src/features/blueprint_course/blueprint_stewardship.tsx`  | `RecordList`                                                 | L: Library/Blueprint | Expanded disclosure event chronology capture.                                           |
| Question star identities - `src/components/question_star_control.tsx`                 | `RecordList` for every nonempty repeated identity collection | L: Library/Blueprint | Populated Instructor disclosure capture.                                                |
| Blueprint star identities - `src/features/blueprint_course/blueprint_stewardship.tsx` | `RecordList` for every nonempty repeated identity collection | L: Library/Blueprint | Populated Instructor disclosure capture.                                                |
| Instructor Accounts - `instructor_accounts_page.tsx`                                  | `RecordList` with caller-owned inputs                        | L: Instructor        | Account state, reason input, and action capture.                                        |
| Library Watch notifications - `library_watch_notifications_page.tsx`                  | `RecordList`                                                 | L: Library/Blueprint | Event identity, time, and target action capture.                                        |
| Known Blueprint forks - `blueprint_fork_review.tsx`                                   | `RecordList`                                                 | D: Detail            | Populated fork summary and open action.                                                 |
| Change Proposals - `proposal_workspace.tsx`                                           | `RecordList`                                                 | L: Library/Blueprint | Paging, source/target, state, and date capture.                                         |
| Ordered Assessment entries - `assessment_workspace_questions_view.tsx`                | Move from `RecordList` to `RecordSequence`                   | S: Sequence          | Native order; keyboard move, focus, remove, and saved order checks.                     |
| QuestionPicker selection tray - `question_picker.tsx`                                 | `RecordSequence`                                             | S: Sequence          | Returned Question ID order and keyboard move/remove check.                              |
| Assessment Pool members - `assessment_pool_entry_editor.tsx`                          | `RecordSequence`                                             | S: Sequence          | Exact Question Revision, member order, move, remove, and save check.                    |
| Blueprint Pool members - `blueprint_pool_members_editor.tsx`                          | `RecordSequence`                                             | S: Sequence          | Pinned revision order and persistence check.                                            |
| Blueprint Assessment content - `blueprint_assessment_content_editor.tsx`              | `RecordSequence`                                             | S: Sequence          | Mixed Question/Pool entry order and edit controls.                                      |
| Pool detail members - `library_pool_discovery.tsx`                                    | `RecordSequence`                                             | S: Sequence          | Read-only pinned revision order in selected detail.                                     |
| Pool picker selected members - `question_pool_picker.tsx`                             | `RecordSequence`                                             | S: Sequence          | Selected-Pool member order and revision identity.                                       |
| Pool creation confirmation - `question_pool_create_dialog.tsx`                        | `RecordSequence`                                             | S: Sequence          | Ordered form confirmation and submit behavior.                                          |
| Assessment content in update summaries - `assessment_blueprint_update_review.tsx`     | Nested `RecordSequence` inside caller comparison             | S: Sequence          | Current/proposed ordered contents remain separately labeled.                            |
| Blueprint Course detail - `blueprint_course_detail_workspace.tsx`                     | `RecordOutlineList` / `RecordOutlineItem`                    | O: Outline           | Module-to-Assessment membership, order, selection, and editing check.                   |
| Blueprint history snapshot - `blueprint_history.tsx`                                  | `RecordOutlineList` plus nested `RecordSequence`             | O: Outline           | Immutable Revision to Module to Assessment to entry relationships.                      |
| Fork destination layout - `blueprint_fork_apply.tsx`                                  | `RecordOutlineList` plus nested `RecordSequence`             | O: Outline           | Structural move/removal changes intended Module or Assessment.                          |
| Attempt summary Question reviews - `assessment_attempt_summary_page.tsx`              | `RecordDetailList`                                           | D: Detail            | Long feedback, multipart response, several Questions, and four widths.                  |
| Recovery Attempt choices - `course_student_work_recovery.tsx`                         | `RecordList` selectable records                              | D: Detail            | Keyboard radio selection and selected-Attempt recovery check.                           |
| Recovery Question evidence - `course_student_work_recovery.tsx`                       | `RecordDetailList`                                           | D: Detail            | Response, grading, unavailable evidence, and narrow-width review.                       |
| Bulk metadata current values - `question_bulk_metadata_editor.tsx`                    | `RecordDetailList`                                           | D: Detail            | Full current values remain comparable before edit submission.                           |
| Discussion threads and posts - `library_discussion_panel.tsx`                         | `RecordDetailList` with caller-nested posts                  | D: Detail            | Chronology, reply/edit, resolve state, and narrow-width capture.                        |
| Discussion Impact notices - `library_discussion_panel.tsx`                            | `RecordDetailList`                                           | D: Detail            | Active/cancelled notice state and action confirmation.                                  |
| Content Disciplines - `content_disciplines_page.tsx`                                  | `RecordDetailList` with caller forms                         | D: Detail            | Form labels, validation, save, and narrow-width layout.                                 |
| Fork Assessment differences - `blueprint_fork_review.tsx`                             | `RecordDetailList` paired comparisons                        | D: Detail            | Before/after values, expand state, and relationship clarity.                            |
| Gradebook - `gradebook_page.tsx`                                                      | Move from `RecordList` to `RecordTable`                      | T: Table             | Header, row-header, data-cell relationships; laptop and phone horizontal-scroll review. |
| Course roster - `course_roster_page.tsx`                                              | `RecordTable`                                                | T: Table             | Student/State/Action headers, roster actions, and laptop/phone review.                  |

## Explicit dispositions

The following source renders remain outside the ledger because they are controls, content, or fixed
comparisons rather than repeated records.

| Surface                                                                                                                                                                 | Disposition                                                                                                                                   | Owner                                  |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| `src/pages/course_list_page.tsx` Blueprint source chooser                                                                                                               | Native `<select>` options.                                                                                                                    | L: Instructor                          |
| `src/pages/course_appearance_page.tsx`; `src/pages/profile_page.tsx` Account/time-zone and Course appearance choices                                                    | Fixed and enumerated settings fields; these controls edit one Course or Account rather than present records.                                  | C: disposition owner (no source edits) |
| `src/pages/assessment_workspace/assessment_workspace_create_page.tsx`; Assessment Type and Template selectors in `src/pages/assessment_templates_page.tsx`              | Native `<select>` choices for creating or editing one Assessment or Template; the Template results list remains a separate ledger collection. | L: Instructor                          |
| `src/pages/assessment_workspace/assessment_student_time_accommodations.tsx` Student chooser                                                                             | Native `<select>` followed by one selected Student form.                                                                                      | L: Instructor                          |
| `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` available Pool chooser                                                                         | Native `<select>` options.                                                                                                                    | S: Sequence                            |
| `src/pages/assessment_workspace/assessment_workspace_student_view_page.tsx`                                                                                             | Numbered Question navigation plus one selected answer-free preview.                                                                           | L: Student                             |
| `src/pages/assessment_workspace/assessment_pool_entry_editor.tsx` candidate chooser                                                                                     | Native `<select>` options that feed the ordered member editor.                                                                                | S: Sequence                            |
| `src/components/student_assessment_attempt_navigation.tsx`                                                                                                              | Task-specific Question navigation; preserve its navigation semantics.                                                                         | C: disposition owner (no source edits) |
| `src/features/blueprint_change_proposal/proposal_review.tsx` source/target pair                                                                                         | Fixed source/target comparison.                                                                                                               | C: disposition owner (no source edits) |
| `src/features/blueprint_forks/blueprint_fork_review.tsx` left/right pair                                                                                                | Fixed comparison sides; its repeated fork records remain in D's ledger rows.                                                                  | D: Detail                              |
| `src/features/ple_question_json_authoring/question_json_*.tsx`                                                                                                          | Authored answer alternatives, prompts, labels, and response fields inside one Question.                                                       | C: disposition owner (no source edits) |
| `src/components/question_response_controls/*.tsx`                                                                                                                       | Question response controls inside one delivered Question.                                                                                     | C: disposition owner (no source edits) |
| `src/components/question_response_preview.tsx`                                                                                                                          | One answer-free Question preview with authored content and response fields; keep its Question-specific structure.                             | C: disposition owner (no source edits) |
| `src/components/question_renderer.tsx`, `src/components/student_feedback_panel.tsx`                                                                                     | Authored content blocks and embedded data tables with content-specific semantics.                                                             | C: disposition owner (no source edits) |
| `src/pages/library_page.tsx`, `src/pages/library_browse_controls.tsx`, `src/components/library_bloom_discovery.tsx`, `src/components/content_classification_select.tsx` | Search facets, filter values, and facet counts that control discovery.                                                                        | L: Library/Blueprint                   |
| `src/pages/library_pool_discovery.tsx`, `src/features/question_picker/question_picker.tsx` discovery controls                                                           | Search facets and filter values; their result and selected-record collections remain in S's ledger rows.                                      | S: Sequence                            |
| `src/application_shell.tsx`, `src/ribbon/app_ribbon.tsx`                                                                                                                | Route, breadcrumb, and task-area navigation controls.                                                                                         | C: disposition owner (no source edits) |
| `src/pages/question_detail_page.tsx` authors and aggregate measures                                                                                                     | Inline contributors and name/value properties of one Question.                                                                                | C: disposition owner (no source edits) |
| Blueprint settings `<dl>` in `src/features/blueprint_forks/blueprint_fork_review.tsx`                                                                                   | Name/value properties of one comparison side; the repeated Assessment difference records remain in D's ledger rows.                           | D: Detail                              |
| `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx`, `src/pages/assessment_template_settings_editor.tsx`                                            | Fixed policy fields and validation messages.                                                                                                  | C: disposition owner (no source edits) |

## Work packages and sequencing

| Package               | Owner                                         | Result and success condition                                                                                            | Validation                                                                                                                                                                                                                                                                                                                            |
| --------------------- | --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| I - inventory         | Plan owner                                    | This ledger stays current and each genuine surface has one disposition and whole-file owner.                            | Reconcile audit, Graphify/source paths, and ledger before integration.                                                                                                                                                                                                                                                                |
| F - shared components | Shared UI owner                               | Shared states plus the four sibling contracts are implemented without page-specific props.                              | Keep focused browser assertions for ordered-list, table-header and row-header, nested-list, and keyboard behavior. Exercise shared collection states through `RecordList` and `RecordSequence`; verify sibling ready-state semantics in the same family harness. Use one-time checks for exact geometry and populated page workflows. |
| T - tables            | Table owner                                   | Gradebook and roster use `RecordTable`; active table styles are incorporated after a fresh last-user stylesheet check.  | T runs focused browser/page-fixture checks for both consumers and populated laptop/phone captures for their table semantics.                                                                                                                                                                                                          |
| S - sequences         | Sequence owner                                | Ordered Assessment, picker, Pool, and Blueprint content collections use `RecordSequence`.                               | S runs focused browser/page-fixture checks for its consumers and populated captures for order, exact revision identity, keyboard move, focus, removal, and save.                                                                                                                                                                      |
| O - outlines          | Outline owner                                 | Blueprint detail, history, and fork layout use outline components as complete files.                                    | O runs focused browser/page-fixture checks for its consumers and populated captures for parent-child membership, order, selection, and structural editing.                                                                                                                                                                            |
| D - details           | Detail owner                                  | Attempt/recovery review, bulk metadata, discussion, Discipline, and fork review use expanded records as complete files. | D runs focused browser/page-fixture checks for its consumers and populated captures for long feedback, nested posts, forms, paired differences, and narrow layouts.                                                                                                                                                                   |
| L - flat pages        | Student, Instructor, Library/Blueprint owners | Direct-fit scans use `RecordList` with their existing workflow state.                                                   | Each lane owner runs focused browser/page-fixture checks for its consumers and populated captures showing states, primary action, Course context, and relevant viewports.                                                                                                                                                             |
| C - integration       | Integration owner                             | Ledger, architecture/design documents, changelog, and plan closure agree with source and evidence.                      | Required gates, fresh screenshots, Markdown links, `git diff --check`, and final ledger review.                                                                                                                                                                                                                                       |

I and F precede page migration. T, S, O, D, and L may proceed after F fixes the interfaces. Source
edits begin once the inventory and component interfaces are settled; browser UI checks follow the
affected implementation. A
whole file has one owner: coupled surfaces such as QuestionPicker, Pool discovery, Blueprint
history, fork review, recovery, and the discussion panel stay within the named owner's package.
T, S, O, D, and each Student, Instructor, and Library/Blueprint lane own and run their focused
browser or page-fixture checks and populated captures. They reuse an existing stable contract
where it covers the consumer; a temporary populated capture supplies the remaining evidence, so
the migration does not create seven permanent fixtures solely for this plan.

## Acceptance and integration

- Run focused checks for the changed package, then the fast UI checks and full repository gate at
  integration. If a gate fails, fix or revert the affected bounded change and rerun its check.
- Review populated pages at widths that exercise changed responsive behavior. This migration needs
  laptop and phone review for compact rows and horizontally scrolling tables; add an intermediate
  width only when a changed breakpoint affects content or action access. Use temporary populated
  evidence when the screenshot corpus lacks a useful example.
- Verify table headers and row headers, ordered positions, nested membership, and accessible
  actions directly. Preserve independent Attempt History pagination for each Course.
- Retain Student Course identification in Coursework, Scores, Progress, Attempt History, and
  Response Stats. Keep Course-specific views visibly tied to their Course.
- A fresh last-user search found no users of `instructor_data_tables.css`, so the obsolete file was
  removed. Gradebook and roster now use `RecordTable`; `record_family.css` owns their shared table
  skin and scrolling, while each page keeps its minimum width and column proportions.
- At integration run fast UI checks, fresh screenshots, the complete repository gate,
  Markdown-link validation, and `git diff --check`. Update the changelog after each completed
  package. Close this plan only when all ledger evidence is present.

## Implementation and verification record - 2026-09-25

The shared component family and every source migration in the ledger are implemented. The Gradebook
and roster stylesheet was removed after its last-user search; architecture, design, and changelog
records describe the current component ownership.

The fresh complete screenshot corpus, `./launchers/run_fast_ui_checks.sh`, and
`source ./source_me.sh && ./launchers/all_test.sh` passed. The full gate included the database
persistence oracle, ordinary installation-data provision and replay, and the PostgreSQL/MinIO
Course-appearance coherence oracle. Browser and Live Demo commands ran outside the sandbox. The
Markdown-link test passed (325 tests), and `git diff --check` passed.

The fresh corpus was visually reviewed for Gradebook, the Course roster, ordered Assessment
entries, Blueprint selection, and Student Progress, Response Stats, Attempt History, and full
feedback. A temporary production-browser capture additionally verified a populated Module-to-
Assessment outline with selection and return, and a narrow Course roster with all three headers and
actions retained through horizontal scrolling. The roster table measured 334 px visible width and
704 px content width; the browser reported no page errors. Temporary captures are in
`/private/tmp/record-presentation-evidence/`.

A current-source check matched the component named in all 56 ledger rows. It caught Known Blueprint
fork summaries using `RecordDetailList` despite their compact scan role; they now use `RecordList`,
while the selected paired comparison remains separate. A temporary browser fixture verified the
populated list semantics, open and compare actions, keyboard traversal, narrow metadata collapse,
and absence of page or console errors. The fast offline and Chromium UI gates passed after this
correction.

Temporary current-source browser fixtures subsequently verified the populated historical Blueprint
snapshot and fork destination. The history view retained Module-to-Assessment membership and the
exact ordered Question Revisions in a native `<ol>`. The destination editor retained nested lists,
and direct browser actions moved a Module, moved an Assessment between Modules, removed the intended
Assessment and Module, restored the Module, and cancelled back to the saved layout. Populated fork
comparison details retained distinct left/right Assessments and their current differences.

The same fixture verified Course identification, disclosed counts, duration, and the Review action
in Student Response Stats; a three-Question Attempt review with long feedback and a response table;
keyboard selection and recovered Question evidence; bulk metadata current values; nested discussion
posts and active/cancelled Impact notices; and editable Content Discipline records. These populated
views were inspected at laptop and phone widths. Each phone view retained its content and actions
without horizontal page overflow. Known Forks also passed open/compare keyboard traversal and kept
its identity and actions when lower-priority metadata collapsed. Chromium reported no page or console
errors. Temporary screenshots and populated captures are under
`/private/tmp/record-presentation-evidence/`; the one-time ignored fixture was removed at closeout.

Final verification after the Known Forks correction passed `source ./source_me.sh &&
./launchers/all_test.sh`, including 9,439 Python tests, Rust and browser-code checks, database
persistence, installation-data provision/replay, and the PostgreSQL/MinIO Course-appearance oracle.
`source ./source_me.sh && ./launchers/run_fast_ui_checks.sh` passed outside the sandbox. A fresh
`source ./source_me.sh && ./devel/capture_screenshots.sh --fresh --verify` passed screenshot manifest
closure, privacy, and artifact-integrity checks; the replay retained 149 byte-different PNGs for
review. Direct comparison of the migration-related screenshots found only seeded IDs/times and
persona accents changing, with the same rendered structure. Markdown-link validation passed (325
tests) and `git diff --check` passed.

A final table-family review moved the duplicate Gradebook and roster skin into `record_family.css`,
replaced invalid `minmax()` width hints with percentages, and added logical column alignment to
`RecordTableColumn`. The browser contracts now check full-width tables and last-column edges on
laptop, named headers and row headers, retained roster actions, and narrow horizontal scrolling
without page overflow. The focused Chromium lane passed outside the sandbox; current-source
Gradebook and roster captures were inspected at laptop and phone widths with no browser errors.

The final `source ./source_me.sh && ./launchers/all_test.sh` passed after those changes, including
9,439 Python tests and all three real-service oracles. `source ./source_me.sh &&
./devel/capture_screenshots.sh --fresh` published the complete current screenshot corpus and atlas;
the updated Gradebook and roster captures were visually reviewed. Chromium ran outside the sandbox.

All 56 ledger sources match their named component and every collection has a disposition.
Implementation and verification are complete. Repository style requires closing the plan with
`git mv` to `docs/archive/`; `.git` is read-only in this environment, so plan closure remains
pending until the required index-writing move can run.
