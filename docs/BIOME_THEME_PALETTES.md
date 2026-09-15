# Biome Theme Palettes

## Status

| Field | Value |
| --- | --- |
| Specification status | Proposed implementation contract |
| Proposed registry | 25 Course Themes |
| Palette shape | Four fixed colors in light mode and four fixed colors in dark mode |
| Runtime status | Not implemented |
| Current runtime | 15 persisted IDs, three light-only anchors, derived Surface |
| Narrow color evidence | 100 Accent-on-Canvas/Surface pairs pass 5.5:1 |
| Lowest measured pair | Salt Marsh light Accent on Canvas, 5.67:1 |
| Remaining blockers | Display-mode policy, semantic-token projection, legacy-ID migration, default theme, rendered acceptance |

The hex values are proposed source values. They become authoritative only after the complete
acceptance criteria in this specification pass. The existing 15-theme runtime remains authoritative
until the atomic cutover is complete.

## Scope

This specification defines:

- The proposed Course Theme IDs, chooser order, display names, and visual identities.
- Four fixed palette roles in coordinated light and dark modes.
- The semantic-token, contrast, persistence, migration, and rollout contracts.
- The evidence required to admit, revise, or remove a theme.

This specification does not define arbitrary user colors, per-Course palette editing, Question
content colors, Assessment Type colors, status colors, or the global PLE UI palette.

## Registry principles

- Theme count follows demonstrated visual differentiation; it is not a fixed product target.
- Every retained theme contributes recognizable light/dark territory that no other theme owns.
- Closely related habitats are valid only when their complete rendered palettes remain distinct.
- Ecological classification is secondary to recognizable, accessible visual identity.
- Theme IDs are durable data contracts. Display names and palette values may change through a
  reviewed migration without changing an ID.

## Naming requirements

- Use a concise biome or habitat name that predicts the palette's visual identity.
- Use a specific modifier when the unmodified term is ambiguous or visually generic.
- Mix biome and habitat categories when that produces clearer theme identities.
- Reject a name that differs ecologically but produces redundant rendered colors.

## Palette roles

Each theme provides the following four fixed colors in light mode and again in dark mode.

| Role | Purpose |
| --- | --- |
| **Canvas** | Main course page background. |
| **Surface** | Cards, panels, navigation, and other raised content areas. |
| **Secondary** | Supporting identity color for decoration, navigation emphasis, and selected regions. |
| **Accent** | Strongest identity color for course rails, emphasis, and eligible interactive uses. |

Accent carries the primary theme identity and Secondary reinforces it. Canvas and Surface remain
restrained reading backgrounds. A raw palette color is not automatically a valid text, icon,
control, or focus color; semantic use follows the contrast contract below.

Light and dark palettes are coordinated presentations of one Course Theme, not separate identities.
Non-thematic text and status colors remain part of the global PLE UI system.

## Compatibility boundary

This document is a design specification, not a description of the current runtime registry. PLE
currently persists a closed set of 15 Course Theme IDs and maps each ID to three light-only anchors:
Canvas, Secondary, and Accent. The browser derives Surface, cards, text, actions, links, focus, and
borders from those anchors. The current application declares `color-scheme: light`; it does not
select the dark palettes in this document.

The proposed registry contains 25 themes and eight fixed colors per theme. It cannot be copied into
the current three-anchor registry. Surface is an authoritative theme input in this specification;
the current runtime derives Surface with `color-mix()`.

## Palette data model

Each Course Theme has exactly four authoritative colors in light mode and four coordinated colors
in dark mode:

1. Canvas
2. Surface
3. Secondary
4. Accent

These eight fixed hex values are the complete stored registry input for a theme. PLE persists only
the stable theme ID in Course data; it does not copy palette colors into Course rows. Display mode
selects the light or dark four-color set without changing the Course Theme ID.

The browser derives text, links, actions, focus, quiet surfaces, cards, and borders through one
shared and tested projection. A measured exception may override one named semantic token. An
exception must not replace or silently alter an authoritative palette color.

## Proposed palette values

| Theme | Light Canvas | Light Surface | Light Secondary | Light Accent | Dark Canvas | Dark Surface | Dark Secondary | Dark Accent |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Tropical Rainforest** | #F3F8F3 | #FFFFFF | #D4E8D6 | #23643A | #101A14 | #18251C | #274732 | #7FD39A |
| **Cloud Forest** | #F2F7F6 | #FCFEFD | #D3E4E1 | #2B6263 | #11191A | #1A2526 | #2C4546 | #83CBCD |
| **Redwood Forest** | #F7F4F1 | #FFFDFC | #E7D8CE | #7A3F2B | #1B1512 | #271D18 | #493128 | #D99A7D |
| **Taiga** | #F2F7F7 | #FCFEFE | #C8E4EA | #264F49 | #0F1A1A | #172827 | #173F3D | #8FD9E6 |
| **Savanna** | #FAF7EC | #FFFDF7 | #F2D66D | #705B0B | #1D1A10 | #292417 | #5A4614 | #F0D56D |
| **Tallgrass Prairie** | #F6F8EF | #FEFFF9 | #DCE7B9 | #4F6B1F | #171A10 | #222718 | #3A4822 | #A8CA6B |
| **Sagebrush Steppe** | #F7F6F0 | #FFFDF8 | #DFDDC8 | #5E6542 | #191914 | #24251D | #414332 | #B2B58A |
| **Chaparral** | #F8F5EF | #FFFDF9 | #E4DBC9 | #79513A | #1B1713 | #272019 | #49392D | #D6A27F |
| **Heathland** | #F8F3F8 | #FFFDFE | #E8C8EE | #70407B | #1B141C | #271C28 | #54305E | #DCA1E8 |
| **Arctic Tundra** | #F4F7F9 | #FFFFFF | #D8E2E8 | #4A6072 | #13181D | #1D252C | #34434F | #A5BED0 |
| **Sand Desert** | #FBF6EC | #FFFDF8 | #EAD7B5 | #8A5421 | #1E1810 | #2B2116 | #503820 | #E2A663 |
| **Peat Bog** | #F7F3F1 | #FFFDFC | #E4D4D0 | #6D3F4A | #1A1415 | #261C1E | #463137 | #C994A1 |
| **Mangrove** | #F1F7F4 | #FCFEFD | #CCE2D7 | #245F4D | #0F1915 | #17241E | #294537 | #7CC9A7 |
| **Alpine Lake** | #F1F7FB | #FCFEFF | #CEE4F1 | #2B6081 | #101820 | #18242E | #29445A | #7DBEE9 |
| **Salt Marsh** | #F5F8F2 | #FEFFF9 | #D8E5CF | #486A51 | #131A15 | #1D261F | #354838 | #9BC7A4 |
| **Rocky Shore** | #F4F6F7 | #FEFFFF | #D8E0E3 | #4B6470 | #14181B | #1E2529 | #37454B | #A4C2CC |
| **Open Ocean** | #F0F6FB | #FCFEFF | #C9E0F2 | #1E5E91 | #0D1720 | #142331 | #203E55 | #69B7EA |
| **Coral Reef** | #FFF4F2 | #FFFDFB | #F5D2CA | #9B493A | #1E1413 | #2B1D1A | #54332D | #F08E79 |
| **Kelp Forest** | #F4F7F0 | #FEFFF9 | #E4D28A | #365E47 | #0C181D | #13252B | #5A4B1D | #89CEA5 |
| **Wildflower Meadow** | #FBF4F8 | #FFFDFE | #F1CCE0 | #85345E | #1C1419 | #291C24 | #552C43 | #F0A2C7 |
| **Autumn Woodland** | #FCF5ED | #FFFDF8 | #F2D1A2 | #8A3F14 | #1D1510 | #2A1E15 | #55331E | #F0A45F |
| **Tropical Lagoon** | #EFFAF9 | #FBFFFE | #BDEDE8 | #0C686F | #0D1B1D | #14282B | #1D5055 | #5DDBD6 |
| **Volcanic Field** | #F8F2F0 | #FFFDFC | #E5CCC5 | #8B3125 | #1A1211 | #281A18 | #573029 | #F18B72 |
| **Red Rock Canyon** | #FBF3EB | #FFFDF8 | #F0CCAA | #8C451A | #1D1510 | #2A1D15 | #58351F | #EFA469 |
| **Glacier** | #F1FAFC | #FCFFFF | #C9EEF4 | #246678 | #0D191E | #14272E | #24515E | #74D7EB |

## Visual identity requirements

| Theme | Primary visual identity |
| --- | --- |
| **Tropical Rainforest** | emerald, dense foliage, humid tropical green |
| **Cloud Forest** | misty teal-green, blue-gray, fog |
| **Redwood Forest** | redwood bark, deep forest green |
| **Taiga** | near-black spruce, icy blue, snow |
| **Savanna** | bright dry gold, dark ochre, acacia green |
| **Tallgrass Prairie** | rich grass green, straw, open sky |
| **Sagebrush Steppe** | silver sage, wheat, dusty earth |
| **Chaparral** | gray-green, sage, terracotta |
| **Heathland** | subdued heather, violet, muted green |
| **Arctic Tundra** | lichen, icy gray-blue, stone |
| **Sand Desert** | sand, amber, burnt orange |
| **Peat Bog** | peat brown, sphagnum green, cranberry-purple |
| **Mangrove** | dark tropical green, muddy teal |
| **Alpine Lake** | clear blue, granite, snow |
| **Salt Marsh** | sea green, straw, tidal blue |
| **Rocky Shore** | slate, seafoam, dark water |
| **Open Ocean** | deep blue, bright cyan |
| **Coral Reef** | turquoise, coral, tropical blue |
| **Kelp Forest** | dark marine green, golden kelp, deep blue water |
| **Wildflower Meadow** | pink, magenta, violet, fresh green |
| **Autumn Woodland** | scarlet foliage, orange, amber, chestnut |
| **Tropical Lagoon** | vivid aqua, bright turquoise, pale sand |
| **Volcanic Field** | lava red, ember orange, basalt charcoal |
| **Red Rock Canyon** | orange-red rock, sandstone, dark umber |
| **Glacier** | white ice, pale glacial cyan, intense ice blue |

## Differentiation requirements

Review the palettes as one complete system. Related habitats may share broad color families only
when each occupies different visual territory across its complete light/dark palette.

- **Tropical Rainforest** is saturated and emerald. **Cloud Forest** is cooler, mistier, and more teal-gray.
- **Redwood Forest** uses red-brown bark tones to separate it from the green forest themes.
- **Taiga** pairs near-black spruce with icy blue and snow rather than misty teal-gray.
- **Savanna** owns bright dry gold through its decorative Secondary while its dark ochre Accent remains readable. **Tallgrass Prairie** is fresher and greener. **Sagebrush Steppe** is muted silver-sage and earth.
- **Chaparral** combines restrained sage with warm earth. **Heathland** owns subdued heather and violet.
- **Arctic Tundra** emphasizes muted lichen, gray-blue, and stone. **Glacier** is brighter and more chromatic, using white ice, glacial cyan, and intense ice blue.
- **Sand Desert** is warm sand and amber. **Red Rock Canyon** is more saturated orange-red, sandstone, and dark umber.
- **Peat Bog** uses peat and cranberry-purple. **Mangrove** is darker and more tropical. **Salt Marsh** shifts toward sea green and straw.
- **Alpine Lake** is clear blue with granite and snow.
- **Rocky Shore** is slate and seafoam. **Open Ocean** owns the deepest blue/cyan identity.
- **Coral Reef** is coral-led. **Tropical Lagoon** owns the brightest aqua and turquoise territory.
- **Kelp Forest** combines dark marine green, golden kelp, and deep water rather than generic terrestrial olive.
- **Wildflower Meadow** owns pink and magenta, while **Heathland** remains a subdued purple-green theme.
- **Autumn Woodland** owns scarlet, orange, and amber foliage rather than bark or generic warm brown.
- **Volcanic Field** owns lava red, ember, and basalt charcoal.

Each palette must evoke its named habitat and remain distinguishable from every other available
theme. Ecological realism does not justify a visually redundant palette.

## Accessibility requirements

Use contrast requirements by semantic role. A single ratio for every palette color would
unnecessarily mute decorative colors without proving that the rendered interface is accessible.

| Use | Required contrast | Palette rule |
| --- | --- | --- |
| Normal-sized text, including links | At least 4.5:1 against its rendered background | Prefer 5.5:1 when it preserves the intended palette identity. |
| Large text | At least 3:1 against its rendered background | Do not rely on this exception for ordinary controls or body text. |
| Meaningful icons, control boundaries, and focus indicators | At least 3:1 against adjacent colors | Test the actual component and every state in which the visual cue is required. |
| Decorative Secondary color | No standalone ratio | Provide a separately tested foreground token whenever text or a meaningful icon appears on it. |
| Canvas, Surface, and decorative boundaries | No standalone ratio | Require 3:1 when the boundary itself is necessary to identify a component or state. |

The 5.5:1 value is a preferred comfort target for normal thematic text, not an admission requirement
for every palette color. Passing it does not replace the rendered checks required for a component's
actual semantic role.

Required validation:

1. Test every foreground/background text pair produced by the implementation.
2. Test normal-sized Accent text on Canvas and Surface at the 4.5:1 required floor and record whether it also reaches the preferred 5.5:1 target.
3. Test the chosen foreground token for any text or meaningful icon placed on Secondary.
4. Test links, buttons, selected states, focus indicators, validation states, and disabled states in context.
5. Review all themes side by side in both modes for visual duplication.
6. Adjust a failing or redundant palette while preserving its declared visual identity.

Decorative colors have no text-contrast requirement unless they carry text or a meaningful icon.
They must remain distinguishable from adjacent surfaces whenever that distinction communicates
structure or state.

## Proposed registry

The order below is the Course Appearance chooser order for the proposed registry. Each ID is the
permanent wire value for its theme. Changing a display name does not change its ID.

| Order | ID | Display name |
| ---: | --- | --- |
| 1 | `tropical-rainforest` | Tropical Rainforest |
| 2 | `cloud-forest` | Cloud Forest |
| 3 | `redwood-forest` | Redwood Forest |
| 4 | `taiga` | Taiga |
| 5 | `savanna` | Savanna |
| 6 | `tallgrass-prairie` | Tallgrass Prairie |
| 7 | `sagebrush-steppe` | Sagebrush Steppe |
| 8 | `chaparral` | Chaparral |
| 9 | `heathland` | Heathland |
| 10 | `arctic-tundra` | Arctic Tundra |
| 11 | `sand-desert` | Sand Desert |
| 12 | `peat-bog` | Peat Bog |
| 13 | `mangrove` | Mangrove |
| 14 | `alpine-lake` | Alpine Lake |
| 15 | `salt-marsh` | Salt Marsh |
| 16 | `rocky-shore` | Rocky Shore |
| 17 | `open-ocean` | Open Ocean |
| 18 | `coral-reef` | Coral Reef |
| 19 | `kelp-forest` | Kelp Forest |
| 20 | `wildflower-meadow` | Wildflower Meadow |
| 21 | `autumn-woodland` | Autumn Woodland |
| 22 | `tropical-lagoon` | Tropical Lagoon |
| 23 | `volcanic-field` | Volcanic Field |
| 24 | `red-rock-canyon` | Red Rock Canyon |
| 25 | `glacier` | Glacier |

## Deferred themes

These themes are excluded from the proposed registry. A future revision may admit one only after
all eight colors, semantic tokens, rendered comparisons, and acceptance evidence pass.

- **Lavender Field:** demonstrate bright lavender/violet territory distinct from Heathland's subdued
  heather and green.
- **Sunflower Meadow:** demonstrate bright yellow/gold territory unavailable from Savanna while
  every semantic use meets its role-specific contrast requirement.
- **Cypress Swamp:** demonstrate cypress green, tannin water, moss, and warm brown territory distinct
  from Peat Bog, Mangrove, Redwood Forest, and Chaparral.

## Runtime record requirements

Each runtime theme record must contain or resolve all of the following:

- The permanent kebab-case ID used by Rust, generated TypeScript, PostgreSQL, APIs, and Course data.
- A student- and Instructor-visible display name.
- An explicit position in the Course Appearance chooser.
- The four fixed light colors and four fixed dark colors in the proposed palette table.
- The shared deterministic projection recipe and any measured semantic-token exceptions needed in
  either mode.
- Measured contrast evidence for every rendered foreground/background pair.
- A reference to the accepted light/dark rendered evidence at narrow and wide viewports.

Unknown IDs must fail closed. No decoder, persistence layer, or browser registry may silently select
a fallback theme.

### Semantic runtime tokens

Canvas, Surface, Secondary, and Accent do not cover every component PLE renders. Each theme must
resolve these tokens in both modes:

| Token | Required use |
| --- | --- |
| `canvas` | Course environment background. |
| `surface` | Primary panels, navigation, and raised regions. |
| `surfaceSoft` | Quiet grouping and inset regions. |
| `card` | Reading and work cards. |
| `secondary` | Decorative support, navigation identity, and restrained emphasis. |
| `accent` | Decorative course rail and primary theme identity. |
| `ink` | Ordinary text. |
| `muted` | Secondary explanatory text. |
| `link` | Links and link states. |
| `action` | Interactive controls. |
| `actionHover` | Pointer-hover treatment for actions. |
| `onAction` | Text and meaningful icons on Action. |
| `onSecondary` | Text and meaningful icons on Secondary. |
| `focus` | Keyboard focus indicator. |
| `border` | Necessary component boundaries and selected states. |

The eight palette colors are the registry inputs. Apply one shared, documented
projection recipe to produce all other tokens in each mode. Explicit overrides are allowed only for
measured exceptions and must name the affected semantic token. Do not add undocumented per-theme
derivations. Browser-computed colors remain the accessibility oracle.

### Display-mode contract

Course Theme and display mode are separate concerns. The persisted Course Theme selects the habitat
identity. The light/dark choice selects that theme's coordinated four-color palette and must not
rewrite Course data.

Both light and dark palettes are required for the proposed registry. The display-mode implementation
must define:

- Whether mode follows the operating system, an account-backed preference, or a three-state
  System/Light/Dark control.
- Where that preference is stored and how it behaves before authentication.
- How the selected mode reaches every Course route, Assessment Attempt, history view, preview, and
  persistent Ribbon without a flash of the wrong mode.
- How forced-colors overrides both palettes while preserving semantics and keyboard visibility.
- Whether increased contrast is a separate presentation preference and how it composes with both
  light and dark mode.

Until these decisions are settled and implemented, the application remains light-only. The dark
values in this specification do not constitute runtime dark-mode support.

### Migration contract

The current 15 persisted IDs do not correspond one-to-one with the proposed registry. Before the
closed registry changes, assign one disposition to every existing ID:

- Retain it as a legacy supported theme.
- Map it once to one new permanent ID through an explicit data migration.
- Redesign it under the same durable ID when preserving that Course identity is intentional.

The decision must cover `tundra`, `forest`, `desert`, `grass`, `arctic`, `ocean`, `tropical`,
`coral-reef`, `swamp`, `underground`, `salt-marsh`, `wetland`, `sea-floor`, `magma`, and `beach`.
Do not infer mappings from similar names. The migration record must state whether existing Courses
retain their appearance, receive a documented successor, or require an Instructor choice. It must
also select the new-Course default; the current default is `grass`.

### Rollout contract

The supported ID set must change as one bounded cross-layer operation. Update and verify:

- The Rust `CourseTheme` enum, `ALL` order, string codec, default, and generated TypeScript union.
- PostgreSQL's `course_theme` constraint, default, migration behavior, and existing rows.
- The browser registry, display names, light/dark tokens, and Course Appearance chooser order.
- The shared projection from each mode's fixed Canvas, Surface, Secondary, and Accent values to the
  remaining semantic tokens.
- API request/response decoding and every Course, Assessment Attempt, and history projection that
  carries the theme ID.
- Course Appearance preview, immediate unsaved preview, save/reload behavior, and error recovery.
- Seeded Courses, fixtures, screenshots, documentation, and generated evidence that enumerate
  themes.

The cutover is atomic across persisted values and readers. A database value must not ship before
every reader accepts it. A value must not be removed while a stored Course can contain it.

## Acceptance criteria

Implementation is complete only when all of these checks pass:

1. Rust, PostgreSQL, generated TypeScript, and the browser registry expose the same ordered ID set.
2. Every theme resolves a complete token set in every supported display mode with no fallback.
3. Normal text, large text, meaningful non-text UI, focus, and necessary boundaries meet their
   role-specific contrast requirements in browser-computed colors.
4. Hover, active, selected, disabled, validation, and focus states remain distinguishable without
   color as the only signal.
5. Forced-colors preserves content, controls, selection, and focus visibility.
6. Instructor preview, save, reload, Student Course presentation, Assessment Attempts, and history
   views preserve the selected theme.
7. A light/dark contact sheet shows every theme at narrow and wide viewports using real applied
   roles rather than isolated swatches.
8. Human visual review confirms that every retained theme contributes recognizable territory and
   that no pair is confusingly similar.

Deferred themes follow the same gate. Passing isolated Accent contrast does not satisfy admission.

## Unresolved decisions

The following decisions block runtime implementation and require explicit product direction:

1. Display-mode selection: operating-system only or a System/Light/Dark account preference.
2. Pre-authentication mode behavior and prevention of a wrong-mode flash.
3. Composition of display mode with the account-backed increased-contrast preference.
4. The shared semantic-token projection and the allowed form of measured exceptions.
5. The disposition of every existing persisted theme ID.
6. The default theme for newly created Courses.

Do not resolve these items implicitly during implementation.
