# Plan: Compact the authenticated Ribbon chrome

## Context

The authenticated shell stacks four horizontal bands before content:

| Band | Source | Height | Assessment |
| --- | --- | --- | --- |
| `site-header` | `src/application_shell.tsx:138-148` | 3.25rem | Holds only the `.brand` link and an invisible live region; right half empty. At <= 48rem the wordmark is hidden (`src/style.css:890-892`), so it spends full height on the "P" alone. |
| Ribbon Context Row | `src/ribbon/app_ribbon.tsx:356-396` | 3rem | Holds a second wordmark (`Peptidyle Learning Engine`) plus role, account, course/attempt labels, Sign out. |
| Ribbon Tabs | `src/ribbon/app_ribbon.tsx:399-414` | 2.5rem | Correct. |
| Ribbon Tasks | `src/ribbon/app_ribbon.tsx:415-430` | 2.5rem | Reserved on every route, including routes that can never hold a task. |

Band 1 duplicates band 2's job one row above it. Band 4 is often empty height.
[docs/UI_DESIGN_GUIDE.md](../UI_DESIGN_GUIDE.md)
line 148 already places Account and Profile "in the upper corner of the Ribbon Context Row", so the
standalone brand band has no owner.

Two design faults in this region caused live bugs and are in scope:

1. **Viewport arithmetic has no owner.** `.shell` computes
   `min-height: calc(100vh - var(--ple-header-block-size))` (`src/style.css:264-268`) and ignores
   the 8rem Ribbon, so every Ribbon page today is ~8rem taller than the viewport and scrolls for no
   reason. `--ple-course-scope-min-block-size` (`src/style.css:68-71`) repeats the formula, and
   `course_theme_scope_styles.ts:5` plus `course_theme_variables.tsx:34` each carry a
   `calc(100vh - 5rem)` fallback. Four copies, already falsified by a band added above them.
2. **A global rule reaches into a component.** Bare `nav`, `nav a`, `nav a::after`, `nav a:hover`,
   `nav a.active` (`src/style.css:202-244`) match the Ribbon's two `<nav>` rows, which then have to
   out-specify them. `.site-header` contains no `nav` at all, and `.nav-action` has zero consumers.

The product is pre-production, so both faults are repaired at the source rather than patched.

## Objectives

- Present one identity band on authenticated routes: brand, role, account, and Sign out together.
- Reserve the Task Row only where the route contract declares tasks, without leaking capability
  admission state through geometry.
- Give viewport height a single owner so adding a band cannot silently falsify a height formula.
- Return `nav` presentation to the surfaces that own it.
- Reduce chrome above content from 11.25rem to the sum of the rows actually reserved.

## Design philosophy

The governing rule for this work, to be recorded as a PLE design decision in
`docs/UI_DESIGN_GUIDE.md` and `docs/DESIGN_DECISIONS.md`:

> Use compact persistent chrome to maintain identity, context, and navigation. Combine related
> persistent controls into the smallest stable header structure that preserves orientation and
> usability. Prioritize application content over redundant branding or empty header space.

`docs/UI_DESIGN_GUIDE.md`, `src/ribbon/ribbon_contract.ts`, and `docs/ux/RIBBON_TASK_MODEL.md`
remain the normative sources for implementation. The design literature is supporting rationale
only, not a dependency: *About Face* (compact persistent navigation; branding minimized so the
persistent bar consumes less real estate), *Practical UI* (removing redundant chrome), *Designing
with the Mind in Mind* (consistent control location), *Refactoring UI* (the shell serves the
features).

Applied principles: fix the design, not the symptom (an override on `.shell` would leave the wrong
formula to be falsified again); long-term over short-term; design for adaptability. One boundary is
deliberately **not** generalized -- the Ribbon keeps three *named* rows. `docs/UI_DESIGN_GUIDE.md`
lines 180-185 define Context / Tab / Task as "the persistent Ribbon's spatial grammar", semantic
roles rather than a generic list. Turning them into an ordered row array would buy flexibility
nothing needs and weaken a contract that carries meaning. What changes is *when a named row is
reserved*.

- Evidence strategy for uncertain methods: two implementation choices are left open for the doer to
  settle from the rendered tree rather than fixed here -- the exact `grid-template-rows`
  declaration in M1 and the height-comparison tolerance in its gate. Each states its invariant and
  its decision rule; see `## Open questions and decisions needed`.

## Scope

- Merge the brand into the Ribbon Context Row; make `site-header` the no-Ribbon surface only.
- Make Task Row reservation follow declared route topology.
- Centralize viewport-height ownership in one shell frame; retire the duplicated formulas.
- Scope or delete the global `nav` rules per their real consumers.
- Trim `--ple-shell-padding-block-start`.
- Extend automated evidence and update the governing docs and changelog.

## Non-goals

- Changing Ribbon row *size* tokens, tab or task presentation, or the capability registry.
- Generalizing the three named Ribbon rows into a row list.
- An account dropdown. `accountLabel` stays static text; Sign out stays one button.
- New Ribbon destinations or Context Controls.
- Any change to route authorization or server contracts.

## Current state summary

Verified in source during planning:

- `ribbonModel()` returns `undefined` for the sign-in route, the `loading` first paint, and the
  `signedOut` / `expired` / `error` recovery surfaces (`src/app.tsx:133-152`,
  `src/auth/session_context.tsx:19-23`). Those are exactly the surfaces with no other brand.
- `taskAreasFor` reads `routeState.route.ribbon.taskGroup`, a static route-contract property
  (`src/ribbon/ribbon_contract.ts:370-396`). It builds an area for every catalog control in the
  group regardless of availability, so availability never empties `taskAreas`.
- All four declared groups (`questionLibrary`, `assignment`, `courseSetup`, `assignmentAttempt`)
  have catalog controls (`src/ribbon/ribbon_catalog.ts:239-383`). Therefore
  `taskAreas.length > 0` is already exactly `taskGroup !== undefined`, and **no new model field is
  required** -- adding one would duplicate route state.
- The only `<nav>` elements are the two Ribbon rows and `.course-card-actions`
  (`src/pages/course_instance_page.tsx:128`). `.nav-action` has no consumers.
- `/` is the role-aware Product Course index (2026-09-08 changelog entry), and `ContentError`
  already sends users there, so it is the correct brand destination for every Product Role.
- `.ple-app-ribbon` links are plain `<a href>` (`src/ribbon/app_ribbon.tsx:99-118`), and
  `tests/playwright/ribbon_m10_shell_evidence.mjs` proves the shell survives their activation.
- The `tests/playwright/ribbon_*_evidence.mjs` scripts are bare node scripts driving
  `page.setContent` over SSR fixtures. No stack, no credentials, no gate invokes them.

## Architecture boundaries and ownership

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M1, M2 / WS-height | `src/application_shell.tsx`, `src/style.css` lines 33-47 and 264-279, `course_theme_scope_styles.ts`, `course_theme_variables.tsx` | Layout ownership only; no Ribbon internals |
| M3 / WS-nav | `src/style.css` lines 202-253, `src/pages/course_instance_page.tsx` + its stylesheet | Presentation transfer only; no markup semantics |
| M4 / WS-shell | `src/application_shell.tsx` | Shell composition only |
| M5, M6 / WS-ribbon | `src/ribbon/app_ribbon.tsx`, `src/ribbon/app_ribbon.css`, `src/ribbon/ribbon_contract.ts`, `tests/support/`, `tests/design/ribbon_treatment_atlas.css` | Ribbon presentation and derived model |
| M7 / WS-ribbon | `src/style.css` line 39 | Token value only |
| M8 | whole system | Integration |
| M9 | `docs/` | Documentation |

WS-height and WS-nav both touch `src/style.css` but in disjoint line regions (33-47 / 264-279
versus 202-253). Each region has exactly one owning workstream; doers must not edit outside their
region.

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1 | One owner for viewport height | Introduce `.ple-shell-frame`; strip `min-height` from `.shell` | Frame owns document growth in both shell shapes |
| M2 | Retire the duplicated viewport formula | Delete `--ple-course-scope-min-block-size` and the `calc(100vh - 5rem)` fallbacks | One height owner, zero duplicated formulas |
| M3 | Return `nav` styling to its owner | Delete header-nav and `.nav-action` rules; move what `.course-card-actions` needs to its page | No global rule reaches into the Ribbon |
| M4 | Conditional `site-header` | Render the header only when there is no Ribbon; lift the live region out | One identity band on Ribbon routes |
| M5 | Brand into the Context Row | Replace `__product-name` with a brand link | One wordmark, at the top of the Ribbon |
| M6 | Declared-topology Task Row | Reserve the Task Row from `taskGroup`, compose the row sum once | Empty row removed without leaking admission state |
| M7 | Trim shell top padding | Lower `--ple-shell-padding-block-start` | Content sits closer under the Ribbon |
| M8 | Whole-system evidence | Build, gates, real-stack lane, screenshots | Integration proof, unattended |
| M9 | Docs and changelog | Amend the guide, decisions, task model, changelog | Contracts match the shipped behavior |

### Milestone: M1 -- One owner for viewport height

- Depends on: none
- Deliverables: `.ple-shell-frame` base class with `.ple-ribbon-shell-grid` as its variant;
  `site-header` moved inside the frame; `min-height` removed from `.shell`;
  `tests/playwright/shell_frame_height_evidence.mjs`
- Workstreams: WS-height
- Entry criteria: none
- Exit criteria: WP-1.1 and WP-1.2 accepted
- Parallel-plan ready: yes (may run concurrently with M3)

### Milestone: M2 -- Retire the duplicated viewport formula

- Depends on: M1
- Deliverables: `--ple-course-scope-min-block-size` deleted; both course-theme style strings rely on
  grid stretch; extended height evidence
- Workstreams: WS-height
- Entry criteria: M1 exit criteria met
- Exit criteria: WP-2.1 accepted; zero repository occurrences of the retired token or
  `100vh - 5rem`
- Parallel-plan ready: no -- same files and invariant as M1, single lane

### Milestone: M3 -- Return `nav` styling to its owner

- Depends on: none
- Deliverables: `src/style.css` lines 202-253 resolved; `.course-card-actions` styled by its owning
  page; `tests/playwright/ribbon_style_ownership_evidence.mjs`
- Workstreams: WS-nav
- Entry criteria: none
- Exit criteria: WP-3.1 accepted
- Parallel-plan ready: yes (disjoint `src/style.css` region from WS-height)

### Milestone: M4 -- Conditional `site-header`

- Depends on: M1
- Deliverables: header wrapped in `<Show when={ribbonModel() === undefined}>`; live region lifted to
  an unconditional sibling; SSR surface assertions
- Workstreams: WS-shell
- Entry criteria: M1 exit criteria met (the frame must exist before the header moves into it)
- Exit criteria: WP-4.1 accepted
- Parallel-plan ready: no -- single file, single lane

### Milestone: M5 -- Brand into the Context Row

- Depends on: M4
- Deliverables: brand link in `__context-identity`; `__product-name` and its atlas rule removed;
  brand sizing in the desktop, coarse-pointer, and 24rem blocks
- Workstreams: WS-ribbon
- Entry criteria: M4 exit criteria met
- Exit criteria: WP-5.1 accepted
- Parallel-plan ready: no -- shares `app_ribbon.css` with M6

### Milestone: M6 -- Declared-topology Task Row

- Depends on: M5
- Deliverables: topology invariant test; conditional tasks row-frame and
  `data-ribbon-task-row` attribute; single composed `--ple-ribbon-block-size`
- Workstreams: WS-ribbon
- Entry criteria: M5 exit criteria met
- Exit criteria: WP-6.1 and WP-6.2 accepted
- Parallel-plan ready: no -- shares `app_ribbon.css` with M5

### Milestone: M7 -- Trim shell top padding

- Depends on: M6
- Deliverables: `--ple-shell-padding-block-start` lowered; chrome-total assertion
- Workstreams: WS-ribbon
- Entry criteria: M6 exit criteria met (chrome total is only meaningful once topology is final)
- Exit criteria: WP-7.1 accepted
- Parallel-plan ready: no

### Milestone: M8 -- Whole-system evidence

- Depends on: M2, M3, M7
- Deliverables: full command sweep green; captured screenshots in `test-results/`
- Entry criteria: every code milestone accepted
- Exit criteria: integration gate met
- Parallel-plan ready: no

### Milestone: M9 -- Docs and changelog

- Depends on: M8
- Deliverables: guide, decisions, task-model, and changelog updates
- Entry criteria: M8 exit criteria met, so docs describe verified behavior
- Exit criteria: documentation close-out requirements met
- Parallel-plan ready: no

## Workstream breakdown

### Workstream: WS-height

- Goal: one owner for viewport height across both shell shapes
- Owner: `expert_coder` (M1 involves an open declaration decision), then `coder` for M2
- Work packages: WP-1.1, WP-1.2, WP-2.1
- Needs: nothing
- Provides: a frame that other milestones compose into
- Review boundary, when modifying the repository: `reviewer` checks that no `calc()` involving
  `100vh` or `--ple-header-block-size` remains outside `.site-header` sizing

### Workstream: WS-nav

- Goal: presentation owned by the surface that renders the markup
- Owner: `coder`
- Work packages: WP-3.1
- Needs: nothing
- Provides: a Ribbon whose computed styles come only from `app_ribbon.css`
- Review boundary, when modifying the repository: `reviewer` confirms `.course-card-actions`
  rendering is unchanged

### Workstream: WS-shell

- Goal: one identity band on Ribbon routes
- Owner: `coder`
- Work packages: WP-4.1
- Needs: WP-1.1
- Provides: the conditional header contract M5 depends on

### Workstream: WS-ribbon

- Goal: brand in the Context Row, Task Row reserved by declared topology, density trimmed
- Owner: `expert_coder` for WP-6.1 (contract change), `coder` otherwise
- Work packages: WP-5.1, WP-6.1, WP-6.2, WP-7.1
- Needs: WP-4.1
- Provides: the final chrome geometry
- Review boundary, when modifying the repository: `reviewer` confirms no geometry decision reads
  capability admission

## Work packages

### Work package: WP-1.1 -- Shell frame

- Owner: `expert_coder`
- Touch points: `src/application_shell.tsx:149`, `src/style.css:264-279`,
  `src/ribbon/app_ribbon.css:14-19`
- Depends on: none
- Acceptance criteria:
  - The wrapper `div` renders unconditionally with a base class and keeps the ribbon grid as a
    variant, for example
    `classList={{ "ple-shell-frame": true, "ple-ribbon-shell-grid": ribbonModel() !== undefined }}`.
  - `<header class="site-header">` renders inside that frame in both shapes.
  - `.ple-shell-frame` sets `display: grid` and `min-block-size: 100dvh`; `.ple-ribbon-shell-grid`
    adds the fixed Ribbon track and inherits the floor.
  - `.shell` no longer declares `min-height`; it relies on default grid-item stretch, so no
    percentage-height chain has to resolve.
  - `--ple-header-block-size` appears in no `calc()`; its only remaining job is sizing
    `.site-header` (`src/style.css:170`), the role `docs/UI_DESIGN_GUIDE.md` lines 53-63 assigns it.
  - **Invariant (fixed):** the frame owns document growth in both shapes. Short page produces no
    vertical scrollbar; long page grows normally; neither depends on how many bands sit above
    content.
  - **Declaration (open):** the exact `grid-template-rows` follows the rendered tree. See
    `## Open questions and decisions needed`.
- Evidence or review, when useful: `reviewer` confirms the invariant holds without any
  band-counting arithmetic
- Obvious follow-ons: WP-1.2

### Work package: WP-1.2 -- Height evidence script

- Owner: `tester`
- Touch points: new `tests/playwright/shell_frame_height_evidence.mjs`
- Depends on: WP-1.1
- Acceptance criteria:
  - Follows the existing evidence shape: stylesheet loading as in
    `ribbon_geometry_evidence.mjs:13-19`, viewport profile loop as in `:98-102`, and the repo's
    existing `near()` tolerance helper at `:48-50` rather than a new constant.
  - Covers {Ribbon shape, no-Ribbon shape} x {short content, content taller than the viewport}.
  - Short cases assert no vertical scrolling using an exact comparison
    (`documentElement.scrollHeight <= documentElement.clientHeight`), which needs no tolerance.
    Tall cases assert `scrollHeight > clientHeight`.
  - Records the pre-fix failure of the short/Ribbon case as the regression proof.
- Obvious follow-ons: WP-2.1 extends this script

### Work package: WP-2.1 -- Retire duplicated formulas

- Owner: `coder`
- Touch points: `src/style.css:68-71`, `src/features/course_appearance/course_theme_scope_styles.ts:5`,
  `src/features/course_appearance/course_theme_variables.tsx:34`
- Depends on: WP-1.2
- Acceptance criteria:
  - `--ple-course-scope-min-block-size` deleted.
  - Both course-theme style strings drop their `min-height` line entirely and rely on grid stretch,
    matching WP-1.1. Do not substitute a percentage height.
  - `grep` returns zero occurrences of `--ple-course-scope-min-block-size` and `100vh - 5rem`.
  - WP-1.2's script gains a course-themed fixture (`.course-theme-scope` wrapper) in both shapes and
    asserts the same invariant, including that the course canvas still paints edge to edge.
- Obvious follow-ons: none

### Work package: WP-3.1 -- Nav ownership

- Owner: `coder`
- Touch points: `src/style.css:202-253`, `src/pages/course_instance_page.tsx:128` and its
  stylesheet, new `tests/playwright/ribbon_style_ownership_evidence.mjs`
- Depends on: none
- Acceptance criteria:
  - `.nav-action` rules (`:246-253` and the `.nav-action` arms of `:208-240`) are deleted; planning
    found no consumer.
  - The bare `nav` rules are resolved by their real consumers: `.site-header` renders no `nav`, so
    header-nav styling is removed rather than left dormant; whatever `.course-card-actions`
    genuinely needs moves into the stylesheet owned by `course_instance_page`.
  - New evidence script SSRs a Ribbon fixture and asserts the computed values the bare rules used to
    supply (`display`, `gap`, `padding`, `color`, `font-weight`) on each Ribbon `<nav>` row and each
    `.ple-app-ribbon__link` are identical to a baseline captured with `app_ribbon.css` alone --
    proving the Ribbon never depended on them.
  - A rendered check confirms `.course-card-actions` presentation is unchanged.
- Obvious follow-ons: none

### Work package: WP-4.1 -- Conditional header, unconditional live region

- Owner: `coder`
- Touch points: `src/application_shell.tsx:138-148`, `tests/e2e/e2e_ribbon_app_component.mjs`
- Depends on: WP-1.1
- Acceptance criteria:
  - `<header class="site-header">` is wrapped in `<Show when={ribbonModel() === undefined}>`,
    keeping its `<A class="brand" href="/">`. `<A>` stays here because this path is inside the
    Router.
  - `<span class="sr-only" role="status" aria-live="polite">{signOutError()}</span>` becomes a
    direct child of the returned fragment, mounted in both shapes.
  - SSR assertions: `.site-header` absent when a Ribbon model is supplied, present when it is not,
    and the `role="status"` region present in both.
  - A synthetic `loading -> authenticated` transition case asserts the **surfaces** are correct
    before and after and that final geometry is stable. Do not assert an exact height delta
    between the two states; that would pin today's constants and fight future density work.
- Evidence or review, when useful: capture the transition once during implementation and report the
  observed movement, so the manager can confirm it reads as a resolve rather than a flash. Today the
  header stays and the Ribbon is *added* (0 -> 8rem); after this change a 3.25rem band is replaced
  by the Ribbon, which is strictly less movement. If the observed result contradicts that, stop and
  report -- reserving the Ribbon track during `loading` is the fallback, and it is a separate change.
- Obvious follow-ons: WP-5.1

### Work package: WP-5.1 -- Brand link in the Context Row

- Owner: `coder`
- Touch points: `src/ribbon/app_ribbon.tsx:363-366`, `src/ribbon/app_ribbon.css:201`, `:227-248`,
  `:413-470`, `:474-500`, `tests/design/ribbon_treatment_atlas.css`
- Depends on: WP-4.1
- Acceptance criteria:
  - `__product-name` is replaced by a brand link, with the `__product-role` badge beside it:

    ```tsx
    <a class="ple-app-ribbon__brand" href="/" aria-label="Peptidyle home">
      <span class="ple-app-ribbon__brand-mark" aria-hidden="true">P</span>
      <span class="ple-app-ribbon__brand-word">Peptidyle</span>
    </a>
    <span class="ple-app-ribbon__product-role">{props.model.context.productLabel}</span>
    ```

  - Plain `<a href>`, matching `RibbonLink` (`:99-118`); the Ribbon keeps importing nothing from
    `@solidjs/router`, so the `tests/support/` harnesses still mount it without a Router.
  - The brand carries no `data-ribbon-control` and no pending-navigation feedback: it is the
    identity control, not a catalog destination, and a control id would place it in the schema.
  - New classes are built from the existing local aliases (`--ple-ribbon-space-control`,
    `--ple-ribbon-space-tight`); the mark reuses the `.brand-mark` treatment
    (`src/style.css:190-200`). No new raw distances.
  - **Two explicit size conditions.** Desktop: the brand joins the `min-block-size: 2rem` selector
    list at `:227-248`, matching `.ple-app-ribbon__link`, so desktop density is unchanged. Coarse
    pointer: it joins the `@media (pointer: coarse)` block at `:474-500` for 2.75rem. The 44px
    figure is a touch-profile requirement only.
  - In `@media (max-width: 24rem)`, `__brand-word` collapses with the clip-based visually-hidden
    recipe already used for the sign-out label at `:426-436` -- not `display: none`, so the word
    stays in the accessibility tree.
  - `.ple-app-ribbon__product-name` and its atlas rule are removed. Planning found no test or
    contract reader.
  - `ribbon_geometry_evidence.mjs` and `ribbon_m9_responsive_evidence.mjs` pass; both now sweep the
    brand anchor (`geometry:131` uses `document.querySelectorAll("a, button")`; `m9:76` uses
    `row.querySelectorAll("a,button")` and asserts `height >= 44` on touch profiles).
  - A new assertion proves exactly one element in the Ribbon carries the product wordmark.
- Obvious follow-ons: WP-6.1

### Work package: WP-6.1 -- Topology invariant

- Owner: `expert_coder`
- Touch points: `tests/test_ribbon_contract.mjs`
- Depends on: WP-5.1
- Acceptance criteria:
  - A test asserts the equivalence the geometry will rely on: for every declared route,
    `taskAreas.length > 0` if and only if `route.ribbon.taskGroup !== undefined`. This holds today
    because every declared group has at least one catalog control
    (`src/ribbon/ribbon_catalog.ts:239-383`), and the test stops that from silently ceasing to hold.
  - A second case asserts `taskAreas.length` is **unchanged** when every task control is made
    unavailable -- the property that keeps geometry from reporting capability admission.
  - **No new `RibbonModel` field.** `taskAreas` is already the derived state; a `taskRowReserved`
    boolean would duplicate route topology and could drift through hand-built fixtures.
- Obvious follow-ons: WP-6.2

### Work package: WP-6.2 -- Conditional Task Row

- Owner: `coder`
- Touch points: `src/ribbon/app_ribbon.tsx:319-340` and `:415-430`, `src/ribbon/app_ribbon.css:3-11`,
  `:57-59`, `:399-409`, `:474-500`, `tests/playwright/ribbon_geometry_evidence.mjs`
- Depends on: WP-6.1
- **Invariant this rests on, stated for future readers.** `taskAreas.length > 0` is a reading of
  *declared route topology*, not of capability admission, because of two verified properties of
  `taskAreasFor` (`src/ribbon/ribbon_contract.ts:370-396`):
  1. It returns `[]` immediately when `route.ribbon.taskGroup` is `undefined`, and otherwise pushes
     one area per catalog control in that group **regardless of each control's availability** --
     `modelForControl` sets availability on the control but never omits it. So making every task
     control unavailable leaves `taskAreas.length` unchanged.
  2. Every declared group -- `questionLibrary`, `assignment`, `courseSetup`, `assignmentAttempt` --
     has at least one control in `RIBBON_TASK_CATALOG` (`src/ribbon/ribbon_catalog.ts:239-383`), so
     a declared group always yields a non-empty `taskAreas`.

  Together these make `taskAreas.length > 0` exactly equivalent to `taskGroup !== undefined`. That
  is why geometry may read it without leaking admission state, and why no `taskRowReserved` field is
  added. WP-6.1 pins both properties as tests so the equivalence cannot lapse silently -- if a
  future group is declared with no catalog control, WP-6.1 fails rather than the Ribbon quietly
  losing a row.
- Acceptance criteria:
  - The tasks `row-frame` renders only when `props.model.taskAreas.length > 0`. The Ribbon root
    carries `data-ribbon-task-row="reserved" | "absent"` so CSS and tests can select on topology.
  - A comment at the render site records the invariant above in one sentence, citing
    `ribbon_contract.ts:370-396`, so the next reader does not mistake it for an admission check.
  - `taskScrollport`, `taskOverflow`, and the task `createEffect` are guarded against the absent row.
  - **The row sum is composed once.** Introduce `--ple-ribbon-reserved-task-size`, defaulting to
    `var(--ple-ribbon-task-block-size)` and set to `0px` under `[data-ribbon-task-row="absent"]`.
    `--ple-ribbon-block-size` is then declared **one time** as
    `calc(context + tab + reserved-task)`, and `grid-template-rows` uses the same three values.
    `@media (max-width: 40rem)` and `@media (pointer: coarse)` change only the individual row
    tokens; neither restates the sum. This preserves the ownership principle M1 and M2 establish.
  - `.ple-ribbon-shell-grid` reads `--ple-ribbon-block-size`, so the shell track follows with no
    extra rule.
  - `tests/support/ribbon_model_fixtures.ts` gains a task-less fixture if it lacks one.
  - `ribbon_geometry_evidence.mjs` runs its full profile matrix against both topologies, asserting
    for each that the rendered Ribbon height and the shell's first grid track both equal
    **the sum of the row tokens currently reserved** -- computed from the custom properties, not
    compared to a literal rem value.
- Obvious follow-ons: WP-7.1

### Work package: WP-7.1 -- Shell padding

- Owner: `coder`
- Touch points: `src/style.css:39`
- Depends on: WP-6.2
- Acceptance criteria:
  - `--ple-shell-padding-block-start`: `clamp(0.75rem, 1.6vw, 1.25rem)` ->
    `clamp(0.5rem, 0.9vw, 0.75rem)`.
  - Computed `padding-block-start` of `#main-content` asserted at the three
    `tests/playwright/ui_corpus_manifest.ts` profiles.
  - Chrome above content is asserted as *the sum of reserved row tokens plus this padding*, derived
    from the custom properties. Do not hard-code the resulting rem totals; row sizes are
    independently owned and this plan does not change them.
- Obvious follow-ons: none

## Acceptance criteria and gates

- Per-patch gate: the work package's own acceptance criteria, plus `./check_codebase.sh` and
  `source source_me.sh && pytest tests/`. Every evidence script the package names must be invoked
  explicitly, because none are wired into a gate.
- Integration gate (M8), all scriptable and unattended:

  ```bash
  source source_me.sh && node pipeline/build.mjs --skip-wasm
  node tests/playwright/shell_frame_height_evidence.mjs
  node tests/playwright/ribbon_style_ownership_evidence.mjs
  node tests/playwright/ribbon_geometry_evidence.mjs
  node tests/playwright/ribbon_m9_responsive_evidence.mjs
  node tests/playwright/ribbon_m9b_density_evidence.mjs
  node tests/playwright/ribbon_m10_shell_evidence.mjs
  node tests/playwright/ribbon_m11_deferred_content_evidence.mjs
  node tests/e2e/e2e_ribbon_app_component.mjs
  node tests/e2e/e2e_ribbon_production_styles.mjs
  ./check_codebase.sh
  source source_me.sh && pytest tests/
  ./devel/run_playwright_tests.sh
  ./devel/capture_screenshots.sh
  ```

- Independent review gate: `reviewer` on the assembled diff, specifically checking that no geometry
  decision reads capability admission, that no `calc()` reintroduces band-counting, and that the
  Ribbon's computed styles depend only on `app_ribbon.css`.

## Test and verification strategy

Per `docs/PYTEST_STYLE.md`, none of this belongs in the pytest fast lane: it needs a browser and
real layout. Evidence stays in the existing `tests/playwright/*_evidence.mjs` lane (bare node,
`page.setContent` over SSR fixtures) and `tests/e2e/*.mjs`, matching
`docs/E2E_TESTS.md`. Per `docs/PLAYWRIGHT_TEST_STYLE.md`, assertions are behavioral and computed
style, never pixel diffs, and each new script documents its selector contract with source
`file:line` citations in a header comment.

Measured success conditions:

1. One identity band on Ribbon routes: brand left, account and Sign out right, exactly one wordmark,
   no band above the Ribbon.
2. Ribbon row heights equal their tokens; Ribbon total and shell track equal the sum of reserved row
   tokens, in both topologies.
3. Task Row present exactly when the route declares `taskGroup`, and unaffected by making every task
   control unavailable.
4. No vertical scrolling on a short page; normal growth on a long one; both shell shapes; course
   theme applied and not.
5. Narrow and touch: brand word visually hidden but present in the accessibility tree, brand target
   >= 44px on touch, desktop density unchanged.
6. Focus order: skip link -> brand -> account region -> Sign out -> Tabs -> Tasks (when reserved) ->
   `#main-content`.
7. Sign-in, loading, and recovery surfaces keep their branded header.
8. Sign-out failure still announces through the live region.
9. Ribbon computed styles unchanged after the global `nav` rules are removed.

`run_live_demo.sh start --headless` provisions the stack without opening a browser, so the
real-stack lane and screenshot capture run unattended. Screenshots land in `test-results/`;
acceptance rests on the measured assertions above, not on anyone viewing them.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Grid track shape wrong for the real child list | Layout breaks in one shape | WP-1.2 short-page case fails in only one shape | `expert_coder` | Invariant is fixed, declaration is derived from the rendered tree; see open questions |
| Header-to-Ribbon swap reads as a flash | Perceived regression on cold load | Captured transition in WP-4.1 shows large movement | `coder` | Stop and report; reserving the Ribbon track during `loading` is a separate change |
| Brand anchor fails the 44px touch sweep | `ribbon_m9` gate fails | Coarse-pointer profile | `coder` | Brand joins the existing coarse-pointer selector list, which already produces 2.75rem |
| Geometry starts reflecting admission | Capability state leaks through layout | Someone switches on visible-control counts | `reviewer` | WP-6.1's unavailability case asserts `taskAreas` is unchanged |
| `.course-card-actions` regresses when `nav` rules move | Visible defect on the course page | WP-3.1 rendered check | `coder` | Transfer only what that surface uses; verify rendering before deleting |
| `--ple-ribbon-block-size` re-duplicated in a media query | The fault this plan removes returns | Review of `app_ribbon.css` | `reviewer` | Sum declared once; media queries change row tokens only |

## Rollout and release checklist

- [ ] M1-M7 accepted with their per-patch gates
- [ ] Integration gate green end to end
- [ ] Screenshots captured to `test-results/` and attached to `docs/UI_DESIGN_REVIEW.md`
- [ ] Independent `reviewer` pass on the assembled diff
- [ ] M9 documentation merged before the change is described as complete

## Documentation close-out requirements

- Active plan / progress tracker: copy this plan to
  `docs/active_plans/active/ribbon_chrome_compaction.md` at execution start (snake_case per
  `docs/REPO_STYLE.md`), and `git mv` it to `docs/archive/` at closure.
- `docs/UI_DESIGN_GUIDE.md`:
  - Navigation section near line 148 -- the brand is the Context Row's leading identity control; the
    standalone band is the no-Ribbon surface only; state the density principle as a PLE decision,
    with the design literature cited as supporting rationale, not as normative.
  - Lines 180-185 -- rows are reserved by **declared topology**; admission, loading, labels, and
    errors still may not change geometry.
  - Lines 53-63 -- record that viewport height is owned by the shell frame and that
    `--ple-header-block-size` sizes `.site-header` only.
- `docs/DESIGN_DECISIONS.md` lines 1050-1082 -- amend the geometry consequence to the
  declared-topology rule, keeping the truthfulness clause. Add entries for the merged identity row
  and for height ownership, each with Decision / Why / Consequence / Owner.
- `docs/ux/RIBBON_TASK_MODEL.md` -- the "intentionally remains reserved" sentence (lines 59-60) and
  the reflow row of the heuristic ledger (line 221).
- `docs/CHANGELOG.md` entry: `### Behavior or Interface Changes` for the merged band and the Task
  Row; `### Decisions and Failures` recording that the always-reserved empty row and the
  per-element viewport `calc()` were set aside, and why.
- Archive / closure notes: `docs/UI_DESIGN_REVIEW.md` receives the captured screenshots as the
  record required by `docs/UI_DESIGN_GUIDE.md` lines 261-267.

## Patch plan and reporting format

- Patch 1: WP-1.1, WP-1.2 (frame and height evidence)
- Patch 2: WP-2.1 (retire duplicated formulas)
- Patch 3: WP-3.1 (nav ownership) -- dispatchable concurrently with Patch 1
- Patch 4: WP-4.1 (conditional header)
- Patch 5: WP-5.1 (brand link)
- Patch 6: WP-6.1, WP-6.2 (topology invariant and conditional row)
- Patch 7: WP-7.1 (padding)
- Patch 8: integration gate and screenshot capture
- Patch 9: remaining repository-required work -- documentation close-out and `docs/CHANGELOG.md`

Each patch reports: files touched, gate commands run, their output, and any acceptance criterion
not met.

## Open questions and decisions needed

Both are non-blocking; each has a decision rule the doer can execute without waiting.

- Manager/subagent decision procedure -- exact `grid-template-rows` for `.ple-shell-frame`:
  - Decision owner or dedicated class: `expert_coder` on WP-1.1
  - Evidence and decision rule: enumerate the wrapper's actual children in both shapes from the
    rendered tree before writing the declaration. The plan fixes the invariant (frame owns document
    growth; `.shell` declares no height), not the track list. If a persistent child beyond the
    header, Ribbon, and content participates in layout, add its track rather than forcing the
    two- or three-track shape suggested here.
- Manager/subagent decision procedure -- height comparison method in WP-1.2:
  - Decision owner or dedicated class: `tester` on WP-1.2
  - Evidence and decision rule: prefer the exact, tolerance-free comparison
    (`documentElement.scrollHeight <= documentElement.clientHeight`) for the no-scroll condition. Use
    a tolerance only if measurement shows sub-pixel variation makes that unstable, and then reuse
    the repo's existing `near()` helper (`ribbon_geometry_evidence.mjs:48-50`) rather than
    introducing a new constant.
- Non-blocking follow-up: the Context Row's own 3rem may be revisitable once the brand, role, and
  account share it, but `docs/UI_DESIGN_GUIDE.md` lines 87-92 require concrete evidence (clipping,
  missed target, lost reachability) for a density change. Gather that from the M8 screenshots
  rather than assuming it here.
