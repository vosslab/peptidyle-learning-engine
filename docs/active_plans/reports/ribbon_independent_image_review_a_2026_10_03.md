# Independent Ribbon image review A

## Verdict

The Ribbon itself is close to finished and intentional. Its two navigation tiers now read as
different, connected surfaces rather than a collection of miniature folder controls. Selection is
immediately apparent in both tiers in the reviewed Light and Dark theme samples. The broader
Instructor interface is less complete: several real pages use their generous canvas as unstructured
empty space, so the carefully resolved Ribbon lands above content that can still look provisional.

The first follow-up should be content composition, especially sparse and empty states, rather than
another Ribbon geometry change. Do not replace the Tier 1 shoulder treatment or Tier 2 open join.

## Evidence examined

I reviewed `instructor/stacked-screenshot.webp`, the 66-header crop stack (1280 by 11748 pixels).
I inspected the source 1280 by 800 WebP theme samples in a full-size cross-theme comparison and
opened representative originals at native resolution. The theme set covered both modes for Arctic,
Beach, Coral Reef, Desert, Forest, Grass, Magma, Ocean, Salt Marsh, Sea Floor, Swamp, Tropical,
Tundra, Underground, and Wetland (30 files named `theme_sample-<theme>-light.webp` and
`theme_sample-<theme>-dark.webp`).

I opened these additional full Instructor pages at native resolution:

- `courses-active-course_list.webp`
- `courses-students-course_roster_active.webp`
- `courses-gradebook-gradebook.webp`
- `questions-browse-library_browse.webp`
- `courses-assessments-assignment_questions_draft.webp`
- `courses-blueprint-blueprint_detail.webp`

For context only, I also inspected the eight `page_*_320.png` and `page_*_393.png` images under
`active_plans/reports/ribbon_state_contrast_2026_10_03/`. They are a controlled component harness,
not real-app corpus screenshots, so they support narrow-state observations only.

## Observed findings

### Ribbon

- Every theme sample has a continuous upper colored band. The selected Tier 1 item is lifted by a
  compact, pale or dark tab face with visibly curved shoulders; the unselected Tier 1 labels sit
  directly in the band. `theme_sample-arctic-light.webp`, `theme_sample-desert-dark.webp`, and
  `theme_sample-tropical-dark.webp` make this relationship especially easy to see.
- Tier 2 is visually square and directly abuts the content. Its selected item carries the content
  surface across the row and has an open lower edge. Inactive Tier 2 choices remain on the darker
  or more muted row surface. This is clear in all 30 theme samples and in the course roster.
- Tier 1 selection remains legible because the active tab has a changed surface as well as the
  stronger label treatment. Tier 2 selection remains legible because its active surface continues
  into the page. Dark examples retain the same distinction without becoming color-inverted copies
  of the light examples.
- Header proportions are stable across the crop stack: the combined branding/Tier 1 row, Tier 2,
  and breadcrumb/content edge retain a compact, repeatable rhythm. The narrow harness retains the
  same tier relationships and makes the selected tab visible at 320 and 393 pixels.

### Whole pages

- Theme character reaches the page canvas, actions, and hierarchy. Arctic is cool and airy, Desert
  is warm, Tropical uses the green/magenta pairing, and their Dark modes feel designed rather than
  simply dimmed. The course assessment samples establish this consistently.
- Text is readable at the rendered desktop size in the reviewed samples. Labels, primary actions,
  tables, and the two Ribbon tiers remain distinguishable by wording, iconography, and surface;
  this review does not make a measured accessibility-conformance claim.
- The roster and Gradebook use concise tables with predictable headers and comfortable scan lines.
  `courses-students-course_roster_active.webp` is the strongest full-page composition reviewed:
  title, roster table, supporting controls, and Course context make a coherent path.
- `courses-assessments-assignment_questions_draft.webp` also has a good instructional hierarchy.
  The page title, status, save guidance, field, and ordered entry are easy to follow without a
  card-heavy dashboard look.

## Judgments and priorities

### 1. Sparse pages still look under-composed

`courses-blueprint-blueprint_detail.webp` has a large field of unstructured peach surface. The
return link sits far to the right of the title, classification editing is detached farther down,
and the main action is isolated near the lower left. The page has the correct information but no
strong visual grouping to explain its order. It looks like a work-in-progress layout more than a
finished Course detail view.

`questions-browse-library_browse.webp` has the same issue in a different form. The narrow left
filter column, wide empty-results panel, and detached paging controls create three weakly related
islands. The empty-state message is visually strong, but it floats in a broad blank panel. This
needs a content-layout decision for empty Library states, not a new decorative card system.

Direction: establish a compact reading column or grid for these pages, align related actions with
their heading or section, and let an empty-state panel occupy the useful content width without
turning most of the page into unused canvas. Preserve the current flat, dense visual language.

### 2. The Ribbon is stronger than the content edge beneath it

The Tier 2-to-content join succeeds, but the content canvas on several pages begins as a single
large, nearly unbounded surface. On `courses-active-course_list.webp` and
`courses-blueprint-blueprint_detail.webp`, that makes the crisp Ribbon edge carry more visual
weight than the task area. The result is top-heavy: the navigation feels complete while the page
below it has little silhouette or rhythm.

Direction: give sections a modest structural rhythm through alignment, measured maximum widths,
and intentional whitespace. The answer is not more borders or rounded cards; the best reviewed
pages already show that row structure and spacing can do this quietly.

### 3. A few light themes verge on an undifferentiated pastel wash

The Light Grass and Tropical samples use a very similar light green content surface over a green
Ribbon. Their selected tiers still read, but the page's major regions have less depth than Arctic
or Desert. The effect is most visible when the assessment list contains little content.

Direction: retain the theme hues but tune the relative lightness or saturation of the continuous
Ribbon, its lower Tier 2 strip, and the content surface so they form a clearer three-surface
sequence. This is a surface-relationship refinement, not a request for gradients, shadows, or
more saturated color.

## Final calls

- **Looks finished and intentional?** The Ribbon: yes, with the caveat that full-page polish is
  incomplete on sparse and empty-state pages. The whole Instructor composition: not yet.
- **Tier 1 selection at a glance in Light and Dark?** Yes. It reads as the open tab, and the
  unselected choices recede while still appearing enabled.
- **Tier 2 selection at a glance in Light and Dark?** Yes. The square active surface visibly
  continues into the page, while inactive choices remain in the row.
- **Important limitation:** this is a static rendered-image review. It does not evaluate hover,
  focus, keyboard behavior, animation, zoom, real narrow-device scrolling, or measured contrast.
