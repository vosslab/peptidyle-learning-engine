# PLE interface design review: historical visual reference

## Outcome

This historical review describes the intended interface as one teaching product rather than a
sequence of bordered components. Its retained visual references use the desktop 16:10 workspace at
1280 by 800 CSS pixels for Instructor and Sysadmin work, an adaptable reading-and-response flow for
Student work, and course themes that retain their palette identity. Student profiles remain variable
across the maintained laptop, tablet, iPhone Pro aspect, and square profiles. Increased contrast is
an optional account presentation preference rather than the visual default.

It is not evidence of a current built-app teaching workflow. The current local Browser Surface is
limited to account/session entry; the screenshots named below are historical references and require a
restored browser owner plus fresh human review before they can support current visual acceptance.

This review implements [UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md). It changes presentation and human
navigation, not course content, grading, answer secrecy, course authorization, assignment behavior,
or learning semantics.

## Page-level findings and resolution

Every row below records a historical intended resolution and a historical visual or test reference;
none asserts that its teaching workflow is available in the current Browser Surface.

| Area                                | Historical design resolution                                                                                                                                                                                                                                                                                                                                                                                                                                               | Historical reference                                                                                                                                                                           |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Global shell and hierarchy          | Uses Courses as the instructor home workspace rather than a duplicative Dashboard dropdown; presents the ordered Product Ribbon: Courses, Question Library, Blueprint Courses, and Account; keeps current invitations visible; reduces page-title scale and permanent explanatory noise; removes the slogan footer; and separates global, course, and page navigation by composition and active state.                                                                     | [active roster](screenshots/instructor/course_management/01_instructor_active_roster.png), [teaching operations](screenshots/instructor/teaching_operations/01_teaching_operations_groups.png) |
| Assignment workspace                | Selecting an assignment title opens its Overview. Questions and Policies are separate pages with focused controls, compact rows, bounded Question Search Results, and a visible save action; Student view provides a stable-identity, answer-free inspection.                                                                                                                                                                                                              | T6 workspace behavior tests and the 1280 by 800 geometry contract                                                                                                                              |
| Question reuse                      | Favors whole-assignment copy and an existing-assignment checklist; direct `AAA-BBBB` Question ID entry remains a recovery and communication path. No public version or UUID is required.                                                                                                                                                                                                                                                                                   | Questions-page keyboard and reuse tests                                                                                                                                                        |
| Question Library                    | Uses All Questions, My Questions, My Question Drafts, Starred, and Watched as distinct intended interface Tasks. All Questions and My Questions query Published Questions; My Question Drafts is a future unbacked destination intended for isolated private authoring storage. It allocates the useful screen width to Question Search, facets, and results. The result viewport grows with loaded rows up to a shared cap instead of leaving a large empty bordered box. | [filtered library](screenshots/instructor/question_library_discovery/03_filtered_library_laptop.png) and the full-width/few-row geometry contract                                              |
| Gradebook and roster                | Presents student names, keeps the compact summary table primary, and loads one composed Assignment Attempt history panel only on request. UUIDs remain internal.                                                                                                                                                                                                                                                                                                           | [instructor gradebook](screenshots/instructor/grading/01_instructor_gradebook.png); one-request and lazy-history tests                                                                         |
| Course identity                     | Preserves all 15 stored palettes, uses the full canvas anchor for the course environment, gives tinted work/group/card surfaces distinct roles, and uses the raw secondary plus accent anchors for active course navigation and identity rails. The chooser previews the applied system instead of reducing each theme to tiny swatches or an unrelated banner.                                                                                                            | [saved course appearance](screenshots/instructor/course_management/02_appearance_saved.png) and the generated theme contact sheet                                                              |
| Student question                    | Keeps timer, prompt, figures, response, status, and actions in one visual sequence. Choices are compact grouped rows rather than independent heavy cards.                                                                                                                                                                                                                                                                                                                  | [Question ready](screenshots/student/delivery/03_problem_ready.png), plus tablet and narrow-phone overflow checks                                                                              |
| Optional accessibility presentation | Standard is the default. Increased contrast changes text, focus, and boundary tokens while retaining the course canvas and hue anchors. Forced colors remains an independent platform mode.                                                                                                                                                                                                                                                                                | Shared theme and forced-colors browser contracts                                                                                                                                               |

Historical instructor screenshot references are collected in
[INSTRUCTOR_PAGE_VISUALS.md](INSTRUCTOR_PAGE_VISUALS.md). Generated design artifacts remain under
`generated/ui/ui_design/` and `generated/ui/course_appearance/`, including the theme contact sheet
and measured `palette_metrics.json` report; neither replaces fresh browser acceptance evidence.

In the historical design, Student view keeps the Instructor account and course authority in place
while presenting an answer-free student landing. It is an inspection surface, not an Assignment
Attempt. Historical Student delivery creates graded work that appears in the Instructor Gradebook;
that delivery-to-Gradebook flow is unavailable in the current account/session-only Browser Surface.

## Historical measured visual contract

- Standard course body, supporting, link, and card text measures from 5.50:1 through 7.92:1 across
  all 15 themes. The design-system guard requires at least 5.5:1 and caps shared ordinary standard
  text at 8.25:1.
- Standard action, hover, and active-course-navigation text measures from 5.64:1 through 9.30:1.
  Saturated action colors are
  not lightened merely to match the ordinary-text ceiling.
- Increased contrast intentionally has no upper contrast ceiling. It retains the selected theme's
  canvas, secondary, and accent anchors.
- Focus remains attached to the focused control. Standard mode uses the modest shared indicator;
  increased contrast strengthens it, and forced-colors yields to the browser palette.
- The 1280 by 800 library controls and result surface each exceed 1,100 CSS pixels of useful width.
  Student targets at 800 by 1280 and 393 by 852 have no horizontal overflow.

## Historical adaptability contract

The governing distance decisions are CSS custom properties in the `:root` design-system block, not
copied magic numbers across pages. Shell width and gutters, vertical rhythm, panel and row padding,
control size, Question Library row/window size, assignment columns, bounded lists, table overflow
thresholds, course-canvas extent, fade distance, color-wash strength, identity-rail size, and mobile
navigation density each have a named `--ple-*` token. The course-appearance `THEME_MIX` recipe owns
surface and readable-color projection percentages for all fifteen palettes. Future
observation-driven changes therefore begin with one shared control and the canonical viewport and
contrast checks described in the design guide.

## Validation and limits

The production build, strict TypeScript/lint/format checks, offline behavior tests, focused browser
tests, walkthrough-runner tests, and server tests are the permanent gates. Live external or
disposable PostgreSQL, MinIO, and WebWork cases remain explicit acceptance runs; this historical
review does not claim those environments or a current browser workflow unless a dated acceptance
record names the run.
