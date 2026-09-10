# Course Appearance accessibility audit

Status: component-harness evidence recorded on 2026-09-09; production-route accessibility remains
open in the [Course Appearance audit](../active_plans/audits/course_appearance_six_pass_review.md).

This record covers the restored Instructor Course Appearance page: independent Theme and Banner
forms rendered from the current authorized Course Appearance view. It is page-scoped evidence, not
a claim that the whole application or every assistive-technology/browser combination is conformant.

## Page-owned task evidence

The explicitly invoked focused browser check builds the Solid page and support harness with esbuild,
serves the bundle over loopback HTTP, and drives Chromium with visible native controls. It uses
`@axe-core/playwright` against the page surface. This retained browser behavior check is outside
fast pytest and the aggregate service gate. Its original reconstruction probes were one-time
evidence and have been removed under [PYTEST_STYLE.md](../PYTEST_STYLE.md).

| Instructor task                                | Accessible behavior proved                                                                                                                                  | Evidence            |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| Select a theme without color or a pointer      | Arrow-key navigation changes the checked radio and preview; the selection has an accessible name and visible text                                           | Focused check below |
| Choose a banner without a pointer-only control | Keyboard focus plus Enter opens the named native chooser; Playwright supplies its selected local file, and the local preview and enabled save action appear | Same test           |
| Receive save outcomes                          | A failed theme write produces an assertive `alert`; a successful write produces a polite `status`                                                           | Same test           |
| Phone reflow                                   | The page has no horizontal overflow and its Banner save control is visible at the phone profile                                                             | Same test           |
| Avoid serious defects detectable by axe        | No serious or critical axe violations on the rendered page                                                                                                  | Same test           |

## Corrections found by the gate

The initial rebuild also checked fixed radio order and all four canonical viewport profiles.
Those results remain implementation-time evidence. Keyboard selection and named non-color state
remain durable behavior checks without assuming which theme follows another. Exact palette ordering, control counts, and
full geometry snapshots do not remain in the suite. Retained checks cover user-visible behavior;
[TEST_EVIDENCE_MODEL.md](../TEST_EVIDENCE_MODEL.md) records the lifetime classification.

- Banner preview images transferred their 6:1 preferred width into the grid's minimum sizing, which
  widened the document to 720 px at the phone profile. The form, preview group, and image now have
  explicit zero minimum inline sizing, so the responsive width is authoritative.
- Several raw decorative accent anchors did not support their palette-role label at normal text
  contrast. The Accent role now uses a recognizable low-strength accent tint with the derived ink
  color; the named checked native radio remains the actual selection state.

## Deliberately reused application evidence

Forced-colors and reduced-motion are application-wide invariants already exercised by the shared
accessibility stylesheet, the production-build browser gate, and Ribbon responsive evidence. M9
does not repeat them for this page. The focused test instead keeps durable coverage for the
controls, content, and reflow introduced by Course Appearance.

## Run

```bash
node --import tsx tests/playwright/course_appearance_m9_accessibility_evidence.mjs
```

The check must run where headless Chromium can create its macOS browser process. Its result is
automated component accessibility evidence. It does not close production-route accessibility,
and any later attended usability review remains optional.
