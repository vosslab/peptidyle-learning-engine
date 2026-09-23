# Merry Hickey plan implementation checklist

This is a companion execution checklist for
[review-the-content-of-merry-hickey.md](review-the-content-of-merry-hickey.md). The active plan is
authoritative; this file neither amends its scope nor substitutes for its decisions.

## Reading and status rules

- `[x]` means current source, a current permanent test, or a cited recorded receipt supports the
  stated outcome. It does not imply an unrecorded broader test run.
- `[ ]` means the code outcome, verification, or residual evidence still needs to be produced or
  rechecked. A package is not complete until its applicable unchecked items are resolved.
- Preserve concurrent work. Inspect `git status --short` before editing a touch point, and do not
  overwrite or reformat another worker's changes.
- Manager checkpoint: while work is active, reread the authoritative plan at each work-package
  boundary and about every 30 minutes of active execution. The manager performs this check; it does
  not depend on a human reminder or response.
- Build and focused behavior checks precede screenshot publication. Screenshot artifacts are
  evidence of a completed behavior, not a substitute for building or testing it.

## Evidence lanes

- Focused test: the named Node, Playwright, or temporary probe for one work package.
- Fast compliance: the recorded `source ./source_me.sh && ./launchers/run_fast_checks.sh`
  passed 9,287 Python tests plus Rust, TypeScript, Node, lint, and format after the final UI source
  edits. That receipt predates the later test-surface cleanup; in that review snapshot the focused
  UI lane passed, but the aggregate fast-check attempt exited 1 when sandbox permissions blocked
  Chromium startup for `course_instance_assessment_refresh.mjs`. The isolated test passed with
  browser launch permission; no aggregate pass is claimed for the later test tree. See the
  2026-09-23 changelog entry.
- Historical full-suite receipts: on 2026-09-23, six separate invocations for M1-M6 exited 0 on the
  current integrated tree, passing Rust, TypeScript, Node, lint, format, all 9,267 Python tests,
  and all three live-service acceptance cases. The earlier clean M7/final aggregate also exited 0.
  These are dated receipts, not reconstructions of intermediate snapshots. The final source snapshot
  has no refreshed full-suite receipt; that status is explicit and does not block this plan.
- UI acceptance uses the fast compliance command plus focused behavior/browser checks for changed
  scope, visual review of relevant saved artifacts, and the recorded clean integration replay. Do
  not repeat `all_test.sh` solely to close a UI milestone.
- Live browser: a real browser check or Live Demo workflow, distinct from static source review.
- Screenshot receipt: the capture manifest, atlas/publication artifacts, and the clean replay
  receipt. It is not established by a focused test or full suite alone.

## Authority and scope notes

- The plan's viewport policy is controlling for this work: Student has direct laptop, tablet,
  phone, and square views; Instructor and Sysadmin are laptop-only; Public capture scope remains as
  defined by the current screenshot corpus. A `covered_by` entry is only a representative
  substitution and leaves its omitted viewport unverified.
- One clean semantic Live Demo replay must record workflow, manifest closure, privacy, dimensions,
  and published-artifact integrity. Observed PNG byte/hash differences are reported with their
  apparent cause; they are not a hard failure by themselves.
- No concrete conflict with Human Guidance was found. Student's four-view requirement agrees with
  [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#student-interface) (lines 620-632); the Instructor
  laptop usability statement at lines 407-427 does not require extra published viewports. Existing
  extra Instructor screenshots in [SCREENSHOT_ATLAS.md](../SCREENSHOT_ATLAS.md) are historical
  evidence, not a conflicting capture requirement. For a later cross-cutting design question, assign
  a fresh architect subagent to make and record the evidence-based recommendation; the manager
  applies it within the active plan and updates this checklist.

## M1 shell contract

- [x] **WP-A1 - add `tierOneArea`.** `src/route_contract.ts` defines a required
      `tierOneArea` and current route records carry it.
  - [x] Focused TypeScript and route-contract evidence passed with the 29 Ribbon Node tests.
  - Residual evidence: route inventory shows every applicable route selects a role-valid tab.

- [x] **WP-A2 - key tier one to Product Role.** `src/ribbon/ribbon_schema.ts` exports
      `PRODUCT_TIER_ONE` and `ribbonSchemaFor(productRole)`; Student includes Courses, Coursework,
      and Grades.
  - [x] Focused schema and Ribbon-contract checks passed in the 29 Ribbon Node tests.
  - Residual evidence: tab IDs are invariant on every route reachable by each role.

- [x] **WP-A3 - move scope tabs to tier two.** Current Ribbon catalog and route records expose
      Coursework and Grades as tier-one destinations and task groups for scoped work.
  - [x] The focused catalog/route checks passed: no `TAB_CATALOG` item requires route parameters,
        and each named scope tab is a task control.

- [x] **WP-A4 - pin Student tabs to a Course.** `route_scope_controller.ts`, the Grades route,
      and `student_course_pinning.mjs` implement current-course links and the `/student` fallback.
  - [x] The focused Student-pin browser check passed from an active Assessment Attempt.
  - Residual evidence: browser output records both links with the Attempt's Course Instance ID.

- [x] **WP-A5 - reserve shell height.** `application_shell.tsx` always renders
      `BreadcrumbPrelude`; current shell/Ribbon sources carry the unconditional reserved-row design.
  - [x] The shell-geometry check passed for all signed-in roles and planned screen sizes.
  - Residual evidence: browser offsets, not source inspection, prove no route changes height.
  - [x] Fresh `image_evaluator` review covered six production-shell fixture PNGs (Student laptop,
        tablet, phone, and square; Instructor and Sysadmin laptop) and four corrected Student
        `assessmentOverview` captures at the same four canonical sizes. Role-specific Ribbon
        navigation and the reserved task row pass visually. Student shows
        `Home / Biochemistry / Problem Set 7`; Instructor shows
        `Home / Biochemistry / Problem Set 7 / Questions`. Student phone is compact and legible,
        with no clipping or obscured controls; no CSS change was justified.
  - The initial structural `assessmentAttempt` fixture showed only `Home / Attempt`: its path
    declares an Attempt ID, and the fixture supplied no resolved relationship scope. This image
    therefore omits route ancestors. `ribbonParamsFor` and `ribbonLabelsFor` in `src/app.tsx`,
    `src/ribbon/route_scope_controller.ts`, and `node --import tsx --test
tests/test_ribbon_contract.mjs` (19/19) establish that resolved scope supplies Course and
    Assessment IDs and the display title, matching the breadcrumb hierarchy contract. No
    production change is warranted.
  - Evidence: `test-results/m1-wpa5-fixture-shell-20260922/` and
    `test-results/m1-wpa5-student-overview-20260922/`. The overview report has no browser, page, or
    console errors. These synthetic images are not Live Demo proof.

- [x] **M1 close-out.** The separate 2026-09-23 M1 `all_test.sh` invocation exited 0 on the
      current integrated tree with all aggregate gates and live-service acceptance passing. This
      receipt does not reconstruct an M1 intermediate snapshot. Screenshot publication is not M1
      completion evidence.

## M2 shell checks

- [x] **WP-B1 - tab invariance check.**
      `tests/playwright/ribbon_shell_contract.mjs` targets stable control IDs rather than text.
  - [x] The role/route tier-one invariance browser check passed.
  - Residual evidence: intentional failure/revert demonstration, if not already recorded.

- [x] **WP-B2 - shell height check.** `tests/playwright/ribbon_shell_contract.mjs` owns the shell geometry check;
      the active plan records all three roles in scope.
  - [x] The focused shell-height browser check passed at the plan's four profiles.
  - Residual evidence: intentional failure/revert demonstration, if not already recorded.

- [x] **M2 close-out.** The separate 2026-09-23 M2 `all_test.sh` invocation exited 0 on the
      current integrated tree with all aggregate gates and live-service acceptance passing. This
      receipt does not reconstruct an M2 intermediate snapshot; live-browser evidence remains
      separate from the full-suite receipt.

## M3 shared components

- [x] **WP-C1 - measure page headings.** The 44-shape `PageFrame` baseline is recorded in
      `DESIGN_DECISIONS.md`; the decision admits slots by layout purpose, not frequency alone,
      and the temporary probe is gone.

- [x] **WP-C2 - build `PageFrame`.** `src/components/page_frame.tsx` and its stylesheet exist;
      the component reads route content layout and owns heading composition and width.
  - [x] Focused TypeScript checks passed.
  - Residual evidence: a sweep receipt confirms pages use the component and no subject-specific
    API leaked into it.
  - [x] Confirm the fixed frame root and content placement, with only WP-C1-admitted slots; route
        width stays outside Ribbon ownership and page-specific styles stay within the content region.
  - [x] The signed-in shell's route `ErrorBoundary` fallback uses PageFrame's title, lede, and
        action slots rather than maintaining a separate page heading/layout.
  - [x] Visually inspect reading and full-width compositions at applicable viewports. The Student
        four-size fixture renders production pages and PageFrame but omits the shell; Instructor
        current-production captures cover reading and full-width layouts with the shell.

- [x] **WP-C3 - build `RecordList` core.** The `src/components/record_list/` core, region
      specification, and stylesheet exist, with permanent alignment evidence present.
  - [x] The four-size RecordList alignment browser check passed.
  - [x] Optional visible region headings use the same RecordList tracks and responsive priority
        visibility as row regions; accessible names remain in the row content.
  - Residual evidence: review confirms core excludes variants, reorder, and windowing.

- [x] **WP-C4 - build date formatting.** `src/format_datetime.ts` supplies explicit display-zone
      formatting and is imported by current page/model code.
  - [x] Focused date Node tests passed 7/7.
  - Residual evidence: sweep records time zone once per page rather than per row.

- [x] **M3 close-out.** The separate 2026-09-23 M3 `all_test.sh` invocation exited 0 on the
      current integrated tree with all aggregate gates and live-service acceptance passing. This
      receipt does not reconstruct an M3 intermediate snapshot; source presence alone is not an
      integration receipt.

## M4 list options

- [x] **WP-D1 - presentation variants.** `record_list_presentation.ts` and the permanent
      `record_list_contracts.mjs` browser check exist; current proof consumers include list/gallery
      and scan/preview shapes.
  - [x] The focused RecordList presentation browser check passed.
  - Residual evidence: review confirms one-variant lists have no switcher or global preference.

- [x] **WP-D2 - reorder.** `record_list_reorder.ts` and its browser contract are present.
  - [x] The six-site, four-axis semantic comparison is recorded in `DESIGN_DECISIONS.md`.
  - [x] The focused RecordList reorder browser check passed, including keyboard, focus-return,
        live-region, and drag evidence.

- [x] **WP-D3 - windowing.** `record_list_window.ts` and its dedicated browser contract are
      present in `record_list_contracts.mjs`; the library row implementation is a current consumer.
  - [x] The focused RecordList windowing browser check passed.
  - Residual evidence: receipt covers variable row height, focus, scroll-to-record, identity/order,
    and unwindowed shared markup.

- [x] **M4 close-out.** The separate 2026-09-23 M4 `all_test.sh` invocation exited 0 on the
      current integrated tree with all aggregate gates and live-service acceptance passing. This
      receipt does not reconstruct an M4 intermediate snapshot; the three focused browser receipts
      remain separately recorded above.

## M5 seven proof pages

- [x] **WP-E1 - Question Drafts.** `question_drafts_page.tsx` imports `RecordList`.
  - [x] Browser and source review passed.

- [x] **WP-E2 - Gradebook.** `gradebook_page.tsx` imports `RecordList` and has its companion
      RecordList stylesheet.
  - [x] Browser and source review passed.
  - [x] Production route evidence confirms heading order and aligned region edges across all four
        canonical viewports; visual review covered the tablet's visible and scrolled score columns.

- [x] **WP-E3 - Library Browse.** `library_browse_rows.tsx` imports `RecordList` and is the
      windowing proof consumer.
  - [x] Browser and source review passed.

- [x] **WP-E4 - Assessment Questions.**
      `assessment_workspace_questions_view.tsx` imports `RecordList`.
  - [x] Browser and source review passed.

- [x] **WP-E5 - Course Instance.** `course_instance_page.tsx` imports `RecordList`.
  - [x] Browser and source review passed.

- [x] **WP-E6 - Student Course landing.** `student_course_landing_page.tsx` imports
      `RecordList`; the active plan records this as the achieved Student proof page.
  - [x] The Course long name is the PageFrame title on Student Course landing and Grades; the
        optional entry banner remains content. The focused landing check scopes that title to
        PageFrame, and four-size visual review confirms the task content remains subordinate.
  - [x] Browser and direct four-size review passed, including the phone action/identity placement.

- [x] **WP-E7 - avatar picker.** `provided_avatar_picker.tsx` imports `RecordList`; the dedicated
      presentation harness/test is present.
  - [x] Browser and source review passed.

- [x] **M5 close-out.** An independent reviewer found no blocker across all seven proof pages, and
      their focused checks/reviews passed. The separate 2026-09-23 M5 `all_test.sh` invocation
      exited 0 on the current integrated tree with all aggregate gates and live-service acceptance
      passing. This receipt does not reconstruct an M5 intermediate snapshot.

## M6 page and date sweep

- [x] **WP-F1 - top-level pages.** Current top-level page files import `PageFrame` broadly.
  - [x] The read-only source sweep found no `class="page"` or `className="page"`, and no
        `toLocaleDateString()` or `toLocaleTimeString()` calls. Remaining `toLocaleString()` calls
        format numeric counts, not dates or times.

- [x] **WP-F2 - assessment workspace pages.** Current workspace page files import `PageFrame`.
  - [x] WP-E4 `assessment_workspace_questions_view.tsx` remains explicitly excluded from WP-F2
        and is the sole proof-page consumer of the questions `RecordList`/reorder view. Its
        dedicated `assessment_questions_record_list.css` rules remain in use.
  - [x] Current shared workspace page styles have no `assessment-workspace-header` markup
        consumer. `assessment_workspace_authoring.css` has no TSX use for legacy
        `.assessment-editor-list`, outer `.assessment-editor-row`/heading,
        `.assessment-editor-row-actions`, `.assessment-editor-pool`,
        `.assessment-editor-pool-fields`, `.assessment-editor-pool-entries`,
        `.assessment-editor-pool-preview`, and `.assessment-editor-question-results` groups.
        Keep `.assessment-editor-row-description` and `.assessment-editor-row-facts` because
        selected-entry markup still uses them. No safely attributable unused tokens were found.
        Record the unused rules as cleanup candidates; do not delete them in this work package.

- [x] **WP-F3 - feature pages.** Current feature page files import `PageFrame`.
  - [x] WP-E7 `provided_avatar_picker.tsx` remains explicitly excluded from WP-F3 and owns the
        shared Gallery/List proof. All current `provided_avatar_picker.css` selectors and spacing
        tokens have current or conditional markup consumers; no unused rules or tokens were found.
        `AvatarVisual` styles have Profile, Instructor Accounts, and shell consumers, so retain them.

- [x] **M6 close-out.** The separate 2026-09-23 M6 `all_test.sh` invocation exited 0 on the
      current integrated tree with all aggregate gates and live-service acceptance passing. This
      receipt does not reconstruct an M6 intermediate snapshot. The source sweep does not establish
      responsive browser behavior. WP-F formatter reuse was separately reviewed and its focused
      presentation test passed 4/4; `npx tsc --noEmit`, the six-file Prettier check, and
      `git diff --check` passed. At M6 completion, the fast aggregate passed 9,268 pytest tests plus
      Rust, TypeScript, Node, lint, and formatting checks. This dated receipt is distinct from the
      later 9,287-test Course-identity follow-up receipt in the Evidence lanes and from earlier
      full-suite and Live Demo replay receipts.

## M7 cleanup and screenshot evidence

- [x] **WP-C5 - retire shared stylesheet rules.** The obsolete width caps and unused `.page`
      geometry are removed, `--ple-theme-primary` is defined, and course-theme positioning targets
      the scoped `.page-frame`. The redundant private `.auth-page`/`.roster-page` width rule is
      gone, and SignInPage no longer passes the unused `auth-page` marker. `roster-page` remains
      for page-owned descendant styling; legacy shared roster styling remains for its named
      follow-up. The unused `assessment-attempt-page` and four `course-appearance` PageFrame content
      classes are removed. The `pending-invitations-page` width cap is removed because PageFrame owns
      this route's reading width; `.pending-invitation-*` descendant styles remain.
  - [x] PageFrame-root `.route-error` content keeps its compact 48rem cap and aligns with the frame's
        left origin. A production-shell Gradebook comparison reviewed the synthetic status state
        before and after; it is visual layout evidence, not live route-failure acceptance.
  - [x] The temporary dangling-token/user scan found no undefined nonfallback CSS variable and was
        removed.
  - Residual evidence: retained legacy roster classes remain follow-up work; no CSS variable is
    read without a definition.

- [x] **WP-G1 - define viewport coverage.** The fresh publication supplies a 139-capture manifest;
      the corpus sources define Student, Instructor, Sysadmin, and Public scenarios.
  - [x] Fresh publication passed at about 22:48 UTC with direct Student laptop, tablet, phone, and
        square captures plus Instructor and Sysadmin laptop captures.
  - [x] Public scope is unchanged.
  - [x] A fresh `image_evaluator` reviewed all 139 current artifacts under the role/viewport policy:
        Student laptop and tablet (49), Student phone and square (49), Instructor laptop (33), and
        current Sysadmin and Public scope (8). Student views keep the course name out of the Ribbon;
        no actionable responsive composition, overlap, or clipping issue was found.
  - [x] Triaged visual findings. The initial Instructor crop concern is intentional: `publication_review.png`
        and the WeBWorK DNA/meiosis captures scroll to show publication fields or preview; the other
        four flagged Instructor images retain shell and title. No recapture is justified. Sysadmin
        `account_created.png` shows a crowded route-level Instructor Account form outside `PageFrame`
        and `RecordList`; defer it without coding as LOW-priority Sysadmin cleanup. Student
        `question_answered_webwork.png` is a candidate because its large embedded area appears blank
        compared with the unanswered capture. Static images cannot establish a rendering defect;
        route it to a separate question-rendering investigation, not the shared UI backbone.
  - Static-review limit: these captures establish visible composition only. They do not establish
    interactive behavior, accessibility, or whether the WebWork difference is a runtime defect.
  - Residual evidence: `covered_by` is documented only as substitution, never equivalence.

- [x] **WP-G2 - seed theme variety.** Current screenshot-corpus sources include scenario theme
      support; fresh `--verify-static`, 13 screenshot-corpus tests, and the atlas link check passed.
  - [x] One clean `--fresh --verify` Live Demo semantic replay passed after final capture generation.
  - [x] That replay passed semantic workflow, manifest closure, scenario privacy, dimensions, and
        published-artifact integrity.
  - [x] The replay observed 130 byte-different PNGs across clean resets because visible IDs and
        dates vary. This is observational evidence, not a speed, pixel, count, or byte-identity
        gate.
- [x] Follow-up clean replay on 2026-09-23 passed after the Sysadmin lifecycle fixture captured its
      created/deactivated row before reload and verified persistence afterward. Visual review of the
      replay and published images confirmed both named states are visible; 135 byte differences are
      observational only.
- [x] SUI-03 responsive Course identity uses the descriptive long name when it fits and switches to
      the Instructor-defined short name when the breadcrumb trail is constrained. The production
      ApplicationShell, AppRibbon, BreadcrumbPrelude, PageFrame, and shared styles passed the
      focused Chromium fixture at 1280px, 320x640 with 200% root text, and 393x852 with 200% root
      text. The long name appears in both breadcrumb and PageFrame heading at laptop width; both
      narrow captures show `Home / BCHM 355` and the long PageFrame heading wraps naturally. The
      browser fixture passed without document horizontal overflow or page/console errors. A fresh
      image evaluator found the Course identity and hidden measurement span visually clear. Route,
      API, and page data are synthetic; this is not full-stack acceptance. The 320px/200% Instructor
      stress capture also shows crowded/clipped Ribbon controls; that viewport is outside the plan's
      Instructor screenshot policy and was not changed here. Artifacts:
      `/private/tmp/ple_ribbon_m11_course_assessments.png`,
      `/private/tmp/ple_ribbon_m2_routed_shell_320x640_text200.png`, and
      `/private/tmp/ple_course_breadcrumb_393x852_text200.png`.

- [x] **Warm-loop runner.** `bash tests/e2e/e2e_screenshot_warm_loop.sh` rebuilt the stale client
      bundle and passed full replay verification in 378 seconds. Its 112 replay byte differences are
      observational only; the owned Live Demo was stopped afterward.

- [x] **M7 close-out.** Code cleanup, focused/static evidence, clean semantic replay, and screenshot
      receipt are recorded above. The dated 2026-09-23 aggregate `all_test.sh` receipt predates the
      final source edits; the final snapshot has no refreshed full-suite receipt, which is not a
      gate under this plan's fast-and-focused acceptance policy.

## M8 documentation and handoff

- [x] **WP-H1 - write follow-up plans.** The five named follow-up files are present in
      `docs/active_plans/active/` and `docs/active_plans/decisions/`.
  - [x] Fresh source reconciliation gives 54 sites: 7 converted plus 47 remaining.
  - Residual evidence: each deferred item appears once and only once.

- [x] **WP-H2 - record decisions.** Current `HUMAN_GUIDANCE.md`,
      `DESIGN_DECISIONS.md`, and `CODE_ARCHITECTURE.md` cover the listed decisions.
  - [x] Human Guidance and `DESIGN_DECISIONS.md` agree that unavailable required Instructor
        destinations are annotation-free controls, not usable links.
  - [x] Durable sections cover PageFrame composition, reserve-but-never-fabricate, RecordList and
        windowing ownership, and the scan-row summary rule.

- [x] **WP-H3 - stage audit closure.** The active plan contains the SUI-01 through SUI-08 staging
      disposition and `docs/CHANGELOG.md` has concurrent milestone documentation work.
  - [x] The staging disposition maps each audit finding to one milestone or named follow-up plan
        without changing the historical audit.
  - [x] `tests/_temp/` contains no files.
  - [x] Record the formatter reuse and SUI-03 headless visual evidence in the 2026-09-23 changelog
        entry, alongside the latest 9,287-test fast-check receipt. Earlier full-suite, clean replay,
        warm-loop, and screenshot receipts remain dated evidence for their own scope.

- [x] **M8 close-out.** The handoff records documentation files changed, focused and aggregate
      fast checks, earlier full-suite/live replay/screenshot receipts, the unrefreshed final-snapshot
      full-suite status, and remaining SUI follow-ups. Keep this plan active while SUI-04, SUI-06,
      and SUI-08 follow-up acceptance remains open.

## Final manager handoff

- [x] Reopen the authoritative plan and Human Guidance; confirm the M1-M8 scope and Student
      four-viewport versus Instructor/Sysadmin laptop-only policy. Six separately invoked historical
      full-suite runs passed on the current integrated tree; no full-suite rerun followed the final
      source edits, per the fast-and-focused gate selection.
- [x] Report the three documentation files updated for this close-out without claiming ownership
      of concurrent source, style, or screenshot changes.
- [x] Separate focused-test, fast aggregate, earlier full-suite, live-browser, and screenshot
      receipts in the close-out record.
- [x] Fresh architect review found no unresolved in-scope cross-cutting design question. The
      Student/Sysadmin tier-2 contents question remains the explicitly non-blocking follow-up in
      the plan; no additional design recommendation or human approval is needed.
