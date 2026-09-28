# Plan: Whole-interface themes with independent Light/Dark mode

## Context

"The fact that all 15 screenshots look nearly identical is not a constraint to preserve. It is the
problem this plan exists to solve." The 15 `docs/screenshots/instructor/theme_sample-*.png`
captures are the **before** state: one near-white interface with colored pinstripes.

Why it looks that way:

- `course_theme_registry.ts:55-73` derives every surface as a small tint of the theme color mixed
  into white, so all themes share the same off-white page.
- Theme identity survives only in a thin gradient rail and a card edge
  (`course_theme_variables.tsx:38-49`, `course_theme_scope_styles.ts:3-23`).
- Theme tokens sit on an inner div (`course_theme_variables.tsx:127`), so `html`, `body`, and page
  edges never take the theme. There is no dark mode (`src/style.css:4`), and many CSS files
  hard-code light colors.

The user's model (overrides `docs/HUMAN_GUIDANCE.md` where they differ):

- 15 actual visual themes, not near-white pages with different pinstripes.
- An Instructor's personal theme applies to global Instructor pages.
- Each Course has its own independently selected theme, also controlled by the Instructor.
- Light/Dark is completely independent of theme and belongs to the viewing user.
- The browser preference is only the default until the user explicitly chooses Light or Dark;
  clearing the choice returns control to the browser. No System/Auto option or mode rules.
- Every theme has Light and Dark forms: 15 x 2 = 30 looks. The default theme is `grass`.
- No inheritance, override, or theme-strength machinery.

PLE is pre-production, so this plan fixes foundational contracts directly, with no legacy support.
This plan gets the themes in place; palette refinement is later work.

## Objectives

- Each of the 15 themes visibly changes the whole interface in both modes.
- Every look is defined by five palette colors, with all other theme colors derived by one shared
  rule.
- One component owns theme and display mode for the whole document.
- Course pages, global Instructor pages, and the viewer's Light/Dark preference follow the rules
  below.
- Text and controls stay readable in all 30 looks.

## Design philosophy

- KISS: the smallest design that satisfies the model. Add mechanisms, state, and tests only when
  implementation demonstrates a need.
- Fix the design, not the symptom: replace the derive-toward-white registry, inner-div scope, and
  rail with real palettes applied at the document root. Tuning today's mix percentages or thickening
  the rail is rejected.
- Pre-production: the theme set now serves both Course and personal themes, so investigate renaming
  the Course-only `CourseTheme` contract to `Theme`. Do it now if the blast radius is reasonable,
  because after production it would need a data migration.
- Perfect is the enemy of good: define the required end state here and let repository
  investigation during implementation choose table names, API shapes, and exact color formulas.
- Evidence strategy for uncertain methods: pilot four themes on real pages before authoring the
  rest, and let the pilot settle the shared color derivation.

## Scope

- One theme contract that serves Course and personal themes across database, Rust, TypeScript, and
  docs.
- Five palette colors in Light and Dark for each of the 15 themes, plus one shared derivation.
- One document-level owner for theme and display mode.
- Theme colors painted across the page background, Ribbon, panels, cards, and selection.
- Light-only hard-coded colors made to work in Dark.
- Stored Light/Dark preference for every Account and personal theme for each Instructor, with a
  Ribbon toggle and a Profile appearance section.
- Light and Dark screenshots, tests, and authority docs.

## Non-goals

- Keep the 15 theme IDs, display names, chooser order, and Course theme selection behavior.
- Keep User Role, Assessment Type, and status colors global; give them dark values only.
- Build no System/Auto option, schedules, or automatic-mode rules.
- Build no per-look color exceptions, theme-strength setting, per-Course mode, or custom colors.
- Leave `html[data-contrast="increased"]` (it has no setter) untouched; note it in `docs/TODO.md`.

## Current state summary

| Area | Today | File |
| --- | --- | --- |
| Theme contract | Course-only names (`CourseTheme`, `ple_data.course_theme`) | `crates/question_model/src/course_appearance.rs:18-75`, `schemas/base_schema/20_tables/course_instance.sql:6-14,51-52` |
| Palette | 3 light-only anchors, surfaces derived toward white, ad-hoc per-theme overrides | `src/features/course_appearance/course_theme_registry.ts` |
| Theme application | inline tokens on a shell div; Course page re-applies theme to its own section | `course_theme_variables.tsx:123-136`, `course_instance_page.tsx:630` |
| Unsaved preview | Course Appearance preview override, cleared on save, error, and Course change | `course_theme_variables.tsx:84-105`, `course_theme_context.ts` |
| Page base and Ribbon | neutral `:root` colors; white highlights | `src/style.css:4-21,91-106`, `src/ribbon/app_ribbon.css` |
| Light-only literals | about 45 color declarations; `--ple-warning` used but never defined | `style.css`, `app_ribbon.css`, `user_role.css`, `question_renderer_styles.ts`, others |
| Preferences | Account settings hold only time zone; session carries identity and role | `crates/server/src/profile_settings.rs`, `crates/server/src/auth.rs:240-256` |
| Tests and screenshots | light-anchor contrast test; screenshot runtime light only | `tests/test_course_theme_scope.mjs`, `tests/playwright/screenshot_corpus/runtime.ts:117,126` |

## Current theme system failures

This is an audit of what is wrong today. Completion is judged by the final behavior, not by
proving each item separately; most disappear with the architecture change.

- All themes render the same near-white page; identity is a pinstripe rail.
- Theme never reaches `html`/`body`; two competing theme scopes exist.
- The Ribbon ignores theme color.
- Palettes are light-only anchors with ad-hoc overrides.
- No dark mode; many light-only literal colors; status and User Role colors are light-only.
- Non-Course pages have no theme; no personal theme or stored Light/Dark preference exists.
- The theme contract is named for Courses only, though personal themes will use the same set.
- Tests and screenshots cover Light only.
- Authority docs describe the old model (`docs/HUMAN_GUIDANCE.md:337-338,491`, the superseded
  `docs/active_plans/active/theme_completion_plan.md`).

## Behavior contracts

### Theme ownership

This is the central invariant:

| Page | Active theme |
| --- | --- |
| Inside a Course (Course pages, Assessment Attempts, history, Course Appearance) | that Course's theme (its unsaved preview while choosing) |
| Global pages, signed-in Instructor | the Instructor's personal theme |
| Global pages, everyone else or signed out | default theme (`grass`) |

Light/Dark belongs to the viewer and applies to whichever theme is active.

### Display mode

There are two display modes, Light and Dark. The viewer's stored preference is Light, Dark, or
unset.

| Preference | Displayed mode |
| --- | --- |
| unset (new Account, cleared, or signed out) | the browser preference, followed live |
| Light or Dark | that mode, everywhere |

- A two-state Light/Dark toggle next to Profile sets the preference.
- A "Follow browser setting" action on Profile clears it; it appears only while a preference is set.
- Changing mode never changes a theme; changing a theme never changes mode.

### Palette model

A look is one theme in one mode. Each look has exactly five palette colors:

| Palette color | Paints |
| --- | --- |
| Canvas | Page background everywhere, including behind the Ribbon. Visibly hued, not off-white. |
| Surface | Ribbon top row, panels, cards, tables, inputs. Differs from Canvas. |
| Secondary | Ribbon task row, section headers, table headers, quiet fills. |
| Accent | Primary buttons, links, focus ring, active indicators. |
| Highlight | Selected tab or row, current item, badges. Distinct from Accent. |

Text, muted text, hover, borders, focus, and text-on-color come from one shared derivation applied
to every look. The pilot settles that derivation. A look that fails readability gets different
source colors or an improved shared derivation, never a per-look exception.

### Document ownership

One component owns appearance and writes the active look to the `<html>` element, including its
`color-scheme`. The root is the only place that themes page edges, scrollbars, and native controls.
It replaces the inner-div scope and the Course page's own theming. The Course Appearance unsaved
preview goes through this owner. Theme chooser cards may preview a theme on their own card only.

## Milestone plan

Record the fast-gate baseline before stage 1. After that, every change must add no new failures
versus that baseline; unrelated pre-existing failures are recorded, not fixed here.

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1 | Shared theme model | Rename decision, five-color registry, shared derivation, rule functions | One model |
| M2 | Document-level application | One owner on `<html>`; scopes and rail removed | Theme reaches the whole page |
| M3 | Surface migration | Paint surfaces; fix light-only colors | Themed and Dark-safe CSS |
| M4 | Palettes | Pilot four themes, then the other eleven | 30 real looks |
| M5 | User preferences | Stored Light/Dark preference, personal theme, controls | The model works end to end |
| M6 | Screenshots and docs | Light and Dark captures, authority docs | Evidence and docs match |

### Milestone: M1 shared theme model

- Depends on: baseline recorded.
- Deliverables:
  - Investigate the blast radius of renaming `CourseTheme` to `Theme` across database, Rust,
    generated TypeScript, TypeScript, and tests. If reasonable, rename it as a mechanical change
    with no behavior change and move global appearance code out of `src/features/course_appearance/`.
    Otherwise keep the names and record why in `docs/DESIGN_DECISIONS.md`.
  - Registry holds five palette colors in Light and Dark per theme. Interim values are transcribed
    from today's colors so the app runs unchanged.
  - One shared derivation from the five colors to the remaining theme tokens.
  - Pure functions for the Theme ownership and Display mode rules, with one test per rule row.
- Exit criteria: rename decision recorded (and, if renamed, no old names remain); the app looks
  unchanged; rule tests pass.
- Parallel-plan ready: no; a rename would touch every theme reference. The literal-color work in M3
  may start alongside.

### Milestone: M2 document-level application

- Depends on: M1.
- Deliverables: one appearance owner writing to `<html>`, following the browser mode while the
  preference is unset; `CourseThemeVariables`, `.course-theme-scope` rules, the Course page section
  theming, and the rail removed; the Course Appearance preview kept.
- Exit criteria: the theme covers the whole page; the preview returns to the saved theme after
  leaving the page or switching Course; the Ribbon stays mounted across navigation.
- Parallel-plan ready: no; one owner for the shell.

### Milestone: M3 surface migration

- Depends on: M2 for painting surfaces; the literal-color work depends only on the baseline.
- Deliverables:
  - Canvas, Surface, Secondary, Accent, and Highlight painted as in the Palette model; neutral
    `:root` color literals removed.
  - Light-only literals fixed: white mixes and white highlights use theme tokens; white text on
    colored fills uses text-on-color tokens; `--ple-warning` defined and token fallbacks removed;
    status and User Role colors get dark values. Modal scrims and image hotspot outlines stay.
  - Question content checked in Dark on real Questions and made readable with theme tokens.
- Exit criteria: a Light Course workspace is visibly themed edge to edge; in Dark the sampled pages
  show no white boxes or unreadable text.
- Parallel-plan ready: yes; the literal-color work is independent of M1 and M2.

### Milestone: M4 palettes

- Depends on: M3.
- Deliverables:
  - Pilot Forest, Arctic, Magma, and Desert in both modes on the Course workspace and Assessment
    Question Editor; settle the shared derivation.
  - Author the other 11 themes in both modes, each consistent with its biome name.
  - A permanent contrast test over all 30 looks: normal text and text-on-color at least 4.5:1;
    focus and control boundaries at least 3:1.
- Exit criteria: screenshots show real themes, not white pages with pinstripes, in both modes; the
  contrast test passes.
- Parallel-plan ready: no for the pilot; the remaining themes may be split between two authors.

### Milestone: M5 user preferences

- Depends on: M1 for storage; M2 for the controls.
- Deliverables:
  - Every Account stores a nullable Light/Dark preference; each Instructor stores a personal theme
    (default `grass`). The app receives both at startup. Follow the existing Account time zone
    pattern for storage and endpoints.
  - Two-state Light/Dark toggle next to the Profile avatar.
  - Profile appearance section: the Instructor personal theme chooser (shared with Course
    Appearance) and the "Follow browser setting" action.
  - A persistence test for the preference and the personal theme.
  - Check a fresh page load for a visibly wrong theme or mode; fix it if real.
- Exit criteria: the Theme ownership and Display mode rules hold in the browser.
- Parallel-plan ready: yes; storage can proceed beside M2-M4, and the controls follow M2.

### Milestone: M6 screenshots and docs

- Depends on: M4, M5.
- Deliverables:
  - Screenshot corpus captures Light and Dark: the 15 themes in both modes on the Course
    workspace, a personal theme on the filtered Question Library, and before/after comparison.
  - Authority docs updated (see Documentation close-out requirements).
- Exit criteria: `./devel/capture_screenshots.sh --verify` passes; `./launchers/all_test.sh` shows
  no new failures versus baseline.
- Parallel-plan ready: no; one shared screenshot corpus.

## Acceptance criteria and gates

The plan is done when the final behavior holds:

- The 15 themes look like real themes in both modes, judged by screenshot comparison with the
  before set.
- The Theme ownership and Display mode rules hold in the browser.
- The contrast test passes for all 30 looks.
- The fast gate and full suite show no new failures versus the baseline.

## Test and verification strategy

- Permanent tests: the rule functions, the all-looks contrast test, the preference and personal
  theme persistence test, and the existing theme tests updated for the rename.
- Everything else is normal browser testing and screenshot comparison. Add further checks only if
  an actual problem appears.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Themed surfaces feel loud | Conflicts with the compact, restrained design | pilot screenshots | palette author | Lower Canvas saturation in the shared derivation before M4's remaining themes |
| Question content unreadable in Dark | Students cannot read Questions | M3 Dark check | M3 owner | Fix with theme tokens; investigate further only if a renderer resists |
| A rename misses a reference | Build or runtime failure | compile or fast gate | M1 owner | Any rename lands alone and first |
| Wrong theme or mode visible on load | Jarring color switch | M5 load check | M5 owner | Fix the cause found |

## Documentation close-out requirements

- `docs/HUMAN_GUIDANCE.md` "Role colors and themes" states the user's model; `:337-338` and `:491`
  no longer conflict.
- `docs/BIOME_THEME_PALETTES.md` describes five palette colors, 30 looks, the shared derivation,
  and default `grass`.
- `docs/DESIGN_DECISIONS.md`: theme and mode independence; nullable preference instead of a System
  mode (keeps Light/Dark from growing into a rules engine); `<html>` ownership; the theme contract
  rename decision.
- `docs/API_CONTRACTS.md`, `docs/TERMINOLOGY_CONTRACT.md` (Theme, Personal Theme, Display Mode,
  Display Mode Preference, palette color, look), and `docs/SCHEMA_TABLES.md` match the result.
- `docs/active_plans/active/theme_completion_plan.md` is marked superseded; `docs/TODO.md` notes the
  unset increased-contrast attribute; docs/CHANGELOG.md has an entry per milestone.

## Open questions and decisions needed

- None blocking. Names, schemas, API shapes, and color formulas come from repository
  investigation during implementation.
