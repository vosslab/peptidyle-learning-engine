# Plan: Fast UI development lane

## Context

This side plan makes the fast UI lane execution-ready. The primary
[review-the-content-of-merry-hickey.md](../review-the-content-of-merry-hickey.md) owns the seven
production-list conversions, Live Demo integration, and screenshot publication.

Current-checkout evidence shows a focused isolated Chromium harness surface for the Ribbon,
`PageFrame`, `RecordList`, and representative consumers. Use synthetic slot content for primitive
API checks and actual production page markup for route-composition parity cases. If a fast-lane
failure exposes a harness-design issue, isolate that cause and use the evidence to choose a focused
correction.

Architect review approved this cross-cutting approach with the corrections recorded below.

## Objectives

- Provide one obvious command for isolated, headless Chromium UI checks in a stack-free environment.
- Use synthetic slot content for primitive `RecordList` API checks, actual production consumer
  components for consumer cases, and actual production page markup for route-composition cases.
- Add one small CLI case map over existing fixtures; retain headed Chromium as an optional debugging
  mode for a selected case.
- Keep the full-stack parity sample to the three approved same-data route-composition scenarios.

## Design philosophy

Follow **Fix the design, not the symptom** and **Ground requirements in actual needs** from
[docs/REPO_STYLE.md](../../REPO_STYLE.md). The fast lane earns its cost by rendering real production
components and styles cheaply, while narrowly substituting unavailable services and data. Shared
production ownership is the main drift prevention: the harness imports production page composition
and styles, and the compact parity sample checks only the relationships that cross the application
boundary.

The essential boundary is categorical. A primitive `RecordList` harness uses synthetic slot content
to prove the primitive API. A route-composition case renders actual production page markup and
injects data at the route's narrow production provider or repository boundary. Parity pairs those
route compositions with the corresponding PLE routes, keeping the same UI source on both sides.

## Scope

- Reuse and extend the current production-source harnesses; add one small named CLI map over their entrypoints.
- Run the fast checks headlessly. Keep `--headed --case <name>` available for optional interactive
  debugging by a developer or agent.
- Import the shared production `src/browser_environment.ts` entry where the fast fixture represents
  the PLE browser environment; keep the test host focused on fixture mounting and test-only scrolling.
- Add direct primitive `RecordList` cases for primitive API evidence and route-composition cases
  with actual production page markup for the parity subset.
- Run the restricted parity sample in the existing full-stack integration lane and classify drift
  before assigning a correction.
- Document the fast and headed commands and their evidence boundary in `docs/DEVELOPMENT.md`.

## Scope boundaries

- Center the case set on the shared UI backbone and the primary plan's seven representative
  consumers.
- Run fast checks in localhost Chromium; run the parity sample in the existing full-stack integration
  lane.
- Use named structural relationships as parity evidence and reserve screenshot publication for the
  primary plan.
- Keep remaining migrations and final integration with the primary plan.

## Current state summary

| Evidence area               | Current evidence                                                                                                                                                                                     | Corrected use in this plan                                                                                                                                                                       |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Shell and `PageFrame`       | `ribbon_shell_loader.ts` compiles real `App`, `ApplicationShell`, and `PageFrame` with narrow fixture API/session data.                                                                              | Extend this route-composition infrastructure with actual reading and `fullWidth` production pages for parity.                                                                                    |
| Primitive `RecordList`      | `record_list_contracts.mjs` uses one shared `record_list_harness.tsx` fixture for alignment, states, presentation, reorder, and windowing against the production primitive.                         | Keep the shared fixture for these distinct durable contracts; use synthetic records and slot content only at the primitive boundary.                                                            |
| Production Student route    | `student_course_entry_m6_harness.tsx` already imports `StudentCourseLandingPage` and injects API data; `student_course_entry_m6_evidence.mjs` checks its Coursework rows across canonical viewports. | Reuse this actual route composition for WP-E6 and the Student Coursework case; bring its shared style loading into the fast lane. `student_courses_page.tsx` is a separate card grid.            |
| Production consumer         | `provided_avatar_picker_harness.tsx` imports the real picker; its browser check covers Gallery/List presentation, identity, and selection across canonical viewports.                                | Include the existing consumer check in the fast lane and case map; use it as evidence that presentation choices generalize without avatar-specific RecordList behavior.                          |
| Production Question Library | WP-E3 row composition is `src/pages/library_browse_rows.tsx`; actual page composition is `LibraryPage`, whose `QuestionLibraryBrowseRepository` is injected at the route boundary.                   | Render `LibraryPage` through its production repository boundary for parity.                                                                                                                      |
| Frontend environment        | `src/index.html` used separate global stylesheet links and `src/main.tsx` imported app-shell styles; harnesses separately assembled the same sources.                                                | `src/browser_environment.ts` now owns the ordered production-wide CSS imports. Production `main.tsx` and full-environment harnesses consume that entry; component styles remain component-owned. |

The concrete PageFrame routes are the reading `StudentCourseLandingPage` route (also the WP-E6
Student Coursework case) and the `fullWidth` Instructor `GradebookPage` route named by WP-E2.

## Resolved decisions

- Architect review: **approved with required corrections**. This plan records those corrections as
  the implementation scope.
- Case selection and optional interactive debugging: one small CLI map routes names to existing
  fixture entrypoints. `--headed --case <name>` opens the selected registered case directly in
  Chromium; headed use is not a completion requirement.
- Style provenance: use `src/browser_environment.ts` as the ordered production-wide CSS entry in both
  the live app and full-environment harnesses. Its global and app-shell imports use the same
  production source; route/component CSS remains imported by its owner. Keep test-host CSS to root
  mounting and test-only scrolling.
- Parity data: compare ordered IDs when the fixture and live route consume the same named seed;
  otherwise compare presentation and row/region structure.

## Approach

1. **Inventory and label cases.** Add one small CLI case map over existing fixtures, labeling each
   entry `primitive`, `production-consumer`, or `route-composition`. Record the rendered production
   component/page, injected service boundary, and parity eligibility. Route-composition cases render
   production page markup.
2. **Restore production style ownership.** Add the small `src/browser_environment.ts` entry to import
   ordered global and current app-shell styles. Import it from production `main.tsx` and each
   full-environment harness; keep route/component CSS with its owner. Retain only the standalone
   embed stylesheet as a separately copied artifact. This shared-source change is the primary
   safeguard against UI drift.
3. **Complete primitive API evidence.** Add direct `RecordList` populated, empty, loading, and error
   cases. Use synthetic records and slot content to exercise the primitive API, including
   presentation, reorder, and compositional windowing. Use the same deterministic records in
   windowed and unwindowed `RecordList` cases; check that row identity and region structure are
   unchanged while the rendered record subset follows the window.
4. **Make the fast entry point focused.** Add one launcher for the registered cases, running them
   headlessly by default. Its optional `--headed --case <name>` form opens the selected case directly
   in Chromium while using the same production-source bundle and local fixture server.
5. **Build actual route-composition cases.** Reuse the existing WP-E6
   `StudentCourseLandingPage` Coursework route. Add the reading `StudentCourseLandingPage` and
   `fullWidth` WP-E2 `GradebookPage` compositions where the current route fixture does not already
   expose them, plus actual `LibraryPage` composition through the narrow production
   provider/repository seam. Use the same named seed before comparing Question IDs; otherwise
   compare presentation and row/region structure.
6. **Run the compact parity sample last.** Add the three approved route-composition scenarios to
   the existing full-stack integration lane. Assert each scenario's named structural relationships
   and classify any drift as fixture, production, or parity-contract drift before assigning a
   correction. Assign cross-cutting decisions to a fresh architect subagent; keep ordinary fixture
   or local production corrections with their named owner.

## Critical files

- `tests/playwright/ribbon_shell_contract.mjs`, `tests/support/ribbon_shell_loader.ts`, and
  `tests/support/ribbon_shell_harness.tsx`: existing production-shell composition infrastructure.
- `tests/playwright/student_course_entry_m6_evidence.mjs` and
  `tests/support/student_course_entry_m6_harness.tsx`: existing actual Student Coursework route
  composition and viewport evidence.
- `tests/playwright/provided_avatar_picker_presentation.mjs` and
  `tests/support/provided_avatar_picker_harness.tsx`: existing real consumer proof for Gallery/List
  presentation and shared selection.
- `tests/playwright/record_list_contracts.mjs`, `tests/support/record_list_harness.tsx`, and
  `tests/playwright/helper_record_list_harness.mjs`: one compiled primitive API fixture for the
  durable RecordList contracts and CSS-host boundary.
- `src/styles/browser_fonts.css`, `src/styles/product_role.css`, and `src/assets/fonts/`: production
  font declarations/assets and product-role styles, imported through `src/browser_environment.ts`.
- `src/browser_environment.ts`, `src/main.tsx`, `src/index.html`, and `pipeline/build.mjs`: the shared
  browser environment source and emitted `main.css` ownership.
- `src/pages/student_course_landing_page.tsx`: WP-E6 Student Coursework list composition.
- `src/pages/library_page.tsx`, `src/pages/library_browse_rows.tsx`, and
  `src/pages/library_page_model.ts`: actual Question Library page, rows, and injected repository
  contract.
- `launchers/` and `docs/DEVELOPMENT.md`: the single command front door and developer instructions.
- `docs/active_plans/review-the-content-of-merry-hickey.md`: authority for the underlying production
  conversions and their seven representative consumers. Its earlier Student navigation proposal is
  superseded by [Human Guidance](../../HUMAN_GUIDANCE.md) and the settled Course-context decision.

## Work packages

### WP-1: Classify registered fast cases

- Owner: UI harness implementer.
- Depends on: none.
- Outcome: a small CLI case map identifies primitive, production-consumer, and route-composition
  cases and their production source, injection boundary, and parity eligibility.
- Acceptance criteria: label synthetic slot-content cases `primitive`, actual consumer components
  `production-consumer`, and production page compositions `route-composition`; record their source,
  data seam, viewport coverage, and parity eligibility. The case map supplies names accepted by
  `--headed --case <name>`.
- Obvious follow-on: WP-3, then WP-2 and WP-4.

### WP-2: Establish focused headless and headed execution

- Owner: UI harness implementer.
- Depends on: WP-1 and WP-3, so registered cases already use production styles.
- Outcome: the fast command runs approved registered cases headlessly. `--headed --case <name>` is
  available to open one selected existing case for optional interactive debugging.
  `docs/DEVELOPMENT.md` documents the command and the fast/parity/Live Demo boundary.
- Acceptance criteria: the required headless command preserves non-zero failures and runs in a
  stack-free environment; the developer guide shows the command and evidence boundary. Keep the
  headed option available for optional debugging; close the work package using the headless evidence.
- Obvious follow-on: WP-4.

### WP-3: Share the production browser environment and primitive coverage

- Owner: UI harness implementer.
- Depends on: WP-1.
- Outcome: direct primitive `RecordList` cases cover populated, empty, loading, and error states
  under production styles.
- Acceptance criteria: production `main.tsx` and full-environment harnesses import the same
  `src/browser_environment.ts`; `main.css` contains its ordered global/app-shell styles before
  route/component-owned styles. Keep test-host CSS to root mounting and test-only scrolling. Use
  synthetic slot content for primitive API evidence. Exercise one deterministic record fixture both
  windowed and unwindowed, checking row identity and region structure in the fast layer.
- Obvious follow-on: WP-2 and WP-4.

### WP-4: Compose the approved actual-page cases

- Owner: UI harness implementer.
- Depends on: WP-1 and WP-3.
- Outcome: route-composition cases render actual production markup for the reading
  `StudentCourseLandingPage` route, the `fullWidth` `GradebookPage` route, Student Coursework, and
  Question Library. Reuse the existing `student_course_entry_m6_harness.tsx` Student composition;
  it shares the WP-E6 route with the reading PageFrame case.
- Acceptance criteria: the Student case renders WP-E6 `student_course_landing_page.tsx` Coursework
  markup. The Question Library case renders `LibraryPage` and injects
  `QuestionLibraryBrowseRepository` through its production route/page seam. Compare Question IDs
  and order when both routes consume the same named seed; otherwise check presentation and
  row/region structure.
- Obvious follow-on: WP-5.

### WP-5: Add the restricted full-stack parity matrix

- Owner: integration implementer.
- Depends on: WP-2 and WP-4; registered route-composition cases supply the parity inputs.
- Outcome: the existing full-stack integration lane runs the following three named structural
  scenarios as a compact backstop. The PageFrame scenario contains a reading/fullWidth route pair;
  the other scenarios cover the Student list and Question Library structure using production's
  normal windowing behavior.

  | Canonical route case                                                           | Required viewport entries | Contract measured                                                                                                                                                      |
  | ------------------------------------------------------------------------------ | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | WP-E6 reading `StudentCourseLandingPage` and WP-E2 `fullWidth` `GradebookPage` | Laptop                    | `contentLayout` mode; title, main, and breadcrumb rail/origin relationships.                                                                                           |
  | WP-E6 Student Coursework route                                                 | Laptop and phone          | Required-region visibility and main-action adjacency on the real route; optional-region priority removal is covered by the existing fast `RecordList` primitive check. |
  | Seeded `LibraryPage` Question Library results route                            | Laptop                    | Presentation mode; row/region structure; ordered IDs when the fixture and live route consume the same named seed.                                                      |

- Acceptance criteria: compare each scenario's named structural relationship: the PageFrame rail
  and intentionally aligned origins, Student required-region visibility and action adjacency, and
  Question Library presentation and row/region structure under normal production windowing, plus
  same-seed order where available. Use browser geometry to establish the named rail/origin
  relationships, and assert responsive visibility and action adjacency directly. Keep this parity
  sample smaller than the fast case set. Classify any mismatch as fixture, production, or
  parity-contract drift before assigning a correction.
- Obvious follow-on: WP-6.

### WP-6: Record bounded evidence and close the side plan

- Owner: documentation maintainer.
- Depends on: WP-2 through WP-5.
- Outcome: record the command, case classifications, parity scenarios, named-seed evidence, results,
  and drift classifications while the primary plan remains unchanged and authoritative.
- Acceptance criteria: record fast primitive evidence, route-composition evidence, the compact
  full-stack parity result, and Live Demo evidence as distinct layers. A parity correction identifies
  its owner and classification. Update the changelog under repository policy and archive after all
  packages pass.

## Verification

| Layer                          | Command or setting                                           | Evidence it establishes                                                                                                                             | Failure handling                                                                      |
| ------------------------------ | ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Fast primitive                 | The focused fast command, headless                           | Production `RecordList` states, presentation, reorder, and identity/region behavior on the same record fixture with windowing enabled and disabled. | Fix the primitive or its direct evidence.                                             |
| Fast production consumer       | The focused fast command, headless                           | Real avatar-picker component supports Gallery/List selection across the existing canonical viewport checks.                                         | Keep the consumer using the general production presentation interface.                |
| Optional interactive debugging | `--headed --case <name>`                                     | Opens one named current-source case in Chromium for a developer or agent when useful.                                                               | Headless behavior checks remain the required evidence; headed use is optional.        |
| Route composition              | The focused fast command, headless                           | Actual production page markup with narrow data/repository injection.                                                                                | Keep fixtures and route providers aligned with production ownership.                  |
| Restricted parity              | Existing full-stack integration lane                         | Three named structural scenarios paired with corresponding PLE routes.                                                                              | Classify fixture, production, or parity-contract drift before assigning a correction. |
| Live Demo                      | Primary-plan integration and screenshot-publication commands | End-to-end services, authorization, workflow semantics, and published screenshots.                                                                  | Keep final integration and publication with the primary plan.                         |

Record that the fast command ran in a stack-free environment; primitive and route-composition
evidence remained distinct; and the compact parity sample used its named relationships. Report
Question Library ID/order when both sides use one named seed. Record the headed option in the
developer guide as a convenience, not as close-out evidence. The primary plan's integration gate
remains the authority for full-stack acceptance.

## Execution record - 2026-09-23

- `./launchers/run_fast_ui_checks.sh` passed headlessly in the stack-free lane: RecordList alignment
  at laptop/tablet/phone/square sizes; primitive states, presentation, reorder, and windowing; the
  real Avatar Picker at four viewports; production Gradebook and Library route composition; and
  signed-in Ribbon shell geometry. No headed browser session was used or needed for close-out.
- `source ./source_me.sh && python3 tests/e2e/e2e_live_demo_production_browser.py --scenario
ui_backbone_parity` passed on the real PLE stack (one Playwright test). It confirmed centered
  reading PageFrame rails, full-width Gradebook rail alignment, required Coursework identity/action
  regions on laptop and phone, and production Library Scan presentation and region structure under
  normal windowing. The Library route used `?tag=protein`; its production search request carried
  that filter, and the visible ordered IDs matched the `fast-ui-question-library-v1` seed subset.
  No fixture, production, or parity-contract drift was observed, so no parity correction was needed.
- The primary plan's 2026-09-23 screenshot replay and warm-loop receipts remain the separate Live
  Demo evidence; this parity run did not expand the screenshot suite.
- The clean `source ./source_me.sh && ./launchers/all_test.sh` rerun exited 0: 9,267 Python tests,
  offline aggregate checks, and all three live-service acceptance cases passed. The initial attempt
  found this manager's parity suite still running; it was stopped, acceptance passed, and the exact
  aggregate command then passed from a clear owner state.
- Evidence for WP-1 through WP-5 is recorded above. WP-6's evidence record is complete and changelog
  rotation is complete. Plan archival is the only remaining close-out step: repository policy
  requires a Git-managed move, and `.git` is not writable in this workspace. No interactive browser
  session is a dependency.
