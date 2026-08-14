# UI design-system and workflow implementation

## Status

Accepted on 2026-08-13. This workstream supersedes older visual-acceptance claims where the accepted
geometry or contrast treatment conflicts with current human guidance. The implemented review and
evidence are recorded in `docs/UI_DESIGN_REVIEW.md`. It does not reopen grading, answer secrecy,
tenancy, retention, or teaching semantics.

## Product contract

- Canonical instructor viewport: 1280 by 800 CSS pixels.
- Representative student viewport: 800 by 1280 CSS pixels, plus one narrow-phone overflow guard.
- Standard course presentation: palette-faithful, ordinary text from 5.5:1 through 8.25:1, with no
  systematic push toward near-black-on-white contrast.
- Increased contrast: optional, account-backed, theme-preserving, and applied only to presentation.
- Identifiers: no UUID, sequential question number, or question version in visible or announced
  content, application navigation URLs, or copyable application links. Typed public references and
  the Crockford Base32 Question ID resolve inside existing authorization boundaries.
- Shared design system: deliberate typography, spacing, hierarchy, controls, focus, navigation,
  content width, grouping, empty states, and responsive behavior.

## Dependency-ordered tasks

1. Record guidance, route and preference contracts, this workstream, and the durable design guide.
2. Add typed public references for courses, assignments, runs, and workspaces; implement the
   Question ID specification without exposing hidden history; return these references in the
   required browser projections and resolve them under the current actor before loading internal IDs.
3. Return roster-owned learner display names in gradebook summaries and remove UUID presentation.
4. Add the account-backed standard/increased presentation preference and one accessible account
   control; forced-colors remains automatic and does not rewrite the stored preference.
5. Refactor shared tokens and shell composition, then restore palette-derived course surfaces.
6. Refactor instructor pages, beginning with assignment organization, catalog width, gradebook,
   roster, course management, workspace, and library.
7. Refactor student course, assignment, run, response, feedback, summary, and empty states.
8. Update durable behavior tests, capture the canonical visual set, measure both contrast modes,
   inspect the renders, and publish the final interface review.

## Acceptance gates

- At 1280 by 800, the assignment editor visibly fits four selected problems, policy controls, and a
  save action without trapping the catalog in a narrow half-column.
- All instructor task pages use the available width deliberately; no page has an unexplained empty
  half while its main browse/edit task is compressed.
- Student pages reflow without horizontal overflow and keep prompt and response in one readable
  visual sequence.
- Standard themes are visibly distinguishable beyond an accent line. Every ordinary text pair meets
  5.5:1; increased contrast strengthens text, focus, and boundaries without changing the hue family.
- Keyboard focus is obvious on the focused element and does not become the dominant page graphic.
- Visible/announced DOM, address bars, and application links contain no UUIDs. Background API and
  asset traffic may retain internal IDs. Question identity uses `AAA-BBBB`, never `P-n-vn`.
- Gradebook rows use recognizable learner names. Run history remains lazy and reads as a designed
  drill-down rather than raw record output.
- Behavior-focused Rust, TypeScript, and Playwright gates pass. Current screenshots at 1280 by 800,
  800 by 1280, and narrow-phone width are inspected and linked from the final audit.

## Acceptance evidence

- `npm run build`: pass.
- `./check_codebase.sh`: pass, including strict TypeScript, ESLint, Prettier, and offline Node
  behavior tests.
- `npx playwright test --workers=4`: pass for the enabled behavior-focused browser suite; live and
  visual acceptance cases remain explicit.
- `cargo test -p server_core --lib`: pass for the server-owned behavior suite; disposable-service
  cases remain explicit.
- `python3 -m pytest -q tests/test_ui_walkthrough_runner.py`: pass for the offline runner contract.
- Palette refinement: the strict catalog/type checks and 13 focused appearance/scope browser tests
  pass; the regenerated rendered metrics place ordinary standard-theme text at 5.50:1 through
  7.92:1 and the reviewed contact sheet shows all 15 applied palette systems without banner-color
  contamination.
- The generated canonical renders and measured theme report were reviewed and are summarized in
  `docs/UI_DESIGN_REVIEW.md`.

## Security invariants

Public references are not secrets and grant no authority. Resolution uses authenticated session
identity, tenant context, current membership, and existing object ownership checks. Student browser
responses remain answer-free; grading stays server-only; course content, assignment behavior,
retention, and audit contracts do not change.
