# UI density and layout audit

Date: 2026-09-17. Status: read-only audit of the current canonical screenshot corpus. No code
changed. Fix packages below are proposals awaiting approval.

## Scope and method

Evidence: the 76 captures under `docs/screenshots/` refreshed on 2026-09-17 (see
`docs/screenshots/current_capture_manifest.json`), plus the CSS that draws them (`src/style.css`,
`src/pages/instructor_data_tables.css`, `src/pages/course_list_page.css`,
`src/ribbon/app_ribbon.css`). Screenshots prove presentation at 1280x800, 800x1280, and 390x844.
They do not prove runtime motion, so the "button moves" finding is inferred from where status
regions are inserted in the DOM order and needs the Playwright oracle in the validation section.

Method: heuristic evaluation against the guideline ledger in
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) (Interface design, Information density and layout,
Instructor interface, Student interface), with a cognitive walkthrough of the four scan-heavy
Instructor tasks (find a Course, find a Question, check who has finished, check what is due).
Comparison patterns come from [ADAPT_UI_AUDIT.md](ADAPT_UI_AUDIT.md). Prior concrete findings
I01-I17 and S01-S08 in [UI_UX_USABILITY_AUDIT.md](UI_UX_USABILITY_AUDIT.md) remain valid; this
audit explains why they keep recurring and proposes the shared fix rather than per-page patches.

HG rules cited by short name below:

| Short name | HG rule (Information density and layout unless noted) |
| --- | --- |
| HG-space | Treat screen space as a limited resource. Prefer useful information over decorative whitespace. |
| HG-cards | Cards and rounded containers should earn their space by representing a distinct object or interaction. Avoid the dashboard style of large rounded cards, generous padding, isolated islands. |
| HG-rows | Present short labels and values in aligned rows or compact grids. |
| HG-spreadsheet | Instructor lists and repeated records should be dense and easy to scan, more like a spreadsheet than cards; favor compact rows or tables with clear columns. (Instructor interface) |
| HG-fold | At 1280 x 800, Instructor pages should expose enough of the current workflow to minimize unnecessary scrolling. |
| HG-jump | Changing a Ribbon selection changes the content below the Ribbon without moving the main content area; Ribbon rows keep their space. (Ribbon and page layout) |
| HG-helper | Use concise helper text near the control it explains. Present shared explanations once per relevant group. (General interface design) |
| HG-disclosure | Progressive disclosure keeps common tasks compact; expandable sections for longer details. (Interaction design) |
| HG-compact-cw | Keep Coursework entries compact in height so Students can scan several items at once. (Student interface) |

## Coverage ledger

All 76 captures were viewed. Grouped by surface so the per-surface findings can be traced back:

| Group | Captures | Count |
| --- | --- | --- |
| Instructor collections | `course_list`, `blueprint_courses`, `inactive_courses_list`, `question_drafts`, `gradebook`, `course_roster_active`, `course_roster_pending_invitation`, `course_assignment_workspace`, `public_blueprint_search`, `assignments_due_soon_empty` | 10 |
| Instructor Library and detail | `question_library`, `question_library_filtered`, `question_library_browse`, `published_question_detail`, `blueprint_question_picker`, `question_pool_creation_review`, `webwork_chi_square`, `webwork_chromosome_shapes`, `webwork_dna_structure`, `webwork_generated_example`, `webwork_hla_genotype`, `webwork_meiosis_prophase`, `webwork_monohybrid_matching`, `webwork_x_linked_counts` | 14 |
| Instructor editors and forms | `assignment_creation`, `assignment_release_draft`, `assignment_release_preview`, `assignment_release_released`, `assessment_template_editor`, `draft_editor_saved`, `publication_review`, `blueprint_course_detail`, `profile_default` | 9 |
| Student Course and Coursework | `course_list_laptop`, `course_list_phone`, `course_not_started_laptop`, `course_not_started_phone`, `course_in_progress_laptop`, `course_completed_laptop`, `invitation_index_laptop`, `invitation_detail_laptop`, `assignment_overview_unanswered_laptop`, `assignment_overview_unanswered_tablet`, `assignment_overview_history_laptop` | 11 |
| Student attempt, one per Question Type at laptop and phone | `question_mc`, `question_ma`, `question_fib`, `question_multi_fib`, `question_num`, `question_match`, `question_order`, `question_hotspot`, `question_webwork` (each `_laptop` and `_phone`) | 18 |
| Student attempt state and review | `assignment_attempt_saved_laptop`, `assignment_attempt_resumed_tablet`, `assignment_attempt_submitted_phone`, `assignment_attempt_summary_laptop` | 4 |
| Student denial | `authorization_denial_laptop`, `authorization_denial_phone` | 2 |
| Public | `sign_in_laptop`, `sign_in_phone`, `session_renewal_laptop` | 3 |
| Sysadmin | `system_administration_home_laptop`, `instructor_accounts_initial_laptop`, `instructor_account_created_laptop`, `instructor_account_validation_laptop`, `instructor_account_deactivated_laptop` | 5 |

## Summary verdict

The product has one coherent visual vocabulary (Atkinson, teal/green/purple role colors, restrained
radii). The bloat is not a styling defect; it is four structural habits applied to every page:

1. Every page spends the first 190-260 px on an eyebrow, a display-size H1, a lede sentence, and
   often a second explanatory sentence before any data appears.
2. Every repeated record is rendered as an object card (kicker line, bold title, metadata line,
   date line, right-aligned pill and button), 90-130 px tall, so 1280x800 shows two to six records.
3. Label/value pairs are stacked vertically (label above value) instead of laid out as columns, so
   values wrap under their labels and row height doubles.
4. Status and validation messages are inserted into normal document flow above or between the
   controls that produced them, so every save, release, or error pushes the form and its buttons
   down.

Fixing the four shared patterns in `src/style.css` and one new shared table style will resolve most
of the per-page findings at once. Per-page work then reduces to choosing columns.

## Fold budget at 1280x800

HG-fold makes the first 800 px the deliverable. This table splits that budget into fixed chrome
(top bar, Ribbon second row, breadcrumb band: 145 px on every signed-in page), page header (from
the breadcrumb to the first data pixel), and data (what remains). Percentages are of 800 px.

| Capture | Chrome | Page header | Data | Data share | Records or controls visible |
| --- | --- | --- | --- | --- | --- |
| `instructor/course_list.png` | 145 | 180 | 475 | 59% | 2 Courses; 210 px empty below |
| `instructor/blueprint_courses.png` | 145 | 370 | 285 | 36% | 2 Blueprints, second cut off |
| `instructor/course_assignment_workspace.png` | 145 | 185 | 470 | 59% | 4 Assessments, fourth cut off |
| `instructor/question_drafts.png` | 145 | 180 | 475 | 59% | 9 Drafts, ninth cut off |
| `instructor/gradebook.png` | 145 | 235 | 420 | 53% | 7 rows, seventh cut off |
| `instructor/course_roster_active.png` | 145 | 110 | 545 | 68% | 4 rows; 260 px empty below |
| `instructor/question_library_filtered.png` | 145 | 655 | 0 | 0% | 15 filter controls, no result |
| `instructor/question_library_browse.png` | 145 | 355 | 300 | 38% | 12 count rows, Bloom heading cut off |
| `instructor/published_question_detail.png` | 145 | 240 | 415 | 52% | preview, metadata, Star/Watch; Learning evidence cut off |
| `instructor/webwork_chi_square.png` | 145 | 205 | 450 | 56% | preview, metadata; Star/Watch cut off |
| `instructor/assignment_release_draft.png` | 145 | 265 | 390 | 49% | 1 entry; add controls cut off |
| `instructor/assignment_release_released.png` | 145 | 385 | 270 | 34% | 1 textarea, 2 date inputs cut off |
| `instructor/assessment_template_editor.png` | 145 | 215 | 440 | 55% | 3 templates; first setting cut off |
| `sysadmin/instructor_accounts_initial_laptop.png` | 145 | 270 | 385 | 48% | 2 accounts, second cut off |
| `student/course_in_progress_laptop.png` | 145 | 170 | 485 | 61% | 1 Coursework item plus 40% of the next |
| `student/question_mc_laptop.png` | 145 | 160 | 495 | 62% | 1 Question; Submit cut off |
| `student/assignment_attempt_summary_laptop.png` | 145 | 190 | 465 | 58% | 1.5 reviewed Questions |

Reading the table:

- Chrome alone is 18% of the viewport before any page draws. The empty Ribbon second row is 40
  of those 145 px on pages that have no second row.
- Page headers run 110-385 px on ordinary pages and 655 px on Library search. Median 215 px,
  27% of the viewport, spent on text that names the page the Ribbon already named.
- Data share is under 60% on 12 of 17 pages, under 40% on 4, and 0% on the page HG calls the
  primary discovery surface.
- Two pages (Active Courses, Roster) leave 200 px or more empty below their data because there
  are only a few records; the header did not shrink to compensate.
- Every list page cuts its last visible record in half, which is the visual signature of rows
  sized by padding rather than content.

Target after P1-P3: chrome 105 px (second row collapsed where absent), page header 60 px or
less, data share 80% or more on list pages, and no page whose first data pixel is below y = 200.

## Root causes with evidence

### R1: page header stack consumes the first quarter of the viewport

Evidence at 1280x800 (y coordinate of first data row):

| Capture | First record starts at | Header content above it |
| --- | --- | --- |
| `instructor/course_list.png` | y = 325 | eyebrow, H1, two-line lede, disclosure link |
| `instructor/blueprint_courses.png` | y = 515 | eyebrow, H1, link, lede, bold prompt, card header, card lede, sort control |
| `instructor/question_drafts.png` | y = 325 | eyebrow, H1, lede, primary button |
| `instructor/gradebook.png` | y = 380 | eyebrow, H1, lede, second sentence, two download buttons, table header |
| `instructor/course_assignment_workspace.png` | y = 330 | eyebrow with reference ID, H1, section eyebrow, H2, lede |
| `sysadmin/instructor_accounts_initial_laptop.png` | y = 415 | eyebrow, H1, two-line lede, create form card, time-zone sentence |
| `student/course_in_progress_laptop.png` | y = 315 | eyebrow, H1, quiet link, H2 |

Owners: `h1` at `src/style.css:291` (`clamp(1.6rem, 2.4vw, 2.1rem)`, `max-width: 28ch`),
`.page-lede` at `src/style.css:309` (`margin: 0.35rem 0 0.85rem`), and the empty 40 px band
between the Ribbon and the breadcrumb on Course-scoped and Profile pages (visible in every
`instructor/course_*`, `instructor/assignment_*`, `instructor/gradebook.png`,
`instructor/profile_default.png`, `student/course_*`, and all `sysadmin/*` captures).

The empty band satisfies HG-jump (rows keep their space) but violates HG-space: 40 px of gradient
with nothing in it on pages that have no second Ribbon row. Violates HG-fold.

### R2: collections render as object cards, not rows

Evidence:

| Capture | Records visible at 1280x800 | Row height | Pattern |
| --- | --- | --- | --- |
| `instructor/course_list.png` | 2 | 130 px | `.instructor-list__row`: kicker "COURSE INSTANCE", title, classification line, date line, theme pill, button |
| `instructor/blueprint_courses.png` | 2 (partial) | 155 px | kicker-less stack of title, counts, ownership sentence, classification line |
| `instructor/course_assignment_workspace.png` | 4 | 120 px | kicker "ASSESSMENT n - RELEASED", title, reference ID line, due line, two text buttons |
| `instructor/question_drafts.png` | 9 | 55 px | closest to target; two-line identity plus stacked state column |
| `instructor/gradebook.png` | 7 | 63 px | real `<table>` but every cell is two lines (name over roster ID, title over reference ID) |
| `instructor/public_blueprint_search.png` | 1 | 75 px | card with rail; counts and Open link on one line (good), title and revision stacked |
| `sysadmin/instructor_accounts_initial_laptop.png` | 2 | 190 px | card per account: avatar+ID heading, "State:" line, "Last sign-in:" line, inline deactivate form |
| `student/course_in_progress_laptop.png` | 1.5 | 330 px | card with kicker, title, Type/Completion two-column grid, score line, divider, "Can start", time-zone line, Due/Time limit/Attempt limit grid, disclosure, button |
| `student/assignment_overview_history_laptop.png` | 1 attempt row | 35 px | "Previous attempts" list is a good compact row model |

Owners: `.instructor-list__row` at `src/style.css:327` (`min-block-size: 4.25rem`, padding
`0.75rem 1rem`, border rail, `border-radius`, card background), `.course-card` at
`src/style.css:388` (`min-height: 5.75rem`, four stacked grid rows), `--ple-list-row-min-block-size`
and `--ple-dense-row-min-height: 3.6rem` at `src/style.css:47-48`. The kicker line
(`.instructor-list__kind`, uppercase 0.78rem) repeats the same text on every row ("COURSE
INSTANCE", "ASSESSMENT n - RELEASED"), which is a column header masquerading as row content.

Violates HG-cards, HG-spreadsheet, HG-fold, HG-compact-cw.

### R3: stacked label/value pairs cause wrapped values and doubled row height

The product has two label/value idioms and uses the tall one in the wrong places:

- Stacked (label on its own line, value beneath): Assessment Question Editor entry footer
  (`POINTS / 1`, `AVAILABILITY / Available`), Student Coursework card (`Type / Practice Question
  Assignment`, `Completion / Completed`), Student View delivery policy (`Status` then indented
  `Unreleased`, `Available` then `Not set`, `Due` then `Dec 1, 2026, 12:00 PM`), Sysadmin account
  card (`State: Active` on one line but `Last successful sign-in: ...` on its own line).
- Inline (label and value on one row): "Before you start" grid (`Questions 4 ... Points possible
  4`), Previous attempts rows, Published Question metadata strip (`Authors Neil R. Voss  Backend
  ple  Discipline Biology ...`).

The inline idiom is the HG-rows target. Wrapping happens where the stacked idiom meets an
auto-sized grid column or where a heading has a character cap:

- `instructor/published_question_detail.png`: H1 wraps to two lines ("Charged / functional
  groups") at 1280 wide because `h1 { max-width: 28ch }` (`src/style.css:292`), not because space
  ran out. The same cap wraps seven of the eight WebWorK detail titles at 1280 ("True/False
  Statements About Chi- / Square Tests", "Matching Chromosome Shapes / to Descriptions",
  "Offspring HLA Genotypes (2 / Markers, Black)", "Parent Genotypes in X-Linked / Recessive
  Crosses"); only "Genetic Disorders from Descriptions" fits. The cap also wraps the
  authorization-denial heading inside its card ("This page is not available to / this account")
  at laptop width. Each wrap costs 35 px and pushes the metadata strip and Star/Watch actions
  below the 800 px fold in six of eight WebWorK captures.
- `student/assignment_overview_unanswered_tablet.png`: the three-column Due / Time limit / Attempt
  limit grid drops `Attempt limit` to a second row at 800 px, leaving a half-empty row.
- `student/assignment_attempt_submitted_phone.png`: matching-review table header "Your match"
  and value "Triple X syndrome" wrap inside a 90 px column while the prompt column has slack.
- `student/course_in_progress_laptop.png`: "9 of 9 questions graded . Assessment score 1 / 9"
  is a sentence under a two-column grid; the score number sits at the end of prose rather than in
  a value column.
- Number values are not pinned: no `white-space: nowrap` on value cells and `tabular-nums` only on
  gradebook score cells (`src/pages/instructor_data_tables.css:112-116`). Any narrowing pushes
  the digit onto the next line first because the label takes the column's minimum width.

Violates HG-rows, HG-spreadsheet.

### R4: status messages are inserted in flow and move the controls that triggered them

Evidence of status regions placed above or between the controls, in document order:

| Capture | What was inserted | What it pushes down |
| --- | --- | --- |
| `instructor/assignment_release_released.png` | two stacked banners "Saved" and "Assessment released. Current edit number: 5." (each 50 px) between the header and the form | the whole Properties form and its Save/Release buttons by about 110 px |
| `instructor/assignment_release_draft.png` | banner "Available published Question added with its exact revision pin. Save Questions when ready." above the title field | title field, entry list, add controls |
| `sysadmin/instructor_account_validation_laptop.png` | red error band inserted between the create form and the time-zone sentence | the account list by 78 px (compare `instructor_accounts_initial_laptop.png`, list starts at y = 415 versus y = 492) |
| `instructor/assessment_template_editor.png` | banner "Template created with the canonical settings..." above the two-column workspace | template list and editor |
| `instructor/blueprint_course_detail.png` | sentence "Blueprint Course loaded. Save creates one immutable Blueprint Revision." above the eyebrow | the H1 and everything below it |
| `student/assignment_attempt_saved_laptop.png`, `student/assignment_attempt_resumed_tablet.png` | "Response saved." band appears below the action row (correct placement, buttons do not move) but "Response format is ready to save." replaces "Complete the response, then save it." above the buttons at the same height (also correct) | nothing above the buttons; this is the pattern to copy. Defect: both lines stay visible at once, so a saved Question shows two green status bands 60 px apart saying nearly the same thing |
| `student/question_webwork_laptop.png` | "Question ready." band and a full-width green button appear below a fixed-height backend frame | nothing moves, but the frame reserves about 380 px for a four-choice Question, so the status and action sit at y = 757 and y = 790 |

The Student attempt page reserves its status line above the buttons and appends success below;
nothing above the buttons changes height. The Instructor and Sysadmin pages append banners at
the top of the content region, so any action that produces a message moves every control below it,
including the one under the pointer. The Assessment Properties page shows two independent banners
that both survive, so a second action moves controls twice.

Also visible: `.instructor-list__actions` uses `flex-wrap: wrap` (`src/style.css:381-386`) and
`.course-card-actions` stacks actions vertically, so a row whose action label changes width
("Edit title and due date" versus a shorter state) can reflow neighbors.

Violates HG-jump in spirit (the rule is written for the Ribbon; the same expectation applies to
page content) and the General interface design rule that feedback should be recognizable near
the affected control.

### R5: explanatory prose is treated as page content

Every page carries one to three sentences of orientation that repeat what the Ribbon label and
page title already say, and several pages carry system-contract sentences a teacher never needs
while scanning:

- `instructor/course_assignment_workspace.png`: "Assessments appear in Course order. Open one to
  edit its Questions and Properties."
- `instructor/assignment_release_draft.png`: "Every Entry retains its exact Question Revision and
  stable identity for future Attempts." and "Exact Bloom Classification for a fixed Question could
  not load. Reload the latest Assessment."
- `instructor/question_library_browse.png`: "Counts describe all authorized Questions matching the
  current choices, not only the rows loaded below."
- `instructor/blueprint_courses.png`: three separate sentences ("Blueprint Courses contain...",
  "Choose a Blueprint Course to inspect...", "Every active Instructor can inspect...") before the
  sort control.
- `instructor/gradebook.png`: "Export Assessment points. Handle Course weighting and percentages
  in your home LMS."
- `sysadmin/instructor_accounts_initial_laptop.png`: "This workspace intentionally lists only an
  account reference, state, and last successful sign-in."
- `student/question_*_laptop.png`: "Tab to the choices and press Space to select. Shortcuts: use
  the Arrow keys or press 1-4." repeated on every Question, and "Complete responses are saved
  before submission. Incomplete responses submit as unanswered unless an earlier saved response
  exists." under every Question.

Violates HG-helper and HG-disclosure. The ADAPT comparison (images 4, 5, 21) already flagged
long introductions as friction.

## Per-surface findings

Severity: High blocks the scan task at 1280x800; Medium doubles scroll or hides comparison;
Low is polish.

| ID | Surface (capture) | Finding | Root cause | Severity |
| --- | --- | --- | --- | --- |
| D01 | My Active Courses (`instructor/course_list.png`) | Two Courses fill 800 px. Kicker "COURSE INSTANCE" repeats per row; classification, dates, theme pill, and button each take their own line or column. | R1, R2 | High |
| D02 | My Blueprint Courses (`instructor/blueprint_courses.png`) | First record at y = 515; sort control is a labeled select inside a card inside prose; each Blueprint takes four lines including a full-sentence ownership note. | R1, R2, R5 | High |
| D03 | Course Assessments (`instructor/course_assignment_workspace.png`) | Four Assessments visible; release state is inside the kicker text; reference ID and due date on separate lines; two text-only actions per row. | R2, R3 | High |
| D04 | Gradebook (`instructor/gradebook.png`) | Correct element (table) but every cell is two lines; reference IDs shown under names and titles by default; two export buttons and two sentences above the header row; no column sort affordance. | R1, R3 | Medium |
| D05 | Roster (`instructor/course_roster_active.png`) | Same two-line-cell table; single "Remove course access" text action per row is the only visible action; "Roster tools" disclosure is good. | R3 | Low |
| D06 | My Draft Questions (`instructor/question_drafts.png`) | Best Instructor list today. Still stacks "Private draft / Edit Number 2" as a two-line cell and repeats the description under every title. | R3 | Low |
| D07 | Question Library search (`instructor/question_library_filtered.png`) | Thirteen filter controls plus a separate Bloom pair occupy the whole first viewport; no result visible; "Discover Published Question Pools" and "Create Question Pool" sit above the search box on every Library page. Contradicts HG "initial Search page should stay simple". | R1, R5 | High |
| D08 | Browse Library (`instructor/question_library_browse.png`) | Four dropdowns, then three count cards each wrapping rows in individual bordered boxes (12 boxes for 12 numbers). | R2 | Medium |
| D09 | Assessment Question Editor (`instructor/assignment_release_draft.png`) | Entry row footer is five stacked label/value cells; a contract sentence and a failed-load sentence sit above the first entry; add controls are below the fold. | R3, R4, R5 | Medium |
| D10 | Assessment Properties (`instructor/assignment_release_released.png`) | Two stacked banners; "Fixed Question point values" heading plus button above the properties card; property fields in a full-width card with two-column date/time inputs. | R4 | Medium |
| D11 | Blueprint Course detail (`instructor/blueprint_course_detail.png`) | Status sentence above the eyebrow; "Return to Blueprint Courses" and "Edit Course classification" centered on their own lines; Star/Watch block with heading and explanatory sentence takes 200 px. | R1, R4, R5 | Medium |
| D12 | Published Question (`instructor/published_question_detail.png`) | H1 wraps at 28ch; "Return to question library" above the eyebrow; "Learning evidence" card holds a two-sentence apology for missing statistics. | R3, R5 | Medium |
| D13 | Search Public Blueprints (`instructor/public_blueprint_search.png`) | Result row is the closest to spec (counts and action inline). Form is a vertical stack with "Classification filters" as an orphan label; only Discipline offered though HG asks for the full hierarchy plus sort by Stars/Watches/Adoptions. | R1 | Medium |
| D14 | Templates (`instructor/assessment_template_editor.png`) | Template list is a card-per-template column at 65 px each showing only name and Type; editor nests card in card in card (workspace, header band, settings group). | R2 | Medium |
| D15 | Assessments Due Soon empty (`instructor/assignments_due_soon_empty.png`) | Empty state is a 150 px card. Acceptable, but the page also shows the same "Showing the next 7 days" sentence twice (lede and body). | R5 | Low |
| D16 | Sysadmin Instructor Accounts (`sysadmin/instructor_accounts_*`) | Two accounts fill the viewport; per-account card with a live deactivation form inside every row; validation band shifts the list. Sysadmin is low priority per HG but the list pattern is the same shared one. | R2, R4 | Medium |
| D17 | Sysadmin home (`sysadmin/system_administration_home_laptop.png`) | Four link cards with description above title (inverted hierarchy) each 100 px; a fifth area (Blueprint promotion) changes pattern to an inline form. | R2 | Low |
| D18 | Student Course home (`student/course_in_progress_laptop.png`) | One and a half Coursework items visible; each item is a 330 px card that repeats "Times shown in America/Chicago" and a three-column timing grid; score is prose. Prior finding S05. | R2, R3 | High |
| D19 | Student "Before you start" (`student/assignment_overview_*`) | Laptop layout is close to the HG spec. Tablet drops Attempt limit to a lone second row. Previous attempts rows are the right density. | R3 | Low |
| D20 | Student attempt (`student/question_*_laptop.png`) | Keyboard instruction sentence and "Finish Assessment" paragraph repeat on every Question; question card is 55 percent of the width with empty margins both sides at 1280 while the timer sentence runs to the right edge. | R5 | Medium |
| D21 | Student attempt review (`student/assignment_attempt_summary_*`) | Each Question uses five headed blocks (status line, "Recorded response" heading, value, "Feedback" heading, value); matching table columns are unbalanced on phone. Prior finding S08. | R3 | Medium |
| D22 | Student Courses / Invitations (`student/course_list_laptop.png`, `student/invitation_index_laptop.png`) | Single-item pages spend 90 px per item on a card whose only content is a title and a button; "Course invitations" and "Your courses" cross-links are styled differently on the two pages. | R2 | Low |
| D23 | Profile (`instructor/profile_default.png`) | Avatar gallery uses 175 px cards with name and a descriptive sentence each; time zone is read-only text with no control; account settings absent. Prior finding I16. | R2, R5 | Low |
| D24 | Question picker dialog (`instructor/blueprint_question_picker.png`) | Ten filter controls above results; results are three-line blocks (title, description, ID) with checkbox; only three results visible. | R2 | Medium |
| D25 | WebWorK Question detail (`instructor/webwork_*.png`, 8 captures) | "Return to question library" link above the eyebrow; H1 wraps at 28ch in 7 of 8; "Question Description" heading plus description repeats the prompt sentence for every matching Question; preview is a grey box inside a white card (double frame, 50 px of padding); metadata strip, "Generated example" caveat, and Star/Watch fall below the fold in 6 of 8. Rendered PGML content itself (colored terms, tables, dropdown matching) is fine. | R1, R3, R5 | Medium |
| D26 | Student attempt chrome (`student/question_*_laptop.png`, 9 captures) | Identical 155 px block above every Question: eyebrow, H1, timer pill, auto-submit sentence, number row, "Question n of 9 . 0 saved . Saved" line. Question card is 55 percent of the width (x 192 to 1088) while the timer sentence runs to x = 1120, so both margins are empty. Prior finding S06. | R1 | Medium |
| D27 | Student attempt chrome on phone (`student/question_*_phone.png`, 9 captures) | 330 px above the Question at 390 px wide: the breadcrumb is clipped at the left edge ("nd Peptides / ..."), the timer pill and the auto-submit sentence take two rows, and the number row collapses to 1, current, 9. The breadcrumb clip is a defect, not a density choice; prior finding S07. | R1 | High |
| D28 | Student attempt per-type instructions (`question_ma_*`, `question_match_*`, `question_fib_*`, `question_order_*`) | Two to three instruction lines before the first control on every Question: MA shows a shortcuts sentence plus "0 selected. Select from 1 through 3."; MATCH shows a three-clause sentence plus "0 of 4 prompts matched"; FIB shows "Short written response" plus "Up to 256 characters. 0 used."; ORDER shows a two-sentence shortcut note. The count lines are useful; the shortcut sentences belong in a one-time hint. | R5 | Medium |
| D29 | Student MATCH choice bank (`student/question_match_laptop.png`, `_phone.png`) | Every bank item is two lines ("Carboxyl" over "Available."), doubling bank height; on phone the bank pushes all prompt slots below the fold. State belongs in a glyph or a muted suffix on one line. | R3 | Medium |
| D30 | Student WebWorK attempt (`student/question_webwork_laptop.png`, `_phone.png`) | Backend frame reserves fixed height: a four-choice Question sits in a 380 px frame at laptop with 120 px empty below the choices, and the phone frame is 430 px tall with the same slack. Status and action sit below the fold. Prior finding I15 noted preview height variance; here it is the Student surface. | R2 | Medium |
| D31 | Student attempt saved state (`assignment_attempt_saved_laptop.png`, `assignment_attempt_resumed_tablet.png`) | Two simultaneous status bands after Save: "Response format is ready to save." above the buttons and "Response saved." below them. One status region should hold one message. | R4 | Low |
| D32 | Authorization denial (`student/authorization_denial_*`) | The heading carries a 2 px orange focus ring (programmatic focus on the H1 for screen readers) that renders as an error box around the title; H1 wraps at 28ch inside a 770 px card; "Your available account tools remain available." restates the eyebrow. | R3, R5 | Low |
| D33 | Sign-in role picker (`public/sign_in_*`) | Six role cards each carry a second line that repeats the first ("Assume the role of Instructor Dr. Elena Rivera" / "Explore Elena Rivera's seeded Instructor account"); on phone the titles wrap and each card is 90 px. Role color rails are the right cue. Demo-only surface; low priority. | R5 | Low |
| D34 | Sysadmin deactivated account (`sysadmin/instructor_account_deactivated_laptop.png`) | Action weight inverts by state: "Deactivate Instructor Account" is a quiet text link beside a pre-rendered reason field, while "Reactivate Instructor Account" is a filled primary button. The consequential action is the lighter one. The reason field should appear on demand. | R2 | Low |

## Target: one shared data-table idiom

Define one `.ple-table` in `src/pages/instructor_data_tables.css` (rename to a shared stylesheet
if it leaves the Instructor scope) and migrate every collection to it. Spreadsheet properties:

- Real `<table>` with `<thead>`; header cells carry the repeated kicker text once ("Type",
  "State", "Due"), never per row.
- Row height target 2.25-2.5rem (one text line plus padding); `--ple-dense-row-min-height` drops
  from 3.6rem to 2.4rem; `--ple-list-row-min-block-size` (4.25rem) is retired.
- One cell, one value. No stacked secondary line by default. Secondary identifiers (roster ID,
  reference ID, revision) get their own column, hidden behind a "Show IDs" toggle (ADAPT's Show
  Descriptions pattern) or shown in a tooltip on the primary cell.
- Numeric and date columns: `text-align: end` for numbers, `white-space: nowrap`,
  `font-variant-numeric: tabular-nums`, fixed `min-width` sized for the longest expected value.
- Sortable columns show a sort indicator in the header; the current sort is visible without a
  separate "Sort by" select (fixes prior I17 and D02).
- Actions: one primary action per row as a compact button or the title link itself; secondary
  actions in an overflow menu or icon buttons with accessible names (ADAPT images 1-3 pattern,
  with the ADAPT_UI_AUDIT caution about consequential actions next to routine ones).
- Row separators are 1 px lines; no per-row border radius, shadow, or card background. Hover
  highlight only.
- Empty collection: one sentence plus one action, inside the table region, not a 150 px card.

Column proposals per collection:

| Collection | Columns (left to right) |
| --- | --- |
| My Active Courses | Course, Discipline / Subject, Starts, Ends, Students, Theme (swatch), Open |
| My Blueprint Courses | Blueprint, Discipline / Subject, Revision, Adoptions, Students ever, Role (Owner / Viewer), Open |
| Public Blueprint search results | Blueprint, Author, Institution, Discipline, Stars, Watches, Adoptions, Revision, Open |
| Course Assessments | Order, Title, Type (icon + label), Release, Due, Questions, Points, Edit, Properties |
| My Draft Questions | Title, Format, Edit number, Saved, Edit, Delete |
| Library results (search, browse, picker) | Select, Title, Type, Discipline / Subject, Author, Backend, Bloom (two short codes), Revision, ID |
| Gradebook | Student, then one column per Coursework with score cell; progress state as a glyph plus tooltip; roster ID behind toggle |
| Roster | Student, Roster ID, State, Joined, Remove |
| Sysadmin Instructor Accounts | Account, Display name, State, Last sign-in, Deactivate (opens reason field inline on click, not pre-rendered) |
| Student Coursework | Type (icon + label), Title, Status, Due, Score, Action; timing details behind the existing "More timing" disclosure at the page level, not per row |
| Previous attempts | Attempt, Submitted, Score, Open (already correct) |

Student Coursework stays a list, not a spreadsheet, on phone: at 390 px each row collapses to
title, type icon, due, and action on two lines, still under 4rem.

## Fix packages

Each package is one bounded task with one verification. Order is by leverage: P1-P3 change shared
CSS and remove most of the bloat everywhere; P4-P8 are per-surface migrations.

Finding to package map: D01-D03, D13, D14, D16, D17, D22, D23 -> P2; D04-D06 -> P2 and P4;
D07, D08, D24 -> P6; D09, D10, D11, D31, D34 -> P3; D12, D25, D32 -> P1 and P5; D15, D33 -> P5;
D18, D19, D21 -> P7 and P4; D20, D26-D30 -> P8.

### P1: page header budget

- Cap the header stack at 96 px total on Instructor and Sysadmin pages: eyebrow 0.7rem uppercase
  inline before the title on the same baseline, H1 at 1.35rem, lede only when it adds a fact
  (dates, time zone, scope), never a restatement of the title.
- Remove `h1 { max-width: 28ch }` (`src/style.css:292`); titles may span the content width.
- Collapse the empty second Ribbon row: keep the reserved block-size only when the current role
  and page have a second row (Course scope tabs, Assessment scope tabs). On pages with no second
  row, reduce the band to the 3 px theme rail. If HG-jump is read as requiring constant height
  everywhere, populate the band with the breadcrumb instead of stacking both.
- Move status sentences that precede the eyebrow ("Blueprint Course loaded...", "Return to ...")
  into either the breadcrumb (return links) or the reserved status region (P3).
- Verification: rerun `./devel/capture_screenshots.sh --only=<instructor list scenarios>` and
  assert first data row y < 200 px on every list capture.

### P2: retire card rows, land the shared table

- Add `.ple-table` per the target section. Retire `.instructor-list__row`, `.course-card`, and
  the per-account card in Sysadmin accounts for collections; keep `.course-card` only if a true
  single-object summary needs it.
- Lower `--ple-dense-row-min-height` to 2.4rem, `--ple-row-padding-block` to 0.3rem, and set
  `--ple-list-row-min-block-size` to `auto` so nothing inherits the 4.25rem floor.
- Move repeated kicker text into `<thead>`.
- Verification: node unit test that every Instructor collection page renders a `<table>` with a
  `<thead>` and no `.instructor-list__row`; screenshot count of visible rows at 1280x800 (target:
  Active Courses >= 12, Assessments >= 10, Gradebook >= 12, Drafts >= 12).

### P3: reserved status region and no-shift feedback

- Every form or workspace gets one fixed-height `role="status"` region directly below its action
  row (the Student attempt page already does this). Success, info, and validation messages render
  there. Nothing above the action row changes height on submit.
- Page-level banners ("Template created...", "Assessment released...", "Available published
  Question added...") move to a single toast-style line in the reserved region, dismiss on next
  action, and never stack; a second message replaces the first.
- Validation errors for a specific field render beside that field with reserved line height
  (the `course-create-form [role="status"] { min-height: 1.4rem }` pattern at
  `src/pages/course_list_page.css:48-54` is the right shape; apply it to the Sysadmin create
  form and the Properties editor).
- Buttons whose label changes with state (Save -> Saved, Star -> Starred) get a fixed
  `min-inline-size` sized to the longest label; `.instructor-list__actions` drops `flex-wrap`.
- Verification: Playwright script in `tests/_temp/` that records the bounding box of the clicked
  button before and 500 ms after each of: Save Properties, Release, Add Question by ID, Create
  Instructor Account with invalid email, Star Blueprint. Assert delta y == 0. Promote to
  `tests/playwright/` only if the check earns permanent protection per PYTEST_STYLE.

### P4: label/value grids become inline pairs

- Replace stacked kicker grids (Assessment entry footer, Student Coursework card, Student View
  delivery policy, Sysadmin account fields) with a two-column definition grid: label column
  `max-content`, value column `1fr`, both on one row; values `white-space: nowrap` where the
  value is a number, date, or enum; `tabular-nums` on numeric values.
- On the "Before you start" and Coursework timing grid, use `grid-template-columns:
  repeat(auto-fit, minmax(14rem, 1fr))` so tablet width yields two balanced rows rather than a
  lone third item.
- Matching-review table: `table-layout: auto` with the prompt column capped at 60 percent so the
  match column keeps a readable width on phone.
- Verification: temporary Chromium check at 1280, 800, and 390 that no element with the value
  class has `clientHeight` greater than one line height.

### P5: cut prose

- Delete lede sentences that restate the page title. Keep only factual ledes (time zone, scope,
  date window) and render them as a single muted line at 0.85rem.
- Move contract sentences ("Every Entry retains...", "Counts describe all authorized...",
  "This workspace intentionally lists...") to tooltips on an info icon next to the heading or into
  the existing "Search tips" style disclosure.
- Show the keyboard instruction sentence on the Student attempt page once, in the "More timing
  and Attempt rules" disclosure or in a dismissible first-Question hint, not on every Question.
  Show the "Finish Assessment" paragraph only in the Finish region, not below every Question.
- Verification: word count of visible non-data text per list page at 1280x800 below 40 words.

### P6: Library search and picker entry

- Initial Search page: search box only, plus the "Search tips" disclosure; all filters collapse
  into one "Filters" disclosure that opens after the first result set (HG: initial Search page
  stays simple). Browse keeps the hierarchy selects but renders the count groups as three plain
  two-column lists (name, count), not twelve boxed rows.
- "Discover Published Question Pools" and "Create Question Pool" move from above the search box to
  the results header row.
- Picker dialog: same collapsed-filter rule; results use the Library table columns with the
  checkbox as the first column.
- Verification: at 1280x800 the search box, tips, and at least eight result rows are visible in
  the same viewport after a query.

### P7: Student Coursework rows

- Course home renders Coursework as the compact row described in the column table; per-row timing
  detail moves behind the existing "More timing and Attempt rules" disclosure; "Times shown in
  America/Chicago" appears once at the list header.
- Score becomes a value column ("1 / 9"), not the tail of a sentence.
- Verification: `student/course_in_progress_laptop.png` shows at least six Coursework rows;
  `student/course_not_started_phone.png` shows at least three.

### P8: Student attempt chrome and backend frame

- Collapse the per-Question header to one row: Assessment title (truncated with tooltip), timer
  pill, and the number row on the same line at laptop; the auto-submit time moves into the timer
  pill's tooltip and the "More timing" disclosure. Target: first Question line at y < 200 at
  laptop, y < 230 at phone.
- Phone breadcrumb: allow horizontal scroll with a fade, or collapse to "... / Attempt" with the
  full path in a disclosure. Never clip the leading text.
- Widen the Question card to the reading width (`--ple-reading-max-inline`, 72rem) so the
  1280 layout stops leaving 190 px margins on both sides.
- One status region per Question: the pre-save line and the post-save line share one slot; a save
  replaces "ready to save" with "saved".
- Backend frame (WebWorK): size to content via the existing resize message channel or a
  `ResizeObserver` on the rendered body, with a minimum of one choice group, not a fixed 380 px.
  The same fix applies to the Instructor preview frame (prior I15).
- MATCH bank items: one line, availability as a muted glyph or "used" strikethrough.
- Verification: `student/question_*_laptop.png` first control at y < 260; phone breadcrumb
  first character visible; WebWorK frame height within 40 px of its content height.

## Validation plan

Oracles, in the order they should run after each package:

1. Static corpus: `./devel/capture_screenshots.sh` for the affected scenarios, then recompute
   the fold budget table (chrome, header, data share, records visible) and compare first-row y,
   visible-row count, and visible-word count against the targets above. The fold budget is the
   headline metric; a package that does not move data share is not done. A small
   `tests/_temp/` Python script over the PNGs can read the row count from the manifest metadata
   or via a Playwright DOM query at capture time.
2. Layout-shift oracle (P3): Playwright bounding-box delta test described in P3.
3. Wrap oracle (P4): DOM query for any `.value` element whose height exceeds
   `1.6 * computed line-height`.
4. HG conformance walk: for each of the nine HG short names in the ledger, one screenshot per role
   that demonstrates the rule, recorded in this file's follow-up report under
   `docs/active_plans/reports/`.
5. Cognitive walkthrough rerun at 1280x800 for the four scan tasks: find a Course by term, find a
   Question by title, see which Students have not started, see what is due this week. Success
   criterion: each answer visible without scrolling and without opening a record.

## Limitations

- Screenshots are static; R4 layout shift is inferred from insertion position and DOM order, not
  measured. The P3 oracle measures it.
- Captures stop at 800 px. That is the HG-fold target viewport, so nothing below it is lost
  evidence: whatever a page pushes past 800 px is content the Instructor must scroll for, which
  is the finding. See the fold budget below.
- Only one Instructor with two Courses and nine Drafts exists in the corpus, so row-count targets
  are extrapolated from row height, not observed.
- Dark theme captures are absent; contrast checks for the new table hover and header colors
  belong to the implementing task.
- Sysadmin findings are recorded for completeness; HG marks that surface low priority.
