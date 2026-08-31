# Course appearance accessibility audit

Status: implementation and focused acceptance complete on 2026-08-09.

This audit covers the PLE-owned instructor course-appearance workflow and the student course-entry
projection. It combines a keyboard cognitive walkthrough, source inspection, built-browser tests,
axe analysis, computed contrast, forced-colors and reduced-motion rendering. The original accepted
run included 320, 480, 768, and 1920 CSS-pixel artifacts; current visual acceptance follows the
repository's desktop-first policy and uses the canonical 1280 by 800 instructor canvas, with a
separate narrow compatibility guard. It does not claim that an institutional identity provider,
browser extension, assistive-technology combination, or third-party content is conformant.

## Task model

| Step             | Instructor goal                              | Keyboard path                            | Visible completion evidence                                                     |
| ---------------- | -------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------- |
| Open settings    | Reach the course-owned form                  | Tab through navigation, Enter            | `Course appearance` heading and current controls load                           |
| Choose a theme   | Identify and select by name                  | Tab into the native radio group, arrows  | Named radio and exact wide/narrow previews update                               |
| Add a banner     | Select one local raster image                | Tab to native file input, Space or Enter | Filename, alt controls, and both image previews appear                          |
| Describe meaning | Choose decorative or informative             | Tab and arrows; type when informative    | Selected state and, when needed, labeled text input appear                      |
| Save once        | Commit the atomic appearance                 | Tab to Save appearance, Enter            | Busy label prevents duplicates; success status is announced                     |
| Recover conflict | Preserve work and inspect current state      | Read focused alert, Tab to review, Enter | Local choices remain until explicit reload; current revision then replaces them |
| Remove a banner  | Distinguish local intent from commit         | Activate remove, keep, or save           | Pending-removal text differs from saved no-banner state                         |
| Enter a course   | Recognize the course without settings access | Navigate to course home                  | Text course title and, only when current, one authorized banner appear          |

## Interaction and state contract

| State                         | Primary action                          | Preserved state                          | Focus or announcement                             |
| ----------------------------- | --------------------------------------- | ---------------------------------------- | ------------------------------------------------- |
| Unchanged                     | Save disabled                           | Current theme/banner/alt                 | Ordinary document order                           |
| Locally edited                | Save enabled                            | Theme, selected file, alt                | Native control retains focus                      |
| Uploading or saving           | Duplicate actions disabled              | Entire local draft                       | Button label and polite live status name progress |
| Field error                   | Save remains available after correction | Theme, file, alt                         | First invalid field receives focus                |
| Network/auth/permission error | Retry after recovery                    | Theme, file, alt                         | Alert heading receives focus                      |
| Stale revision                | Review current appearance               | Theme, file, alt until review            | Conflict heading receives focus                   |
| Removal pending               | Keep current banner or save             | Current presentation remains recoverable | Explicit pending text; no ambiguous empty frame   |
| Successful save               | Edit again                              | New authoritative appearance             | Polite success status; route theme revalidates    |

The form uses native radio, file, text, and button semantics. Theme names remain visible next to
decorative swatches, so no task depends on distinguishing color. The theme radio order is stable and
arrow-key selection follows the browser's native radio-group behavior. The course title is text
outside the image. A decorative banner has `alt=""`; an informative banner uses the author-provided
description. A missing or removed banner produces no student image element.

## Findings and corrections

| Severity | Baseline finding                                                                                 | Correction                                                                                                      | Acceptance                                                      |
| -------- | ------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| High     | The settings route was a contract placeholder with no operable workflow.                         | Added the complete native-control form, atomic save, recovery, and explicit remove/cancel behavior.             | Keyboard Playwright workflow passes.                            |
| High     | Secure banner delivery existed, but the authorized student course-entry page rendered no banner. | Added one context-backed entry identity with a text title and optional current banner; no extra metadata fetch. | Entry-only browser test passes.                                 |
| Medium   | Preview images retained their HTML height at narrow widths and distorted the required ratio.     | Added explicit responsive image height and aspect containment.                                                  | Both previews measure 1200:328 at different CSS widths.         |
| Medium   | The global header and native file input caused horizontal overflow at 320 pixels.                | Wrapped the small-screen header and allowed the grid/file control to shrink.                                    | Narrow forced-colors test reports no overflowing element.       |
| Medium   | The plan required axe evidence, but no axe test dependency or executable gate existed.           | Added `@axe-core/playwright` and a main-content serious/critical gate.                                          | Zero serious or critical violations in the accepted form state. |

## Guideline ledger

| Need                                     | Standard or method                                      | Acceptance criterion                                                                       | Evidence                                            | Status |
| ---------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------ | --------------------------------------------------- | ------ |
| Complete the task without a pointer      | WCAG 2.2 SC 2.1.1 Keyboard                              | Theme, file, alt, save, conflict review, remove, and cancel all have keyboard paths        | `learner_delivery.spec.ts` appearance journey       | Pass   |
| Keep focus visible and ordered           | WCAG 2.2 SC 2.4.3 and 2.4.7                             | Native document order; selected/focused targets remain visible in normal and forced colors | Browser focus assertions and screenshots            | Pass   |
| Name controls without color dependence   | WCAG 2.2 SC 1.4.1 and 4.1.2                             | Every radio has a text name; swatches are decorative                                       | Source inspection and axe                           | Pass   |
| Describe informative images              | WCAG 2.2 SC 1.1.1                                       | Decorative images have empty alt; informative images require 1-160 useful characters       | Model tests and entry-only browser test             | Pass   |
| Preserve readable color pairs            | WCAG contrast plus PLE house target                     | Normal text at least 5.5:1; focus/boundary pairs at least 3:1                              | `palette_metrics.json` and built-browser assertions | Pass   |
| Reflow without two-dimensional scrolling | WCAG 2.2 SC 1.4.10                                      | No document overflow at 320/480 CSS pixels and 200 percent equivalent layout width         | Responsive browser test and screenshots             | Pass   |
| Expose status and recovery               | Nielsen visibility/control; WCAG status/error semantics | Busy, success, field error, general error, and conflict are visible and announced          | State walkthrough and browser assertions            | Pass   |

WCAG references: [Keyboard](https://www.w3.org/WAI/WCAG22/Understanding/keyboard),
[Focus Order](https://www.w3.org/WAI/WCAG22/Understanding/focus-order.html),
[Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html), and
[Non-text Content](https://www.w3.org/WAI/WCAG22/Understanding/non-text-content.html).

## Heuristic result

Scores use 0 for a critical failure and 4 for no material issue in the audited scope.

| Nielsen heuristic               | Baseline | Accepted | Evidence                                                                 |
| ------------------------------- | -------: | -------: | ------------------------------------------------------------------------ |
| Visibility of system status     |        1 |        4 | Busy labels, live status, focused errors, and conflict state             |
| Match with the real world       |        2 |        4 | Theme names, entry banner vocabulary, and exact previews                 |
| User control and freedom        |        1 |        4 | Cancel selection, keep current, remove on save, explicit conflict review |
| Consistency and standards       |        2 |        4 | Native radios/file input/buttons and one save action                     |
| Error prevention                |        1 |        4 | Local raster/size/alt checks, disabled duplicate save, revision CAS      |
| Recognition over recall         |        2 |        4 | All 15 named options and visible current selection                       |
| Flexibility and efficiency      |        1 |        4 | Radio arrows plus ordinary Tab/Enter path                                |
| Aesthetic and minimalist design |        2 |        4 | Three bounded sections, entry-only student banner                        |
| Error recognition and recovery  |        1 |        4 | Preserved draft, actionable alert, explicit reload                       |
| Help and documentation          |        1 |        4 | File policy, alt guidance, save semantics, and this owner record         |

## Validation and artifacts

```bash
node --import tsx --test \
  tests/test_course_appearance_settings.mjs \
  tests/test_course_theme_scope.mjs
```

The prior production-browser scenario and screenshot-corpus publication command
are absent from the current tree. The historical `appearance_saved` state does
not establish current visual acceptance. A Course Appearance UI change needs a
restored browser owner and fresh human visual review in addition to these
durable behavior gates.

The current generated review set is under `generated/ui/course_appearance/`:
`theme_contact_sheet.png`, `settings_1280x800.png`, `settings_forced_colors.png`, and
`palette_metrics.json`. Generated evidence is intentionally
gitignored; the workstream handoff records exact hashes from the accepted run.

## Human-use boundary

No student or instructor participant and no screen-reader user was recruited for this engineering
pass. The fall-pilot readiness owner should include representative Roosevelt instructors and
students in ordinary usability and VoiceOver/NVDA sessions. That evaluation improves confidence in
the product but is not a missing implementation dependency: the PLE-owned semantics, keyboard path,
reflow, image alternatives, status handling, contrast, and automated accessibility gate are present
and executable now.
