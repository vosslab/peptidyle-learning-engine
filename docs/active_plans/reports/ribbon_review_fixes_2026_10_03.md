# Instructor review fixes

## Applied decisions

The specialist reviews support a focused refinement of the existing navigation
model. The visible targets are smoother Tier 1 shoulders, a quieter content
edge, understandable narrow-screen overflow, and a Blueprint page organized
around course work.

- `src/ribbon/ribbon_tier_one.css`: broader outward feet and smaller upper
  corners keep the cap compact while strengthening its connection to the bar.
  Label boxes, target sizes, row heights, and theme surfaces remain stable.
- `src/ribbon/ribbon_content_surface.css`: reduce the desktop rail to 8px and
  remove the duplicate side shadow. Tier 2 retains square corners and its open
  content join. Phone content retains its combined 16px gutter.
- `src/ribbon/app_ribbon.tsx` and `app_ribbon.css`: replace decorative arrows
  with native, named backward/forward scroll buttons. Each moves its own row
  with overlap and preserves selection. Unavailable directions are hidden and
  disabled. Opaque row surfaces keep clipped text out of the arrow area.
- `src/features/blueprint_course/blueprint_course_detail_workspace.tsx`: place
  primary actions and the reusable course structure before stewardship and
  classification in DOM order. Put the collection return link in PageFrame's
  existing action region. This improves both reading and keyboard order.
- `src/components/course_classification.css`: size the metadata edit action to
  its label, aligned with the summary, instead of stretching it across the grid.

The existing small CSS owners remain independently adjustable. The previous
cross-theme active/inactive value step remains intact.

## Recommendations assessed

The Gradebook and Course Assessment editor deliberately map to the Assessments
Tier 1 area in `src/route_contract.ts`. Neither is inherently a descendant of
the existing global Due Soon collection or Assessment Templates collection.
Selecting either one would misstate the current page. This pass therefore
does not add a destination or force a false selected parent. A new workflow
destination would require an information-architecture decision grounded in
actual navigation tasks.

Breadcrumb names and their horizontal access remain intact. A static narrow
capture did not justify replacing authored Course names. Empty-state pages
retain honest whitespace; the concrete Blueprint ordering problem is fixed
without adding decorative cards to fill the viewport.

## Evidence and validation

- Before/after harness renders cover all 15 themes in both modes, with Course
  pages at 320, 393, and 1280px in Grass and Tundra. All six final theme sheets
  were visually inspected for tab silhouette, state contrast, and content joins.
- Existing density evidence passes surface and text contrast, focus, forced
  colors, reduced motion, and stable geometry checks.
- Existing responsive evidence now exercises pointer-forward and keyboard-back
  scrolling while checking that selection remains unchanged. Superseded
  assertions that required decorative, noninteractive arrows were replaced.
- Style ownership, TypeScript, and focused ESLint checks pass.
- Live Instructor checks at 320 and 393px pass pointer/keyboard row scrolling,
  unchanged selection and URL, and document-width containment in light/dark
  renders. Blueprint actions and structure appear before stewardship; its
  editor opens and returns successfully.
- Regenerated all 246 real-app screenshots directly in `docs/screenshots`.
  The capture pipeline completed successfully, including corpus metadata and
  atlas generation. Rebuilt the Instructor crop stack after all captures:
  66 Instructor images, including 30 theme/mode samples.
- Temporary rendering work lives in `tests/_temp/`; no permanent aesthetic
  snapshot or geometry-value test was added.

Representative controlled evidence:

- [Interactive before/after comparison](ribbon_review_fixes_2026_10_03/index.html)
- [Before light](ribbon_review_fixes_2026_10_03/before/final_sheet_light_0.png)
- [After light](ribbon_review_fixes_2026_10_03/after/final_sheet_light_0.png)
- [After dark](ribbon_review_fixes_2026_10_03/after/final_sheet_dark_0.png)
- [Narrow Course page](ribbon_review_fixes_2026_10_03/after/page_grass_light_320.png)
- [Live narrow Course](ribbon_review_fixes_2026_10_03/after/live_course_320_dark.png)
- [Live narrow Blueprint](ribbon_review_fixes_2026_10_03/after/live_blueprint_393_dark.png)
- [Blueprint before](ribbon_review_fixes_2026_10_03/before/courses-blueprint-blueprint_detail.webp)
- [Blueprint after](ribbon_review_fixes_2026_10_03/after/courses-blueprint-blueprint_detail.webp)
- [Instructor crop stack](../../screenshots/instructor/stacked-screenshot.webp)

The source route used the CSS creative skill's layout, theming, and cascade
guidance. Its local Grid guide explains content-sized start alignment instead
of default grid stretching. Native row scrolling uses
[Element.scrollBy](https://developer.mozilla.org/en-US/docs/Web/API/Element/scrollBy)
with instant behavior, so the new controls add no motion dependency.
