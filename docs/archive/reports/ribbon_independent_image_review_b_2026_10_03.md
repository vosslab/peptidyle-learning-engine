# Independent image review B: Instructor Ribbon

Date: 2026-10-03

## Overall verdict

**Needs one more focused visual-polish pass before calling the Ribbon finished.** The current
interface is coherent, unusually calm for a teaching application, and the Theme system has a
strongly consistent visual language. At normal desktop width, the primary and secondary
selections can be found quickly in both Light and Dark mode. The major unresolved issue is
silhouette and surface hierarchy: the active tabs still read more like separate rounded controls
laid on bars than compact file-folder tabs that emerge from one continuous Ribbon and meet the
content surface. This is visible across themes, so it is a system-level composition issue rather
than a palette exception.

## Direct observations

- Every inspected full-size theme sample has the same geometry: a colored Tier 1 bar, a separate
  Tier 2 bar, a selected `Courses` tab, selected `My Active Courses` tab, then a broad content
  surface. The 15 palettes retain the same proportions and type scale in Light and Dark mode.
- In the theme samples, the selected Tier 1 `Courses` tab has a strongly rounded, button-like
  outline at its left and right edges. Its fill is light against the bar in Light mode and dark
  against the bar in Dark mode. The selected Tier 2 item is rectangular and clearer as a local
  selection, but its small light/dark surface appears visually detached from the much larger page
  surface below it.
- The first Ribbon row combines the Peptidyle brand and Tier 1 choices; it is followed by Tier 2
  and the body surface. These three horizontal regions are especially conspicuous in the Theme
  samples and on
  `questions-browse-library_browse.webp`, which adds a long Tier 2 row.
- In the course list, Blueprint detail, Question detail, and Question editor screenshots, broad
  empty areas make the header bands dominate the first viewport. In the roster, gradebook,
  assessment editor, and Question editor, denser content gives the Ribbon more proportionate
  weight.
- Text is visibly readable in the inspected screenshots at their supplied resolution. I did not
  make a numeric accessibility or contrast determination from compressed screenshots.
- At 320 and 393 pixels in the supplied state-contrast harness, selected `Courses` still stands
  out and the Tier 2 choice is identifiable. The clipped label fragments and lone chevrons at
  the horizontal edges show a crowded horizontal navigation treatment. These are controlled
  harness images, not production full-app screenshots.

## Criterion-specific findings

### Ribbon continuity and selected-tab silhouette - fail for the stated design intent

The colored bars themselves are continuous, but the selected tabs do not yet convincingly read as
tabs grown from them. In `theme_sample-arctic-light.webp`, `theme_sample-magma-dark.webp`, and
`theme_sample-tropical-dark.webp`, the Tier 1 selection looks like a compact rounded button or a
miniature folder placed over the strip. The fully softened corners and the visible contrast at
both sides are the strongest cues. The intended curved shoulders would work better if only the
shoulders above the bar are expressed and the lower edge visually continues into the active
surface.

Tier 2 appears closer to the intended square-corner treatment, particularly in
`courses-students-course_roster_active.webp`. Still, its chosen item often reads to me as a pale
rectangle resting on the darker bar, rather than a tab that visibly joins the page. This is an
aesthetic reading of the visible color and edge treatment, not a claim that the implementation
lacks a shared-color join or square corners. A clearer shared surface or unbroken edge down into
the content region would make the intended join more legible; changing the color alone will not
solve that silhouette problem.

### Selection at a glance - pass, with a narrow-width reservation

At desktop width, the selected Tier 1 destination is immediate: `Courses`, `Questions`, and
`Assessments` are consistently isolated by a change of fill and position. Tier 2 selection is
also clear where the screenshots show it, including `My Active Courses`, `Browse Question
Library`, `My Draft Questions`, and `Search Question Library`. In the Gradebook screenshot,
`Assessments` is the selected Tier 1 choice; `Gradebook` appears in the breadcrumb, not as the
selected Tier 2 choice. The selected state stays legible in the inspected Light and Dark theme
samples. The unselected choices recede but retain full-strength type and icon treatment, which
avoids a disabled appearance.

At 320 pixels, selection remains detectable but the overall navigational rhythm does not look
finished: clipped `Questions` text, detached chevrons, and partial Tier 2 labels turn the Ribbon
into a viewport crop rather than a deliberately compact mobile header. Treat this as a
production-component concern to verify in a live narrow route, not proof of a production defect
from the harness alone.

### Hierarchy, rhythm, and visual weight - needs refinement

The page typography is strong: headings, all-caps eyebrow labels, links, controls, and tables
form a stable teaching-workflow hierarchy. The roster and gradebook are particularly successful:
the course context, table header, rows, and actions are calm and easy to scan. The assessment
question editor is dense without looking card-heavy, and the question editor preserves a useful
left authoring/right preview distinction.

The Ribbon has more visual weight than those page systems. Its two rows and their contrast with
the body surface consume enough vertical attention that the page title begins as a secondary
event. The effect is most apparent
in `courses-active-course_list.webp`, `courses-blueprint-blueprint_detail.webp`, and
`questions-search-published_question_detail.webp`, where content is sparse or left aligned. The
course-list page also leaves a large unstructured right and lower field; that amplifies, rather
than causes, the header's dominance. This is a whole-composition concern, not a request to add
decorative cards or fill the space.

### Theme character and consistency - pass

The Theme samples are disciplined. Arctic/ocean/sea-floor remain cool, Desert/Magma remain warm,
Grass/Forest/Wetland remain biological, and Tundra/Underground stay subdued without requiring a
different layout. Dark modes retain an appropriately quieter character while preserving the
same information order. Coral Reef and Tropical give the controls a little more personality;
the common geometry prevents that personality from turning into visual noise.

## Prioritized design directions

1. **Fix the active-tab construction as one Ribbon system.** Keep Tier 1's curved upper
   shoulders, remove the impression of a complete rounded rectangle, and make its lower boundary
   resolve into the selected surface. Keep Tier 2's square geometry and strengthen its perceived
   selected-to-content continuity. Validate this first in a cool, warm, and low-chroma theme
   in both modes.
2. **Reduce the header's visual interruption before changing page content.** Preserve the
   information architecture, but tune the contrast and/or vertical weight of the successive
   bands so a page title and its primary teaching task lead the first viewport.
3. **Make the narrow Ribbon an intentional compact navigation composition.** Ensure every
   visible label and chevron has a purpose and a complete boundary at 320 and 393 pixels. A
   fresh real-app narrow capture should decide acceptance after the change.

## Evidence inventory and limitations

I directly viewed `stacked-screenshot.webp` (66 header crops); all 30 full-size
`theme_sample-{arctic,beach,coral-reef,desert,forest,grass,magma,ocean,salt-marsh,sea-floor,swamp,tropical,tundra,underground,wetland}-{light,dark}.webp` images; and these nine full Instructor
pages: `courses-active-course_list.webp`, `courses-students-course_roster_active.webp`,
`courses-gradebook-gradebook.webp`, `questions-browse-library_browse.webp`,
`courses-assessments-assignment_creation.webp`,
`courses-assessments-assignment_questions_draft.webp`,
`courses-blueprint-blueprint_detail.webp`, `questions-drafts-saved_editor.webp`, and
`questions-search-published_question_detail.webp`. I also viewed four optional controlled
narrow harness images: `page_grass_{light,dark}_320.png` and
`page_tundra_{light,dark}_393.png`.

This was static visual review only. It does not establish hover, keyboard focus, animations,
actual responsive overflow behavior, runtime rendering differences, or measured color contrast.
