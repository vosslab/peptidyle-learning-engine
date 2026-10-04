# Ribbon selection surface contrast

The active tab now separates from its siblings through a consistent value step
in both tiers. Selected tabs use the theme canvas. The continuous inactive bar
mixes 24% theme ink into that canvas, so it recedes without reducing label opacity
or making available navigation look disabled.

## Ownership and visual review

- `src/ribbon/ribbon_surfaces.css` owns the shared surface relationship and its
  adjustable mixing parameter. Each tier's existing stylesheet owns geometry.
- The selected Tier 1 face and apron share one surface; the selected Tier 2
  remains square and continuous with the content. Theme changes repaint connected
  surfaces together. Row heights and control positions remain unchanged.
- The Ribbon focus ring uses theme ink so it stays distinct on both new surfaces.
  Content focus colors remain under the existing theme authority.
- [Interactive comparison](ribbon_state_contrast_2026_10_03/index.html)
  covers all 15 themes in both modes. These are current production components
  rendered in the controlled Course harness. Full-page Grassland and Tundra views
  cover 1280, 393, and 320px widths.

## Separate validation contracts

Screenshot pixel samples show active/inactive surface ratios of 1.58-1.63:1 in
light mode and 2.17-2.20:1 in dark mode, for both tiers. These numbers describe the
rendered surfaces; they are not text accessibility ratios or visual acceptance.
Fresh comparison sheets were reviewed for every theme/mode, including the
previously weak Grassland, Coral reef, Underground, Salt marsh, and Wetland views.

The existing all-theme test now protects a 1.5:1 surface regression floor instead
of merely requiring different token values. Its independent 5.5:1 text check
remains unchanged. Density, focus, forced colors, reduced motion, responsive
overflow, and shared style ownership checks pass. Temporary screenshot studies
stay outside the permanent suite in `tests/_temp/`; pixel samples are one-time evidence.

The CSS mixing mechanism follows the
[MDN color-mix reference](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/color_value/color-mix).

## Real-app refresh

The standard capture launcher regenerated all 246 real-app images directly in
`docs/screenshots`, completed corpus metadata/atlas validation, and captured all
30 Instructor theme samples. The unchanged `crop-stack.sh` then assembled all
66 Instructor screenshots into the 1280 by 11748 crop stack in that same directory.
Real-app Grassland light, Tundra dark, Coral reef light, and Magma dark views were
also inspected at full size. No Git operations were used.
