# PLE implemented interface review

## Outcome

The reviewed interface now reads as one teaching product rather than a sequence of bordered
components. Instructor work uses the 1280 by 800 workspace deliberately, student work stays in one
adaptable reading-and-response flow, and standard course themes retain their palette identity.
Increased contrast is an optional account presentation preference rather than the visual default.

This review implements [UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md). It changes presentation and human
navigation, not course content, grading, answer secrecy, tenant authorization, assignment behavior,
or learning semantics.

## Page-level findings and resolution

| Area                                 | Implemented resolution                                                                                                                                                                                                                                               | Evidence                                                                                                                                                     |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Global shell and hierarchy           | Uses Courses as the instructor home workspace rather than a duplicative Dashboard dropdown; keeps Library, Workspace, and Account visible; reduces page-title scale and permanent explanatory noise; removes the slogan footer; and separates global, course, and page navigation by composition and active state. | [course created](screenshots/instructor/course_authoring/01_course_created.png), [course assignments](screenshots/instructor/course_authoring/02_course_assignments.png) |
| Assignment creation and organization | Uses a full-width two-column instructor workspace, compact selected-question rows, low-emphasis move/remove utilities, direct position controls, bounded catalogs, and a visible save action. Four questions and policy controls fit at the canonical laptop target. | [assignment editor](screenshots/instructor/course_authoring/06_assignment_editor.png) and the 1280 by 800 geometry contract                                  |
| Question reuse                       | Favors whole-assignment copy and an existing-assignment checklist; direct `AAA-BBBB` Question ID entry remains a recovery and communication path. No public version or UUID is required.                                                                             | Assignment-editor keyboard and reuse tests                                                                                                                   |
| Question library                     | Allocates the useful screen width to search, facets, and results. The result viewport grows with loaded rows up to a shared cap instead of leaving a large empty bordered box.                                                                                       | [question library](screenshots/instructor/content_authoring/05_library.png) and the full-width/few-row geometry contract                                          |
| Gradebook and roster                 | Presents learner names, keeps the compact summary table primary, and loads one composed run-history panel only on request. UUIDs remain internal.                                                                                                                    | [instructor gradebook](screenshots/instructor/grading/01_instructor_gradebook.png); one-request and lazy-history tests                                               |
| Course identity                      | Preserves all 15 stored palettes, uses the full canvas anchor for the course environment, gives tinted work/group/card surfaces distinct roles, and uses the raw secondary plus accent anchors for active course navigation and identity rails. The chooser previews the applied system instead of reducing each theme to tiny swatches or an unrelated banner. | [saved course appearance](screenshots/instructor/course_management/02_appearance_saved.png) and the generated theme contact sheet                              |
| Student question                     | Keeps timer, prompt, figures, response, status, and actions in one visual sequence. Choices are compact grouped rows rather than independent heavy cards.                                                                                                            | [problem ready](screenshots/student/delivery/03_problem_ready.png), plus tablet and narrow-phone overflow checks                                             |
| Optional accessibility presentation  | Standard is the default. Increased contrast changes text, focus, and boundary tokens while retaining the course canvas and hue anchors. Forced colors remains an independent platform mode.                                                                          | [account security](screenshots/shared/account/01_account_security_passkey.png) and the account-preference browser contract                         |

The durable instructor corpus is collected in
[INSTRUCTOR_PAGE_VISUALS.md](INSTRUCTOR_PAGE_VISUALS.md). Regenerable implementation evidence stays
under `generated/ui/ui_design/` and `generated/ui/course_appearance/`, including the theme contact
sheet and measured `palette_metrics.json` report.

## Measured visual contract

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
  Student targets at 800 by 1280 and 390 by 844 have no horizontal overflow.

## Dated visual acceptance

On 2026-08-13, `generated/ui/ui_design/student_question_800x1280.png` and
`generated/ui/ui_design/student_question_390x844.png` were inspected once as generated acceptance
evidence. Both had no horizontal overflow and retained a compact prompt-response order. The phone
composition naturally continues vertically.

## Adaptability contract

The governing distance decisions are CSS custom properties in the `:root` design-system block, not
copied magic numbers across pages. Shell width and gutters, vertical rhythm, panel and row padding,
control size, catalog row/window size, assignment columns, bounded lists, table overflow thresholds,
course-canvas extent, fade distance, color-wash strength, identity-rail size, and mobile navigation
density each have a named `--ple-*` token. The catalog's `THEME_MIX` recipe similarly owns the
surface and readable-color projection percentages for all fifteen palettes. Future
observation-driven changes therefore begin with one shared control and the canonical viewport and
contrast checks described in the design guide.

## Validation and limits

The production build, strict TypeScript/lint/format checks, offline behavior tests, focused browser
tests, walkthrough-runner tests, and server tests are the permanent gates. Live external or
disposable PostgreSQL, MinIO, and WebWork cases remain explicit acceptance runs; this review does
not claim those environments unless a dated acceptance record names the run.
