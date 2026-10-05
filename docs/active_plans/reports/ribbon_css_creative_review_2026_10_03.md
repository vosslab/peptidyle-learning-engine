# CSS creative review: Instructor Ribbon

Date: 2026-10-03

## Verdict

The Ribbon is a coherent, theme-capable component with stable desktop geometry and readable
content hierarchy. It is not visually finished against its own file-tab contract. The major
craft problem is perceptual: Tier 1's selected silhouette reads as a small rounded control laid
over the Ribbon more often than as a tab rising out of it. Tier 2 already has square corners and
a shared canvas in the cascade; it sometimes still looks like a rectangle placed on the lower
bar because the visible value step and side rails compete with the intended open edge.

This review is based on static renders and source inspection. It makes no claim about motion,
focus, pointer interaction, browser computed styles, or the actual narrow-route overflow behavior.

## Visual contract

- **Audience and states:** an Instructor navigating courses, questions, and assessments; empty,
  dense, authoring, and reading pages; Light and Dark theme modes; 320, 393, and 1280 pixel
  widths.
- **Hierarchy:** the brand and Tier 1 share one row. Tier 1 is the continuous colored Ribbon,
  with a compact selected file tab whose curved shoulders grow from that plane. Tier 2 is a
  quieter context row. Its selected tab has square corners and reads as the open upper edge of
  the content sheet.
- **Color roles:** resting Tier 1 and Tier 2 must remain distinct from their selected surfaces in
  every theme/mode; inactive destinations should recede while retaining ordinary interactive
  weight. Text readability is a separate measured-contrast question.
- **Geometry:** selection must not move neighbor labels; the content edge and page gutter remain
  stable; narrow navigation exposes an intentional overflow state rather than accidental crops.

## Evidence reviewed

I directly inspected `stacked-screenshot.webp` (66 header crops), all 30 full-size
`theme_sample-{arctic,beach,coral-reef,desert,forest,grass,magma,ocean,salt-marsh,sea-floor,swamp,tropical,tundra,underground,wetland}-{light,dark}.webp` images, and nine full Instructor
pages: course list, roster, Gradebook, Question Library browse, Assessment creation, Assessment
Question Editor, Blueprint detail, Draft Question editor, and Published Question detail. I also
inspected `page_grass_{light,dark}_320.png` and `page_tundra_{light,dark}_393.png` in the
controlled state-contrast harness. Gradebook selects Tier 1 `Assessments`; it has no selected
Tier 2 `Gradebook` tab.

Representative evidence links:

- [theme_sample-arctic-light.webp](../../screenshots/instructor/theme_sample-arctic-light.webp)
- [courses-students-course_roster_active.webp](../../screenshots/instructor/courses-students-course_roster_active.webp)
- [courses-blueprint-blueprint_detail.webp](../../screenshots/instructor/courses-blueprint-blueprint_detail.webp)
- [state contrast review](ribbon_state_contrast_2026_10_03/index.html)

### Strengths to preserve

- The 15 themes hold a consistent typographic scale, row sizing, and selection placement. Arctic,
  Ocean, and Sea Floor stay cool; Desert and Magma stay warm; biological palettes retain their
  character. Dark mode is a designed role reversal rather than a crude inversion.
- Roster, Gradebook, and Assessment Question Editor show good page rhythm: practical data and
  actions carry more weight than decorative containers. The editorial type hierarchy is clear.
- Tier 1 and Tier 2 choices are immediately distinguishable at desktop width in the supplied
  theme samples. Inactive labels recede without looking disabled.
- Existing rules reserve the same borders and box dimensions during selection. That protects the
  stable geometry requirement and should remain non-negotiable.

## Craft findings

### 1. Tier 1 shoulder treatment has the wrong apparent mass

**Observed:** Across Arctic Light, Magma Dark, Tropical Dark, and the crop stack, the active
`Courses`, `Questions`, or `Assessments` face has a compact, high-contrast rounded cap. The
outward feet are too small to establish a continuous shoulder-to-bar transition at screenshot
scale. Against a high-value step, the cap reads as a complete mini folder or pill.

**Judgment:** The file-tab idea is present but its silhouette is too self-contained. This is the
largest visual issue because it repeats on every route and theme.

**Owners:** `src/ribbon/ribbon_tier_one.css`:
`.ple-app-ribbon__top-bar`,
`.ple-app-ribbon__tabs .ple-app-ribbon__link[aria-current="page"]::after`, and `::before`;
tokens `--ple-ribbon-folder-block-size`, `--ple-ribbon-folder-foot-size`, and
`--ple-ribbon-folder-radius`.

**Direction:** Test one shallower cap with wider, quieter shoulder feet before changing colors:
reduce the cap's apparent corner dominance, make the lower apron visually read as one plane, and
avoid adding outlines, shadow, gradients, or another state ornament. Keep the hit target and the
reserved label geometry unchanged.

### 2. Tier 2's implemented join needs a stronger visual reading

**Observed:** Source confirms the intended geometry: `--ple-ribbon-task-radius: 0`, selected
`background-color: var(--ple-theme-canvas)`, a matching selected bottom border, and a content
sheet that deliberately omits a top rule. In the roster and Question Library screenshots, this
is visibly more tab-like than Tier 1. In several theme samples it nevertheless appears as a pale
or dark rectangle sitting on Tier 2, with the rail and difference between row and sheet drawing
attention to its sides.

**Judgment:** This is a perceived-quality issue, not an assertion that square corners or a shared
canvas are absent. The implementation has the right primitives; it needs a small value/edge
rebalance to make the connection read without explanation.

**Owners:** `src/ribbon/ribbon_tier_two.css`:
`.ple-app-ribbon__tasks`, `.ple-app-ribbon__tasks .ple-app-ribbon__link`, and
`[aria-current="page"]`; `src/ribbon/ribbon_content_surface.css`:
`.ple-ribbon-shell-grid` and `.ple-ribbon-shell-grid > .shell`.

**Direction:** Compare a reduced rail prominence and a selected task face that visually carries
into the sheet at its lower edge. Preserve square corners, zero top rule, and the same selected
canvas token. Do not turn Tier 2 into a second rounded-folder system.

### 3. Empty pages give the Ribbon disproportionate first-viewport weight

**Observed:** `courses-active-course_list.webp`, Blueprint detail, and Published Question detail
leave broad right/lower fields. Their page title starts below two saturated navigation rows, so
the header reads as the primary event. Roster and gradebook dilute this effect with substantive
content; no card treatment is needed to compensate.

**Owners:** first inspect `src/ribbon/app_ribbon.css` tokens
`--ple-ribbon-top-block-size`, `--ple-ribbon-task-block-size`,
`--ple-ribbon-space-row`, and `--ple-ribbon-space-inline`; then page-owned layout rules. Do not
solve a page-composition problem by changing content structure inside `AppRibbon`.

**Direction:** First tune only the Ribbon's contrast hierarchy and vertical rhythm after the
tab silhouettes are corrected. A modest reduction in inter-row visual interruption is preferable
to more content boxes or decorative fill.

The user's required active/inactive surface distinction remains a constraint on this refinement:
selected states must stay obvious at a glance in both tiers across every theme and mode.

### 4. Narrow evidence looks technically bounded but visually crowded

**Observed:** The 320/393 harness shows complete selected `Courses` but clipped neighboring
labels, partial Tier 2 names, and strong edge chevrons. The evidence is a controlled component
harness, so it cannot prove a production mobile defect.

**Owners:** `src/ribbon/app_ribbon.css` narrow rules for
`.ple-app-ribbon__top-bar`, `.ple-app-ribbon__tabs-frame`, and
`.ple-app-ribbon__overflow-cue`; `src/ribbon/app_ribbon_density.css` 24rem label rules; the
Tier 1 40rem overflow rules in `src/ribbon/ribbon_tier_one.css`.

**Direction:** Retain row-local horizontal scrolling and complete selected labels. Make the cues
subordinate enough that they read as navigation affordances rather than detached controls. Judge
the final treatment in a real 320/393 route with a long current label and an empty/dense page.

## Cascade inventory

| Surface | Current ownership | Craft implication |
| --- | --- | --- |
| Base grid, shared spacing, overflow cues, focus, narrow density | `app_ribbon.css`, then `app_ribbon_density.css` | Keep shared targets and overflow mechanics outside tier-shape changes. |
| Theme role surfaces | `ribbon_surfaces.css` | Retain semantic tokens; palette changes require measured contrast work. |
| Tier 1 cap, apron, and shoulders | `ribbon_tier_one.css` | Primary owner for the selected-file-tab correction. |
| Tier 2 square task and selected canvas | `ribbon_tier_two.css` | Primary owner for perceived sheet continuity. |
| Page-sheet side rail and lower corners | `ribbon_content_surface.css` | Adjust only in concert with Tier 2 visual proof. |
| Import order | `app_ribbon.tsx` imports base, surfaces, Tier 1, Tier 2, density, content surface | Existing ownership is legible; avoid specificity escalation or a new override file. |

## Success conditions and validation

1. In three representative themes (Arctic, Magma, Tundra), both modes show one continuous Tier 1
   bar with a compact selected tab that reads as a file tab, not a rounded button.
2. Tier 2 remains square and visibly opens onto the canvas in the course, questions, and
   assessments contexts; selection changes do not shift neighbors or sheet edges.
3. Empty, dense, authoring, and long-title pages retain content as the first-viewport focal point.
4. Fresh real-app screenshots at 320, 393, and 1280 pixels test overflow cues, long labels, and
   both schemes. Inspect computed values for the selected task canvas, radius, rail, and reduced
   motion before declaring the visual change final.
5. Delegate a numerical cross-theme surface/text contrast audit to
   `color-accessibility-expert` if any token values change.

## Craft rationale and limits

The focused source route was visual craft, theming, responsive layout, and cascade ownership.
The local CSS-in-Depth cascade chapter supports keeping this correction in the existing
tier-specific owners rather than accumulating overrides. The local Dark Mode guide's design
section supports treating dark surfaces as intentional semantic roles. Those sources guided the
small-owner recommendation; screenshots remain the aesthetic oracle. No browser, computed-style,
motion, or numerical color test ran in this review.
