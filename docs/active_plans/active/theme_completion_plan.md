# Plan: Complete dark halves of current Course Themes

## Context

Human Guidance requires coordinated light and dark Course Theme appearances. The current runtime has
a closed 15-ID `CourseTheme` enum and a browser registry with three light-only anchors that derive
the rest of its tokens. `docs/BIOME_THEME_PALETTES.md` is a proposed 25-theme specification, not
runtime authority. This plan completes dark halves for the current 15 IDs only: tundra, forest,
desert, grass, arctic, ocean, tropical, coral-reef, swamp, underground, salt-marsh, wetland,
sea-floor, magma, and beach.

The current fast gate is red. Stabilization is a prerequisite, not part of palette work.

## Objectives

- Give every current persisted Course Theme four coordinated dark palette roles.
- Select light or dark tokens without changing a Course's durable theme ID.
- Validate actual rendered text, controls, borders, focus, selected, disabled, and validation states.

## Design philosophy

Keep the durable 15-ID data contract and make one coordinated dark presentation per existing ID.
Replacing the runtime with the proposed 25-theme registry is rejected here because it is a separate
architecture decision with unresolved migration and registry-cutover work.

## Scope

- Define Canvas, Surface, Secondary, and Accent dark values for each current runtime ID.
- Replace light-only derived theme projection with an explicit two-mode palette projection.
- Select mode from display preference without persisting mode in Course data.
- Test the current 15 themes side by side in light and dark modes at narrow and wide viewports.

## Non-goals

- Do not admit the proposed 25-theme registry or migrate existing theme IDs.
- Do not add arbitrary user-selected colors or per-Course palette editing.
- Do not silently fall back for an unknown theme ID.
- Do not begin implementation while stabilization remains unresolved.

## Current state summary

`CourseTheme::ALL` owns 15 serialized IDs across Rust, generated TypeScript, and persistence.
`course_theme_registry.ts` currently derives Surface and semantic tokens from three light anchors,
and the app declares light color scheme behavior. Human Guidance and the palette specification
require role-specific rendered contrast rather than a single raw-color ratio.

## Approach

1. After stabilization, document four dark roles for each of the 15 current IDs, retaining ID and
   chooser order. Review duplicates side by side before implementation.
2. Make the browser registry hold explicit light/dark role sets and project semantic tokens through
   one shared deterministic function. Rust/SQL/TypeScript IDs remain unchanged.
3. Select display mode at rendering time. Course data retains only its theme ID; an unknown ID still
   fails closed.
4. Measure every rendered foreground/background pair by semantic use. Adjust a palette, not an
   unrelated component, when a theme fails.
5. Capture representative Course and assessment surfaces at narrow and wide viewports in both modes.

## Critical files

- `crates/question_model/src/course_appearance.rs`.
- The Course Theme SQL constraint and generated `CourseTheme` contract.
- `src/features/course_appearance/course_theme_registry.ts` and Course Appearance rendering.
- `docs/HUMAN_GUIDANCE.md` and `docs/BIOME_THEME_PALETTES.md`.

## Acceptance criteria and gates

- Every one of the 15 current IDs has explicit dark Canvas, Surface, Secondary, and Accent values.
- A Course persists the same ID across light/dark selection; mode selection does not write Course data.
- Normal text meets 4.5:1 on its rendered background, meaningful non-text UI meets 3:1, and 5.5:1
  is recorded as the preferred normal-text target where applicable.
- Decorative Secondary never becomes text/icon foreground without a separately tested semantic token.
- Unknown IDs fail closed at every boundary; no default palette masks contract drift.
- Fast checks, contrast evidence, rendered browser checks, screenshot replay, and `git diff --check` pass.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Dark palette is visually redundant | Course identity is lost | Side-by-side review cannot distinguish two themes | theme owner | Revise the failing current palette before acceptance |
| Raw palette color is used as text | Accessibility failure | Component bypasses semantic tokens | UI owner | Test browser-computed colors in each semantic state |
| Mode becomes persisted Course state | Unnecessary data migration | Schema/API change proposes a mode field | architecture owner | Reject it; persist only the existing ID |

## Verification

Do not start until the fast gate is green. For each theme and mode, measure rendered colors in actual
text, links, buttons, selected/disabled/validation, focus, and boundary contexts; inspect narrow and
wide screenshots; and replay screenshot capture verification. Any failing role-specific contrast or
visual differentiation blocks that palette, not the other fourteen.
