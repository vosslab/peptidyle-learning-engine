# Plan: Modular UI backbone for PLE

## Context

[human-UI-review.md](../../nsh/PROBLEMS/peptidyle-learning-engine/human-UI-review.md) opens with the
main complaint: "every layout feels hacked together and custom, when it should use basic tools to
build from," and asks for "a more fixed modular design that enforces these issues."

The problem is in the design, not in the styling. Four findings from reading the code:

- The top Ribbon is keyed by route scope. Entering a Course replaces
  `Courses | Questions | Assessments` with `Assessments | Students | Gradebook | ...`. The bar
  changing on every page is what the current design asks for
  ([src/ribbon/ribbon_schema.ts:43-75](../../nsh/PROBLEMS/peptidyle-learning-engine/src/ribbon/ribbon_schema.ts)).
- Shell height is conditional. The task row collapses when a route declares no tasks, and the
  breadcrumb row draws only when reserved, so the page content slides up and down as you navigate
  ([src/ribbon/app_ribbon.css:62-68](../../nsh/PROBLEMS/peptidyle-learning-engine/src/ribbon/app_ribbon.css),
  [src/application_shell.tsx:207-213](../../nsh/PROBLEMS/peptidyle-learning-engine/src/application_shell.tsx)).
- There is no shared list, row, table, or page component. About 45 list sites in ~40 files each
  write their own markup, empty state, loading state, and error state. Reorder is written six times
  with six shapes; one of the six works with a keyboard.
- `contentLayout` (`reading` or `fullWidth`) is set on every route and passed into the Ribbon model,
  but nothing reads it. Pages write `class="page"` by hand in ~38 files, and several set their own
  width, so one workflow changes width from page to page.

PLE has no users and no production data. That means we change the contracts themselves rather than
work around them.

## Objectives

- A role sees the same top-level tabs on every page it can reach.
- The breadcrumb row and the page content start at the same height on every signed-in page, for
  every role. A row with no controls still holds its space.
- One page component owns page width and the page heading.
- One list component owns row alignment, empty and error states, and narrow-screen behavior.
- One date function owns date text, and the time zone is named once per page rather than per row.
- Seven different list pages run on the new list component with no page-specific escape hatch.
- The screenshot set covers all four screen sizes so the new components can be checked visually.

## Design philosophy

This plan fixes the design, not the symptom. The Ribbon does not change per page because of a CSS
bug; it changes because tier-1 is keyed by route scope. The rows do not misalign because of padding;
they misalign because each row is its own grid. So the plan edits the contracts that allow the
problem: the Ribbon schema, the route contract, and the shell's height rules.

Trade-off this plan accepts: it spends effort on a route-contract change (adding `tierOneArea` to 39
routes) that a CSS patch would have avoided. That cost is paid once. The patch would be paid on
every future page. This follows "fix the design, not the symptom" and "long-term over short-term"
from [docs/REPO_STYLE.md](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/REPO_STYLE.md), and uses
the pre-production state to change foundations directly.

Rejected alternative: keep the scope-keyed Ribbon and add per-route overrides to force the same tabs
to appear. That hides the real rule behind a growing exception list, and each new route becomes a
chance to get it wrong. It was rejected even though it is smaller.

Second rejected alternative: add `@tanstack/solid-table`. It supplies column, sort, and filter
state, which PLE mostly gets from the server. It does not supply aligned rows, narrow-screen
priority, or keyboard reorder, which are the three things actually asked for. The user chose to
build in-repo.

- Evidence strategy for uncertain methods: two questions are not settled by reading code. Which
  slots belong in the page component (WP-C1 counts what the ~45 pages actually use). Whether the six
  reorder sites share one behavior (WP-D2 compares them on save timing, disabled rule, failure, and
  announcement, then splits shared from separate). Both probes live in `tests/_temp/` and are
  deleted by the work package that runs them.

## Scope

- Make tier-1 tabs depend on Product Role only, and move scope-specific tabs to tier-2.
- Add `tierOneArea` to the route contract and select tier-1 from it.
- Pin the student `Coursework` and `Grades` tabs to the current course.
- Make the breadcrumb row and the tier-2 row always present for every signed-in role, whether or not
  the tier-2 row has controls yet.
- Build `PageFrame` and apply it to all ~38 pages.
- Build `RecordList` with opt-in presentation variants, reorder, and windowing.
- Build one date function and move all 16 call sites onto it.
- Convert seven list pages that each exercise a different pattern.
- Expand the screenshot set to four screen sizes with a seeded course theme.
- Add permanent browser checks for the Ribbon, shell height, and list alignment.
- Write the follow-up plans for the work this plan defers.

## Non-goals

- Convert the remaining ~34 list pages. They go to a follow-up plan once the patterns are proven.
- Remove the undo and restore buttons, flatten the WeBWorK boxes, fix question navigation, or change
  phone branding. Real problems from the review, but none is needed to prove the components, and the
  WeBWorK work carries its own risk across two documents.
- Build My Questions or Starred. Those need routing, query work, SQL, and grants -- a different
  stack.
- Decide what goes **inside** the student or sysadmin tier-2 row. The row itself is built and holds
  its space; only its contents wait. Instructor tier-2 is enough to prove the shell.
- Add the dark halves of the biome themes.
- Run a blanket spacing sweep over files this plan does not already touch.

## Current state summary

| Area | Now | After |
| --- | --- | --- |
| Tier-1 tabs | Keyed by route scope; change per page | Keyed by Product Role; same everywhere |
| Tier-2 row | Collapses when empty; content shifts | Always present for every role; empty is valid |
| Breadcrumb row | Drawn only when reserved | Always present when signed in |
| Page width | `class="page"` in ~38 files, plus 4 private widths | `PageFrame` reads `contentLayout` |
| `contentLayout` | Set on 39 routes, read by nothing | Read by `PageFrame` |
| Lists | ~45 hand-written sites, no shared piece | `RecordList`; 7 converted, ~33 queued |
| Reorder | 6 versions, 1 keyboard-capable | 1 shared version, opt-in |
| Dates | 3 styles in 16 files, 1 hardcoded to UTC | 1 function, time zone named once per page |
| Screenshots | Student: 30 laptop, 22 phone, 3 tablet, 1 square | Four sizes per scenario |

## Architecture boundaries and ownership

Each workstream owns its files outright. No file is written by two streams. The one real overlap --
the seven proof pages are also pages in the sweep -- is removed by giving those seven to WS-PROOF
and excluding them from WS-PAGES.

Date call sites sit inside page files, so replacing them belongs to whichever stream owns that page.
WS-CORE ships the date function itself and touches no page.

`src/style.css` is shared by nearly everything, so it gets one owner: WS-CORE. Other streams never
edit it. When a sweep leaves a rule or token unused, that stream reports the name and WS-CORE
removes it in WP-C5. This keeps the "one writer per file" guarantee literally true rather than
nearly true.

| Workstream | Owns | Review boundary |
| --- | --- | --- |
| WS-SHELL | `src/ribbon/**`, `src/route_contract.ts`, `src/application_shell.tsx`, `src/navigation/**` | Ribbon and route contract |
| WS-CORE | `src/components/page_frame.*`, `src/components/record_list/**`, `src/format_datetime.ts`, and **`src/style.css`** | New shared components and the shared stylesheet |
| WS-PROOF | The seven pages in the M5 table | List conversion patterns |
| WS-PAGES | All other `src/pages/**` and `src/features/**` page files | Page heading sweep |
| WS-EVIDENCE | `tests/playwright/**`, `tests/playwright/screenshot_corpus/**` | Checks and screenshots |
| WS-DOCS | `docs/**` | Documentation |

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M1 / WS-SHELL | Ribbon schema, catalog, route contract, shell | Ribbon and route contract |
| M2 / WS-EVIDENCE | Ribbon and shell browser checks | Checks |
| M3 / WS-CORE | `PageFrame`, `RecordList` core, date function | New shared components |
| M4 / WS-CORE | Variants, reorder, windowing | New shared components |
| M5 / WS-PROOF | Seven converted pages | List conversion patterns |
| M6 / WS-PAGES | ~32 swept pages | Page heading sweep |
| M7 / WS-CORE | Shared stylesheet cleanup | Shared stylesheet |
| M7 / WS-EVIDENCE | Screenshot set | Screenshots |
| M8 / WS-DOCS | Docs and follow-up plans | Documentation |

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1 | Shell contract | Tier-1 by role; tier-2 for scope; rows always reserved | Same tabs and same height on every signed-in page |
| M2 | Shell checks | Browser checks for tabs and height | Drift fails the build |
| M3 | Shared components | `PageFrame`, `RecordList` core, date function | The three pieces exist |
| M4 | List options | Variants, reorder, windowing, opt-in | Lists pay only for what they use |
| M5 | Proof: seven pages | Seven patterns on `RecordList` | The component is proven |
| M6 | Page sweep | `PageFrame` and dates on ~32 pages | One page shape everywhere |
| M7 | Cleanup and screenshots | Retire dead CSS, then capture four sizes | Visual evidence across sizes |
| M8 | Docs and handoff | Decisions recorded, follow-ups written | Nothing deferred is lost |

### Milestone: M1 shell contract

- Depends on: none.
- Deliverables: WP-A1 through WP-A5.
- Workstreams: WS-SHELL.
- Entry criteria: none.
- Exit criteria: `npx tsc --noEmit` passes; tier-1 ids are the same for a role on every route it can
  reach; no branch anywhere in the shell changes a row height.
- Parallel-plan ready: no. M1 rewrites one contract that every other package in it reads. Splitting
  it would mean two owners editing `ribbon_catalog.ts` and `route_contract.ts` at once. It runs as
  one lane, concurrently with M3.

### Milestone: M2 shell checks

- Depends on: M1, because the checks assert the contract M1 creates.
- Deliverables: WP-B1, WP-B2.
- Workstreams: WS-EVIDENCE.
- Entry criteria: M1 exit criteria met.
- Exit criteria: both runners pass at all four sizes and fail when M1 is reverted.
- Parallel-plan ready: yes. WS-E-tabs (WP-B1) and WS-E-height (WP-B2) write separate files and share
  no state. Maximum 2, because there are two checks.

### Milestone: M3 shared components

- Depends on: none. It creates new files and reads no Ribbon internals.
- Deliverables: WP-C1 through WP-C4.
- Workstreams: WS-CORE.
- Entry criteria: none.
- Exit criteria: `contentLayout` has a reader; `RecordList` core holds no variant, reorder, or
  windowing code; the date function exists with no callers yet.
- Parallel-plan ready: yes. WS-C-page (WP-C1, WP-C2), WS-C-list (WP-C3), and WS-C-date (WP-C4) own
  separate new files. Maximum 3, one per file set.

### Milestone: M4 list options

- Depends on: M3, because each option plugs into the core built there.
- Deliverables: WP-D1 through WP-D3.
- Workstreams: WS-CORE.
- Entry criteria: WP-C3 done.
- Exit criteria: a list that uses no option imports no option code.
- Parallel-plan ready: yes. WS-D-variants, WS-D-reorder, and WS-D-window write three separate files
  next to the core and do not edit it. Maximum 3. If any of them needs to edit the core, that stream
  stops and hands the core change back to the manager, so two streams never edit the core at once.

### Milestone: M5 proof, seven pages

- Depends on: M3 and M4, because the pages need the core and its options.
- Deliverables: WP-E1 through WP-E7, one per page.
- Workstreams: WS-PROOF.
- Entry criteria: M4 exit criteria met.
- Exit criteria: all seven render through `RecordList` with no page-specific escape hatch and no
  prop that exists to satisfy one caller; assessment rows measure 100-150px.
- Parallel-plan ready: yes, with one rule. The seven pages are separate files, so seven lanes are
  possible. The core is expected to change here -- that is the point of proving it -- and a core
  change must not happen in seven places at once. So core changes are reported to the manager,
  applied once in WS-CORE, and the lanes continue. Maximum 7; practical 3.

### Milestone: M6 page sweep

- Depends on: M3 only. It uses `PageFrame` and the date function, not `RecordList`.
- Deliverables: WP-F1, WP-F2, WP-F3.
- Workstreams: WS-PAGES.
- Entry criteria: WP-C2 and WP-C4 done.
- Exit criteria: no `class="page"` outside the component; no `toLocaleString(` or
  `new Intl.DateTimeFormat(` outside the date function.
- Parallel-plan ready: yes. Split the ~32 pages into three groups by directory: `src/pages/`
  top-level, `src/pages/assessment_workspace/`, and `src/features/`. Groups do not share files.
  Maximum 3. Runs at the same time as M5.

### Milestone: M7 stylesheet cleanup and screenshots

- Depends on: M5 and M6, so the pictures show the new components and the final stylesheet.
- Deliverables: WP-C5, WP-G1, WP-G2.
- Workstreams: WS-CORE for WP-C5, then WS-EVIDENCE.
- Entry criteria: M5 and M6 exit criteria met, and both sweeps have reported the rules and tokens
  their pages stopped using.
- Exit criteria: no CSS variable is read without a definition; every scenario has four sizes or a
  recorded `covered_by` reason; two capture runs are byte-identical.
- Parallel-plan ready: no, and the order matters. WP-C5 runs first so the screenshots show the final
  stylesheet; WP-G1 and WP-G2 then write the same manifest, and the capture script owns the Live
  Demo. One lane.

### Milestone: M8 docs and handoff

- Depends on: M5, so the follow-up plans can record what proving the component taught us.
- Deliverables: WP-H1 through WP-H3.
- Workstreams: WS-DOCS.
- Entry criteria: M5 exit criteria met.
- Exit criteria: every deferred item appears once with its evidence; every SUI finding maps to a
  milestone here or a named follow-up plan; no probe left in `tests/_temp/`.
- Parallel-plan ready: yes. WP-H1 (follow-up plans), WP-H2 (decision records), and WP-H3 (changelog
  and audit closure) write separate files. Maximum 3.

## Workstream breakdown

### Workstream: WS-SHELL

- Goal: make tier-1 depend on role, move scope tabs to tier-2, and stop the shell changing height.
- Owner: `expert_coder`.
- Work packages: WP-A1, WP-A2, WP-A3, WP-A4, WP-A5.
- Needs: nothing.
- Provides: a stable tab list and stable heights for WS-EVIDENCE to check.
- Review boundary, when modifying the repository: Ribbon and route contract.

### Workstream: WS-CORE

- Goal: build the three shared pieces and their options.
- Owner: `expert_coder` for the core, `coder` for the options.
- Work packages: WP-C1 through WP-C5, WP-D1 through WP-D3.
- Needs: nothing, except that WP-C5 waits for the sweeps to report unused rules.
- Provides: `PageFrame` for WS-PAGES and WS-PROOF; `RecordList` for WS-PROOF; the date function for
  both.
- Review boundary, when modifying the repository: new shared components.

### Workstream: WS-PROOF

- Goal: prove `RecordList` against seven different patterns, and fix it where it does not fit.
- Owner: `coder`, with core changes handed back to WS-CORE.
- Work packages: WP-E1 through WP-E7.
- Needs: `RecordList` and its options from WS-CORE.
- Provides: the proven patterns that the follow-up migration plan copies.
- Review boundary, when modifying the repository: list conversion patterns.

### Workstream: WS-PAGES

- Goal: put every other page on `PageFrame` and the date function.
- Owner: `coder`.
- Work packages: WP-F1, WP-F2, WP-F3.
- Needs: `PageFrame` and the date function.
- Provides: one page shape for the screenshots to show.
- Review boundary, when modifying the repository: page heading sweep.

### Workstream: WS-EVIDENCE

- Goal: add the permanent checks and expand the screenshot set.
- Owner: `tester`.
- Work packages: WP-B1, WP-B2, WP-G1, WP-G2.
- Needs: M1 for the checks; M5 and M6 for the screenshots.
- Provides: the gates that keep the contracts from drifting.
- Review boundary, when modifying the repository: checks and screenshots.

### Workstream: WS-DOCS

- Goal: record what was decided and write down what was deferred.
- Owner: `maintainer` for records, `planner` for follow-up plans.
- Work packages: WP-H1, WP-H2, WP-H3.
- Needs: M5 results.
- Provides: the follow-up plans.
- Review boundary, when modifying the repository: documentation.

## Work packages

### Work package: WP-A1 add `tierOneArea` to the route contract

- Owner: `expert_coder`.
- Touch points:
  [src/route_contract.ts](../../nsh/PROBLEMS/peptidyle-learning-engine/src/route_contract.ts).
- Depends on: none.
- Acceptance criteria: all 39 routes carry one of `courses`, `questions`, `productAssessments`,
  `coursework`, `grades`, `instructorAccounts`, `disciplines`, `account`. The value is the tab a
  user would press to reach the route. `profile`, `accountSettings`, `signIn`, and the invitation
  routes take `account`, which selects nothing. No optional field, no default.
- Evidence or review, when useful: `npx tsc --noEmit`.
- Obvious follow-ons: WP-A2.

### Work package: WP-A2 key tier-1 to Product Role

- Owner: `expert_coder`.
- Touch points:
  [src/ribbon/ribbon_schema.ts](../../nsh/PROBLEMS/peptidyle-learning-engine/src/ribbon/ribbon_schema.ts),
  [src/ribbon/ribbon_contract.ts:360-368](../../nsh/PROBLEMS/peptidyle-learning-engine/src/ribbon/ribbon_contract.ts).
- Depends on: WP-A1, because selection reads the new field.
- Acceptance criteria: `SCHEMAS` is replaced by
  `PRODUCT_TIER_ONE: Record<ProductRole, ReadonlyArray<RibbonSchemaSlot>>`; `ribbonSchemaFor` no
  longer takes `scope`; instructor is `courses, questions, productAssessments`, student is
  `courses, coursework, grades`, sysadmin is
  `courses, questions, instructorAccounts, disciplines`; `selectedFor` reads `tierOneArea`.
- Obvious follow-ons: WP-A3.

### Work package: WP-A3 move scope tabs to tier-2

- Owner: `expert_coder`.
- Touch points:
  [src/ribbon/ribbon_catalog.ts](../../nsh/PROBLEMS/peptidyle-learning-engine/src/ribbon/ribbon_catalog.ts),
  [src/route_contract.ts](../../nsh/PROBLEMS/peptidyle-learning-engine/src/route_contract.ts).
- Depends on: WP-A2.
- Acceptance criteria: `assessments`, `students`, `gradebook`, `teachingOperations`,
  `blueprintUpdates`, `courseSetup`, `studentAssessments`, and `attempt` move from `TAB_CATALOG` to
  `RIBBON_TASK_CATALOG`; `coursework` and `grades` are added to `TAB_CATALOG`; `taskGroup` is filled
  in on `courseAssessments`, `gradebook`, `courseRoster`, `assessmentCreate`; no `TAB_CATALOG` entry
  declares `requiredParams`.
- Obvious follow-ons: WP-A4.

### Work package: WP-A4 pin student tabs to a course

- Owner: `expert_coder`.
- Touch points:
  [src/ribbon/route_scope_controller.ts](../../nsh/PROBLEMS/peptidyle-learning-engine/src/ribbon/route_scope_controller.ts),
  [src/route_contract.ts](../../nsh/PROBLEMS/peptidyle-learning-engine/src/route_contract.ts).
- Depends on: WP-A3.
- Acceptance criteria: `currentCourseInstanceId` is set whenever a `courseInstance` or
  `assessmentAttempt` scope resolves and kept for the session; `coursework` and `grades` build links
  from it through `buildRoutePath`; with no course pinned they point at `/student`; a new route
  `studentCourseGrades` at `/student/courses/:courseInstanceId/grades` shows `assessmentScore` and
  `gradedQuestionCount`, which
  [src/api/decoders/live_student_course_landing.ts:111-171](../../nsh/PROBLEMS/peptidyle-learning-engine/src/api/decoders/live_student_course_landing.ts)
  already returns, so no new endpoint is needed.
- Evidence or review, when useful: from an active Attempt, both links carry that Attempt's course.

### Work package: WP-A5 make shell height unconditional

- Owner: `expert_coder`.
- Touch points:
  [src/ribbon/app_ribbon.css](../../nsh/PROBLEMS/peptidyle-learning-engine/src/ribbon/app_ribbon.css)
  lines 5-9, 62-68, 590-594;
  [src/application_shell.tsx](../../nsh/PROBLEMS/peptidyle-learning-engine/src/application_shell.tsx);
  [src/ribbon/ribbon_contract.ts](../../nsh/PROBLEMS/peptidyle-learning-engine/src/ribbon/ribbon_contract.ts).
- Depends on: WP-A3.
- Acceptance criteria: **every signed-in role reserves the tier-2 row.** Delete the
  `[data-ribbon-task-row="absent"]` rule, the `0rem` default, the `ribbonTaskRow` memo, and the
  `data-ribbon-task-row` attribute. Remove `breadcrumbPreludeReserved` and draw `BreadcrumbPrelude`
  for every route that has a Ribbon model. No policy constant is added, because there is no longer
  a conditional to name. Square the Ribbon and breadcrumb corners per
  [docs/HUMAN_GUIDANCE.md:276](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/HUMAN_GUIDANCE.md).
  Also strip the "Not available yet" text from `UnavailableRibbonChoice`
  ([src/ribbon/app_ribbon.tsx:112-132](../../nsh/PROBLEMS/peptidyle-learning-engine/src/ribbon/app_ribbon.tsx)),
  keeping the label, the disabled look, and the accessible description.
- Why this shape: two questions were tangled together. *Does the row take space?* and *does the row
  hold controls?* Separating them removes the conditional layout state instead of parameterizing it.
  The answer to the first is always yes. The answer to the second is yes where the role and workflow
  have real controls -- today, instructors, whose contents are already listed in
  [docs/HUMAN_GUIDANCE.md:430,490,568](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/HUMAN_GUIDANCE.md).
  Student and sysadmin rows render empty until their contents are decided.
- Notes: 17 routes declare no tasks, and an empty reserved row is a valid result for them. Empty
  means empty. It is not a reason to invent controls.

### Work package: WP-B1 tab check

- Owner: `tester`.
- Touch points: `tests/playwright/ribbon_tier_one_invariance.mjs`.
- Depends on: WP-A2.
- Acceptance criteria: for each role, the tier-1 control **id** sequence is the same on every route
  that role can reach. It reads `data-ribbon-control-id`, not visible text or DOM shape, so wording
  and markup can change later without touching the check.

### Work package: WP-B2 height check

- Owner: `tester`.
- Touch points: `tests/playwright/shell_geometry_invariance.mjs`.
- Depends on: WP-A5.
- Acceptance criteria: for every signed-in route a role can reach, the breadcrumb top offset and the
  `#main-content` top offset each take one value per screen size. **All three roles are enrolled
  now**, with no exceptions and no enrolled-role list, because WP-A5 removed the conditional that
  made an exception necessary. The value may differ per role if the role tag legitimately changes
  the Ribbon's height; it may not differ per route.

### Work package: WP-C1 measure page headings

- Owner: `tester`.
- Touch points: `tests/_temp/page_header_shape.mjs`.
- Depends on: none.
- Acceptance criteria: reports, for each of the ~45 files using `class="page"` or `eyebrow`, which of
  eyebrow, title, lede, and action row it uses. A slot belongs in `PageFrame` when it positions or
  bounds the page rather than saying something about the page's subject, and the count shows pages
  rebuilding it. Frequency alone does not decide. Time zone text is excluded: it is page content,
  not layout.
- Obvious follow-ons: delete the probe; WP-C2.

### Work package: WP-C2 build `PageFrame`

- Owner: `expert_coder`.
- Touch points: `src/components/page_frame.tsx`, `src/components/page_frame.css`.
- Depends on: WP-C1.
- Acceptance criteria: holds the slots WP-C1 admitted plus `contentLayout`, read from shell context
  so pages do not restate it; `h1` is `clamp(1.25rem, 1.6vw, 1.6rem)`; the component sets the page
  width itself so the private widths in `src/style.css` stop being used; nothing about a page's
  subject leaks into the component. Deleting those now-unused widths happens in WP-C5, once the
  sweeps confirm no page still relies on them.

### Work package: WP-C3 build `RecordList` core

- Owner: `expert_coder`.
- Touch points: `src/components/record_list/region_spec.ts`, `record_list.tsx`, `record_list.css`.
- Depends on: none.
- Acceptance criteria: a `RecordRegion<Row>` declares `id`, `role` (`identity`, `metadata`,
  `status`, `actions`), `width`, `align`, `priority`, and a `content` function that may return
  structured markup rather than one value. The list owns `grid-template-columns` and each row is
  `display: grid; grid-template-columns: subgrid`, which is the fix for
  [src/style.css:327](../../nsh/PROBLEMS/peptidyle-learning-engine/src/style.css) where each row is
  its own grid and columns line up only by luck. Rows are separated by a divider or a light
  alternating background, not a bordered card, per
  [docs/HUMAN_GUIDANCE.md:293-295](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/HUMAN_GUIDANCE.md).
  Spacing inside a region is one step tighter than between regions. `priority` decides what drops
  first on a narrow screen so identity and the main action survive. The core holds no variant,
  reorder, or windowing code.

### Work package: WP-C4 build the date function

- Owner: `coder`.
- Touch points: `src/format_datetime.ts`.
- Depends on: none.
- Acceptance criteria: one `Intl.DateTimeFormat` wrapper taking an explicit display time zone. No
  call sites are changed in this package.

### Work package: WP-C5 retire shared stylesheet rules

- Owner: `expert_coder` in WS-CORE, the single owner of
  [src/style.css](../../nsh/PROBLEMS/peptidyle-learning-engine/src/style.css).
- Touch points: `src/style.css` only.
- Depends on: M5 and M6, because a rule can only be removed once its last user is gone.
- Acceptance criteria: remove the per-page widths `.assessment-attempt-page`, the `.question-card`
  cap, `.attempt-summary`, and `--ple-assessment-workflow-max-inline`; remove the one-off layout
  tokens at `style.css:33-75` that WS-PROOF and WS-PAGES reported as unused; keep tokens still read
  by unconverted pages, which the follow-up migration plan retires; define `--ple-theme-primary`,
  which three rules read and nothing defines.
- Evidence or review, when useful: `tests/_temp/dangling_token_scan.mjs` shows no CSS variable is
  read without a definition and no removed rule still has a user, then is deleted.
- Notes: WP-C2 builds `PageFrame` but does not delete the widths it replaces; deletion waits here,
  so `src/style.css` has exactly one writer across the whole plan.

### Work package: WP-D1 presentation variants

- Owner: `coder`.
- Touch points: `src/components/record_list/record_list_presentation.ts`.
- Depends on: WP-C3.
- Acceptance criteria: each list declares the variants its content supports, and a switcher appears
  only where there is more than one. There is no global `table | comfortable | compact` set and no
  saved preference. Single-variant lists include gradebook, roster, and drafts. Multi-variant lists
  are the question library (`scan`, `preview`) and the avatar picker (`gallery`, `list`).

### Work package: WP-D2 reorder

- Owner: `expert_coder`.
- Touch points: `tests/_temp/reorder_semantics.mjs`, then
  `src/components/record_list/record_list_reorder.ts`.
- Depends on: WP-C3.
- Acceptance criteria: the probe first compares the six existing reorder sites on save timing,
  disabled rule, failure behavior, and announcement. Sites that match on all four use the shared
  control; sites that differ take only the array helper. Six copies do not by themselves prove one
  shape fits. The shared control reuses `reordered()` from
  [src/features/blueprint_forks/blueprint_fork_apply_model.ts:33](../../nsh/PROBLEMS/peptidyle-learning-engine/src/features/blueprint_forks/blueprint_fork_apply_model.ts)
  and the accessible pattern from
  [src/components/question_response_controls/ordering.tsx:60-127](../../nsh/PROBLEMS/peptidyle-learning-engine/src/components/question_response_controls/ordering.tsx)
  -- focus returns to the moved row and a live region announces the move, the only one of the six
  that works with a keyboard. Drag uses the native `draggable` attribute and pointer events. Move
  buttons show an arrow next to the text.
- Obvious follow-ons: record the comparison in
  [docs/DESIGN_DECISIONS.md](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/DESIGN_DECISIONS.md)
  as the reason for the split, then delete the probe.

### Work package: WP-D3 windowing

- Owner: `coder`.
- Touch points: `src/components/record_list/record_list_window.ts`.
- Depends on: WP-C3.
- Acceptance criteria: windowing composes with the list rather than living inside it. The split is
  **`RecordList` decides how records look; the window decides which records exist.** `RecordList`
  keeps regions, alignment, hierarchy, narrow-screen behavior, and the empty, loading, and error
  states. The window helper keeps the visible range, overscan, scroll position, and spacer height.
  The windowed library page renders its records through exactly the same row code as an ordinary
  page, so turning windowing off needs no different row markup. The hand-written window in
  [src/pages/library_browse_rows.tsx:139-177](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/library_browse_rows.tsx)
  is replaced by it, and only the library list turns it on.
- Evidence or review, when useful: `node tests/playwright/record_list_window.mjs` proves five
  things, which are the ways windowing usually breaks -- rows of differing height do not corrupt
  scrolling; keyboard focus does not vanish when its row leaves the window; scrolling to a record
  lands on that record; windowing changes neither row identity nor order; and the same row markup
  renders with windowing off.

### Work packages: WP-E1 to WP-E7 convert seven pages

- Owner: `coder`, one per page.
- Depends on: WP-D1, WP-D2, WP-D3.

| WP | Page | Pattern proved |
| --- | --- | --- |
| WP-E1 | [src/pages/question_drafts_page.tsx](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/question_drafts_page.tsx) | Simple scan list, one variant |
| WP-E2 | [src/pages/gradebook_page.tsx](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/gradebook_page.tsx) | Dense table, `fullWidth`, many columns |
| WP-E3 | [src/pages/library_browse_rows.tsx](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/library_browse_rows.tsx) | Two variants plus windowing |
| WP-E4 | [src/pages/assessment_workspace/assessment_workspace_questions_view.tsx](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/assessment_workspace/assessment_workspace_questions_view.tsx) | Reorder by keyboard and drag |
| WP-E5 | [src/pages/course_instance_page.tsx](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/course_instance_page.tsx) | Dense rows, per-row actions, theme accent |
| WP-E6 | [src/pages/student_course_landing_page.tsx](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/student_course_landing_page.tsx) | Student list, phone layout keeps action next to identity |
| WP-E7 | [src/features/profile_avatar/provided_avatar_picker.tsx](../../nsh/PROBLEMS/peptidyle-learning-engine/src/features/profile_avatar/provided_avatar_picker.tsx) | Same records, two genuinely different shapes: `gallery` and `list` |

- Why WP-E7 is worth a seventh lane: the other six all prove a denser or roomier row. The avatar
  picker proves that one set of records and one selection behavior can render as a grid of images or
  as a named list. That is a much harder test of the presentation API, and it is the consumer most
  likely to show that the API is really just a padding switch.
- Acceptance criteria, all seven: renders through `RecordList`; no page-specific escape hatch,
  meaning no `if (page === ...)` inside the component and no prop that exists to serve one caller.
  The core is expected to change during this milestone -- finding its weak spots is the point -- but
  a core change is handed to WS-CORE and applied once, never edited in two lanes.
- Acceptance criteria, specific rows: drop `assessmentId` from
  [src/pages/gradebook_page.tsx:43-64](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/gradebook_page.tsx);
  drop the raw `Assessment {id}` line and the per-row time zone from
  [src/pages/course_instance_page.tsx:355-360](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/course_instance_page.tsx);
  in
  [src/pages/question_drafts_page.tsx:221-261](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/question_drafts_page.tsx)
  show the title once, drop the `Private draft` badge the page already implies, shorten
  `Draft Question Edit Number 7` to `Edit 7`, and add a short content preview.
- Evidence or review, when useful: `tests/_temp/row_height_probe.mjs` reports assessment row height,
  target 100-150px, then is deleted. Row height is a design target, not a permanent rule.

### Work packages: WP-F1 to WP-F3 page sweep

- Owner: `coder`, one per group.
- Depends on: WP-C2, WP-C4.
- Touch points: WP-F1 `src/pages/` top level minus the proof pages; WP-F2
  `src/pages/assessment_workspace/` minus WP-E4's file; WP-F3 `src/features/` minus WP-E7's file.
- Acceptance criteria: each page uses `PageFrame` instead of hand-written `class="page"` and heading
  markup, and each date call moves to the date function. This sweeps fully, unlike list conversion,
  because a page heading is the same few slots everywhere and needs no per-page judgment. The date
  sweep is also full: 16 call sites, mechanical, and it fixes a real bug -- the hardcoded UTC
  formatter at
  [src/pages/course_instance_page.tsx:50-53](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/course_instance_page.tsx).
  The time zone is named once in the page body, never per row.
- Obvious follow-ons: in each page's **own** stylesheet, move spacing onto the 7-step scale. These
  packages do not edit `src/style.css`; instead each reports the names of rules and tokens its pages
  stopped using, and WS-CORE removes them in WP-C5. No blanket sweep and no permanent spacing check:
  the failure being fixed is inconsistent components, which the shared pieces now prevent by
  construction.

### Work package: WP-G1 four sizes per scenario

- Owner: `tester`.
- Touch points:
  [tests/playwright/screenshot_corpus/](../../nsh/PROBLEMS/peptidyle-learning-engine/tests/playwright/screenshot_corpus).
- Depends on: M5, M6.
- Acceptance criteria: each `(role, scenario)` declares all four sizes, or marks one `covered_by`
  with a reason, which the manifest schema already supports. Today student captures are 30 laptop,
  22 phone, 3 tablet, 1 square.

### Work package: WP-G2 seeded theme variety

- Owner: `tester`.
- Touch points: the same scenario files and the capture script.
- Depends on: WP-G1, because both write the manifest.
- Acceptance criteria: the course theme comes from a hash of the scenario id, so the set shows
  several palettes and two runs are still byte-identical. The count seen is reported as evidence,
  not asserted as a number to hit.

### Work package: WP-H1 write the follow-up plans

- Owner: `planner`.
- Depends on: M5.
- Acceptance criteria: five files, each starting from evidence already gathered rather than a fresh
  search.
  - `docs/active_plans/active/record_list_migration_plan.md`: every remaining list site, each with
    its page, its pattern from the WP-E table, its variants, and its region roles. **Build the list
    from a fresh inventory, not from the approximate counts in this plan.** Re-run the site sweep,
    subtract the seven converted in M5, and state the exact number. The plan is wrong if
    `converted + remaining` does not equal the inventory total, which is the check that stops a site
    being silently dropped. Notes that
    [src/pages/instructor_data_tables.css](../../nsh/PROBLEMS/peptidyle-learning-engine/src/pages/instructor_data_tables.css)
    cannot be deleted until its last user moves, so nobody removes it early.
  - `docs/active_plans/active/student_task_surface_plan.md`: remove undo and restore
    ([src/components/question_response_controls/common.tsx:441-478](../../nsh/PROBLEMS/peptidyle-learning-engine/src/components/question_response_controls/common.tsx)
    plus seven call sites and the `"restored"` state), flatten the five WeBWorK boxes to one, match
    the native question surface to it, add `...` and narrower buttons to question navigation, drop
    the word "Peptidyle" on phones, and measure the attempt vertical budget against the review's
    444px.
  - `docs/active_plans/active/ribbon_destination_completion_plan.md`: My Questions needs no backend
    work, since `QuestionSearchAuthorship::AuthoredByCurrentAccount` already exists at
    [crates/question_model/src/question_search.rs:55-61](../../nsh/PROBLEMS/peptidyle-learning-engine/crates/question_model/src/question_search.rs);
    Starred needs a `starred_by_current_account` filter, its predicate, and a grant, with an index
    only if `EXPLAIN` on seeded data shows one is needed.
  - `docs/active_plans/decisions/role_tier_two_navigation.md`: what belongs inside the student and
    sysadmin tier-2 rows. The rows already exist and hold their space, so this decides contents
    only; adding controls cannot move the page content.
  - `docs/active_plans/active/theme_completion_plan.md`: dark halves of the 15 biome palettes,
    already required by
    [docs/HUMAN_GUIDANCE.md:318-337](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/HUMAN_GUIDANCE.md).

### Work package: WP-H2 record the decisions

- Owner: `maintainer`.
- Depends on: M5.
- Acceptance criteria: the settled student tier-1 is recorded at
  [docs/HUMAN_GUIDANCE.md:630](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/HUMAN_GUIDANCE.md),
  and line 417 is reconciled with the annotation-free control;
  [docs/DESIGN_DECISIONS.md](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/DESIGN_DECISIONS.md)
  gains entries for tier-1 by role, unconditional row heights for every role, the
  reserve-but-never-fabricate rule, the split between `RecordList` and windowing, and the scan-row
  content rule;
  [docs/CODE_ARCHITECTURE.md](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/CODE_ARCHITECTURE.md)
  describes the UI composition layer.
- Notes: the scan-row rule says a scan row carries identity, status that affects a decision, one
  value or date that affects a decision, and a main action, with ids and extra metadata moving to
  the detail page. It applies to scan rows only. Tables such as gradebook and roster show the
  columns their task needs, which is legitimately more than four; capping them would recreate the
  current problem in a new form.

### Work package: WP-H3 close out

- Owner: `maintainer`.
- Depends on: WP-H1, WP-H2.
- Acceptance criteria: each SUI-01 through SUI-08 finding in
  [docs/active_plans/audits/student_ui_stability_and_density_audit_2026-09-21.md](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/active_plans/audits/student_ui_stability_and_density_audit_2026-09-21.md)
  maps to a milestone here or a named follow-up plan;
  [docs/CHANGELOG.md](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/CHANGELOG.md) is updated per
  milestone; no probe remains in `tests/_temp/`.

## Acceptance criteria and gates

- Per-patch gate: `source ./source_me.sh && ./launchers/run_fast_checks.sh`.
- Integration gate: `source ./source_me.sh && ./launchers/all_test.sh`, run once per milestone after
  its lanes merge.
- Independent review gate, when useful: `reviewer` checks M5 for escape hatches, since that is the
  one judgment a passing test cannot make. It should pay closest attention to WP-E7, where a
  presentation API that is really just a padding switch would show up first.

Permanent rules created here:

- Tier-1 control ids are the same for a role on every route.
- On every signed-in route, the breadcrumb row and the page content each start at one height per
  screen size, for all three roles. A tier-2 row with no controls still holds its space.
- The page title's left edge does not move across a student workflow.
- `RecordList` region edges line up across all rows at every screen size.
- A list with one variant shows no switcher.
- Windowing changes which records render, never how a record looks, its identity, or its order.
- Seven different pages run on `RecordList` with no page-specific escape hatch.

One-time measurements, reported to the changelog and then their probes deleted:

- Assessment rows measure 100-150px, down from about 326px.
- Spacing in touched files uses only the 7-step scale.

## Test and verification strategy

New browser checks, each a permanent gate:

```bash
node tests/playwright/ribbon_tier_one_invariance.mjs
node tests/playwright/shell_geometry_invariance.mjs
node tests/playwright/student_course_pinning.mjs
node tests/playwright/record_list_alignment.mjs
node tests/playwright/record_list_presentation.mjs
node tests/playwright/record_list_reorder.mjs
node tests/playwright/record_list_window.mjs
```

Existing runners that must keep passing:

```bash
node tests/playwright/ribbon_geometry_evidence.mjs
node tests/playwright/ribbon_responsive_evidence.mjs
node tests/test_screenshot_corpus.mjs
bash tests/e2e/e2e_screenshot_warm_loop.sh
```

Screenshot rebuild:

```bash
source ./source_me.sh && ./devel/capture_screenshots.sh
source ./source_me.sh && ./devel/capture_screenshots.sh --verify
```

Failure semantics: a failing per-patch gate blocks that work package only. A failing integration
gate blocks the milestone and every milestone that depends on it. When a permanent check fails after
merge, compare the behavior against the contract written in
[docs/DESIGN_DECISIONS.md](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/DESIGN_DECISIONS.md),
then repair whichever is wrong -- the code or the check. Usually it is the code, and the change is
reverted. Sometimes the check encoded the contract badly, which is likely during M5, where the
component is deliberately learning from real consumers; then the check is corrected and the reason
recorded. A check is a constraint that has to keep earning its place, not a fact.

Fixtures already exist and are reused rather than rebuilt:
[tests/playwright/ribbon_harness_server.mjs](../../nsh/PROBLEMS/peptidyle-learning-engine/tests/playwright/ribbon_harness_server.mjs),
`tests/support/ribbon_shell_harness.tsx`, `ribbon_responsive_harness.tsx`, and the seeded Live Demo
behind
[devel/capture_screenshots.sh](../../nsh/PROBLEMS/peptidyle-learning-engine/devel/capture_screenshots.sh).
No milestone needs a person to choose a design or approve a picture.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| `RecordList` does not fit one of the seven patterns | High. The component is wrong and ~33 later pages would inherit it | A WP-E lane needs a prop that serves only its page | WS-CORE | That is why seven different patterns are converted before the rest. The lane stops, hands the core change to WS-CORE, and it is applied once |
| Presentation variants turn out to be a padding switch | Medium. The API looks general but only changes spacing | WP-E7's `gallery` cannot be expressed without a new escape hatch | WS-CORE | WP-E7 exists to surface this early. A gallery and a list share records and selection but not geometry, so it is the hardest honest test |
| Two WP-E lanes edit the core at the same time | Medium. Lost work and conflicting shapes | Two lanes report a core gap in the same window | manager | Only WS-CORE writes the core. Lanes report, they do not patch |
| Page sweep collides with proof pages | Medium. Two owners on one file | A sweep group includes a proof page | manager | WS-PAGES explicitly excludes the seven. Stated in the ownership table |
| Tier-1 rework breaks route access | High. A role could see a link it may not use | `tierOneArea` set wrong on a route | WS-SHELL | Access is enforced by `withRouteAccessBoundary` and the server, not the Ribbon. WP-B1 checks the tab list; existing access tests stay green |
| Scope creep back into list conversion | Medium. The risk this plan was reshaped to avoid | An eighth page is added to M5 because it "looks quick" | manager | M5 is exactly the seven pages in the WP-E table, each admitted for a distinct pattern. A page that repeats a pattern already covered goes to the migration plan, however small it looks |
| Deferred work is quietly lost | Medium. The review's complaints go unanswered | M8 is skipped as paperwork | WS-DOCS | WP-H1 is an exit condition, and WP-H3 fails if any SUI finding is unmapped |
| An empty tier-2 row invites someone to fill it | Medium. Invented navigation to avoid blank space | A later page adds a tab with no workflow behind it | WS-SHELL | WP-A5 states that empty means empty and WP-H2 records it as a decision. A control needs a workflow, not a gap to occupy |

## Documentation close-out requirements

- Active plan / progress tracker: this file, plus the five follow-up plans from WP-H1.
- docs/CHANGELOG.md entry: one per milestone, including the WP-D2 reorder comparison and the WP-E
  row-height measurement, which are the learning this work produces.
- Archive / closure notes: map SUI-01 through SUI-08 to a milestone here or a follow-up plan, then
  `git mv` the audit to `docs/archive/` if every finding is closed.

## Resolved decisions

- Build the list system in-repo with SolidJS. No table or drag-and-drop dependency.
- Student tier-1 is `Courses | Coursework | Grades`, pinned to the current course.
- Unavailable Ribbon controls stay visible but lose the "Not available yet" text, which is longer
  than the label it describes.
- This plan proves the components on seven pages. The other ~33 go to a follow-up plan.
- Every signed-in role reserves the tier-2 row. Whether a row holds controls is a separate question
  from whether it takes space, and only the first is still open for students and sysadmins. An empty
  student or sysadmin row is correct today. An **instructor** page missing the tier-2 controls that
  [docs/HUMAN_GUIDANCE.md:430,490,568](../../nsh/PROBLEMS/peptidyle-learning-engine/docs/HUMAN_GUIDANCE.md)
  lists for it is a defect in that page, not a reason to reopen the design.

## Open questions and decisions needed

None of these block execution.

- Manager/subagent decision procedure:
  - Decision owner or dedicated class: `expert_coder` in WS-CORE for component shape; `planner` with
    the `ui-ux-engineer` skill for the scan-row rule in WP-H2.
  - Evidence and decision rule: WP-C1 decides the `PageFrame` slots from counted page structure.
    WP-D2 decides shared versus separate reorder from a four-axis comparison. Both write their
    result down before code depends on it.
- Non-blocking follow-up: what belongs **inside** the student and sysadmin tier-2 rows. The rows
  exist and hold their space either way, so this decides contents only and cannot destabilize the
  shell. Recorded in WP-H1.
