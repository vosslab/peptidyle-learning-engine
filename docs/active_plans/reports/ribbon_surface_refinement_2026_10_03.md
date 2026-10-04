# Ribbon and content surface refinement

The structural model stays fixed. This pass addresses the visual execution:
traced outlines made the tab joins look cut out, tight label spacing crowded
the shoulders, and strong horizontal rules competed with the content.

The selected Tier 1 now uses broader, antialiased shoulders and a borderless
silhouette. Both bars use a restrained accent tint to separate their surfaces
from selection. Tier 2 remains square, with a continuous opening into content.
Label spacing is more generous while reserved row heights stay stable. The
selected label and breadcrumbs use quieter weights. The face and adjoining
surface repaint together on theme changes, avoiding a brief mismatched seam.

The page has a narrow surrounding rail and quiet side edges. Its top stays
open to Tier 2; the breadcrumb no longer adds a horizontal rule. Small bottom
corners finish the outer sheet. Phone rail and inner gutter retain 16px for
content. Course section rules are neutral and subordinate to headings.

Real-page review also exposed an empty media track in text-only records. The
shared record grid now creates that track only when an image exists, so text
and facts use the row. Facts wrap between label and value rather than squeezing
short labels into broken words. Primary record actions now use the existing
shared primary-link/button treatment, replacing a local color-only treatment
that left Student actions looking like highlighted text. These fixes use the
existing layout and action-style owners.

## Visual review

The [local comparison](ribbon_surface_refinement_2026_10_03/index.html) switches
between before and after, desktop and phone, and light and dark. These renders
use the current application shell, synthetic Course data, and production themes.
They are local review artifacts, not a publication or a replacement corpus.

- [Course light](ribbon_surface_refinement_2026_10_03/final_1280_light.png)
- [Course dark](ribbon_surface_refinement_2026_10_03/final_1280_dark.png)
- [Roster light](ribbon_surface_refinement_2026_10_03/roster_1280_light.png)
- [Roster phone dark](ribbon_surface_refinement_2026_10_03/roster_393_dark.png)

The [curved-tab reference](https://www.geeksforgeeks.org/javascript/how-to-make-curved-active-tab-in-navigation-menu-using-html-css-javascript/)
informs the negative-space transition. Its proportions, palette, and code are
not copied. Review compares the whole composition as well as the joins.

Final local-demo examples after the content fixes:

- [Course dark](ribbon_surface_refinement_2026_10_03/live_course_dark.webp)
- [Roster light](ribbon_surface_refinement_2026_10_03/live_roster_light.webp)
- [Question library](ribbon_surface_refinement_2026_10_03/live_library_light.webp)
- [Student phone](ribbon_surface_refinement_2026_10_03/live_student_phone.webp)

## Validation

- All-theme density and contrast, responsive behavior, reduced motion,
  forced colors, and current-source routed-shell checks passed.
- Component style-ownership check passed.
- TypeScript, focused lint, formatting, and documentation checks passed.
- All eight views in the local before/after comparison loaded successfully.
- Four real local-demo scenarios produced 53 staged captures, covering Course,
  roster, Question library, Student landings, and all Course themes.
- Existing RecordList behavior checks passed. A one-time geometry check in
  `tests/_temp/ribbon_surface_review.mjs` verifies text-only width at 320, 393,
  768, and 1280px. No new permanent geometry or aesthetic test is retained.
- Final staged captures include the shared record fixes. No publishing or Git work.

Tier geometry and paint stay in the separate tier stylesheets. The new
`ribbon_content_surface.css` owns the surrounding rail and page edge.
Navigation, lifecycle, and breadcrumb composition are unchanged.

## Completion ownership

| Owner | Success condition | Evidence |
| --- | --- | --- |
| Tier styles | Smooth curved/square joins, coherent selected surfaces | Fresh light/dark, narrow/wide renders; theme and motion checks |
| Content surface | Intentional outer edge with an open tab-to-page join | Course and roster renders; routed-shell geometry checks |
| Shared record grid | Text-only rows use their width; image rows retain media | Existing RecordList checks, temporary geometry evidence, and real captures |

Visual acceptance is the rendered composition. The automated checks protect
behavior and accessibility; they do not certify aesthetic quality.
