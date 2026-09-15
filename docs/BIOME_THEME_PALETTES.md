# Biome Theme Palettes

## Status

The theme names and hex values in this document are an initial design specification. The hex values are **preliminary and subject to contrast testing in the rendered PLE interface** before becoming authoritative.

The initial palette has been checked for the limited case of **Accent used as text or an icon on its own Canvas and Surface**. All 48 light/dark Accent pairings meet the current 5.5:1 house target in that check; the lowest preliminary ratio is 5.62:1 (Savanna, light mode). This is not a complete accessibility audit. Actual text, links, controls, focus states, selected states, disabled states, and any use of Secondary still require testing in context.

## Purpose

PLE course themes use recognizable biome and habitat names with coordinated light and dark palettes. Theme names should evoke a clear visual scene and suggest a distinct palette without requiring strict ecological classification.

The 24-theme set was selected from a broader candidate vocabulary to favor visual separation. Closely related habitat names were omitted when they would likely produce redundant palettes.

## Naming guidance

- Prefer specific, visually evocative habitat names over broad ecological categories.
- Use names such as **Kelp Forest**, **Alpine Lake**, and **Peat Bog** instead of generic names such as Kelp, Lake, or Wetland.
- Add a modifier when it meaningfully narrows the visual identity.
- Keep names concise and recognizable.
- Biomes and habitats may be mixed when that produces clearer and more distinctive themes.
- Favor a smaller set of strongly differentiated themes over many themes occupying similar visual territory.

A useful naming test is: **Could someone picture the habitat from the name and anticipate its colors?**

## Palette roles

Each theme provides four fixed colors in light mode and four fixed colors in dark mode.

| Role | Purpose |
| --- | --- |
| **Canvas** | Main course page background. |
| **Surface** | Cards, panels, navigation, and other raised content areas. |
| **Secondary** | Supporting theme color for subtle emphasis, borders, selected areas, and other restrained thematic treatment. |
| **Accent** | Strongest theme color for links, controls, icons, and primary visual identity. |

Accent carries most of the recognizable theme identity, with Secondary reinforcing it. Canvas and Surface remain restrained so the theme does not compete with course content.

Theme colors are fixed hex values rather than runtime-generated transformations. Light and dark palettes are coordinated variants of the same theme, not separate theme identities.

Text colors that are not explicitly thematic should remain part of the general PLE UI color system rather than being derived from these biome palettes.

## Preliminary palettes

| Theme | Light Canvas | Light Surface | Light Secondary | Light Accent | Dark Canvas | Dark Surface | Dark Secondary | Dark Accent |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Tropical Rainforest** | #F3F8F3 | #FFFFFF | #D4E8D6 | #23643A | #101A14 | #18251C | #274732 | #7FD39A |
| **Cloud Forest** | #F2F7F6 | #FCFEFD | #D3E4E1 | #2B6263 | #11191A | #1A2526 | #2C4546 | #83CBCD |
| **Redwood Forest** | #F7F4F1 | #FFFDFC | #E7D8CE | #7A3F2B | #1B1512 | #271D18 | #493128 | #D99A7D |
| **Taiga** | #F2F6F6 | #FCFEFE | #D6E2E3 | #315C62 | #11191B | #192427 | #2B4146 | #8BC3CC |
| **Savanna** | #FAF7EC | #FFFDF7 | #E9DDAE | #76610F | #1D1A10 | #292417 | #4A4020 | #D8C268 |
| **Tallgrass Prairie** | #F6F8EF | #FEFFF9 | #DCE7B9 | #4F6B1F | #171A10 | #222718 | #3A4822 | #A8CA6B |
| **Sagebrush Steppe** | #F7F6F0 | #FFFDF8 | #DFDDC8 | #5E6542 | #191914 | #24251D | #414332 | #B2B58A |
| **Chaparral** | #F8F5EF | #FFFDF9 | #E4DBC9 | #79513A | #1B1713 | #272019 | #49392D | #D6A27F |
| **Heathland** | #F8F3F8 | #FFFDFE | #E7D6E8 | #744878 | #1B141C | #271C28 | #49344B | #D09BD4 |
| **Arctic Tundra** | #F4F7F9 | #FFFFFF | #D8E2E8 | #4A6072 | #13181D | #1D252C | #34434F | #A5BED0 |
| **Alpine Meadow** | #F4F8F3 | #FEFFFC | #D9E6D3 | #496A44 | #141A13 | #1E261C | #364830 | #A0C990 |
| **Sand Desert** | #FBF6EC | #FFFDF8 | #EAD7B5 | #8A5421 | #1E1810 | #2B2116 | #503820 | #E2A663 |
| **Palm Oasis** | #F1F9F7 | #FCFFFE | #CCEAE3 | #19666B | #0F1B1A | #162726 | #244947 | #65D2C3 |
| **Peat Bog** | #F7F3F1 | #FFFDFC | #E4D4D0 | #6D3F4A | #1A1415 | #261C1E | #463137 | #C994A1 |
| **Reed Marsh** | #F7F8EE | #FFFFF9 | #DFE5BD | #59691F | #181A10 | #232718 | #404A22 | #B7CD70 |
| **Mangrove** | #F1F7F4 | #FCFEFD | #CCE2D7 | #245F4D | #0F1915 | #17241E | #294537 | #7CC9A7 |
| **Mountain River** | #F2F7FA | #FCFEFF | #D1E3EC | #2D5F78 | #10181D | #18242B | #2A4352 | #7FC1E0 |
| **Alpine Lake** | #F1F7FB | #FCFEFF | #CEE4F1 | #2B6081 | #101820 | #18242E | #29445A | #7DBEE9 |
| **Coastal Estuary** | #F3F8F6 | #FDFEFD | #D3E5DF | #37675E | #111A18 | #1A2623 | #2E4842 | #8CC8BA |
| **Salt Marsh** | #F5F8F2 | #FEFFF9 | #D8E5CF | #486A51 | #131A15 | #1D261F | #354838 | #9BC7A4 |
| **Rocky Shore** | #F4F6F7 | #FEFFFF | #D8E0E3 | #4B6470 | #14181B | #1E2529 | #37454B | #A4C2CC |
| **Open Ocean** | #F0F6FB | #FCFEFF | #C9E0F2 | #1E5E91 | #0D1720 | #142331 | #203E55 | #69B7EA |
| **Coral Reef** | #FFF4F2 | #FFFDFB | #F5D2CA | #9B493A | #1E1413 | #2B1D1A | #54332D | #F08E79 |
| **Kelp Forest** | #F4F7F0 | #FEFFF9 | #D7E1C7 | #4E672F | #141A10 | #1E2717 | #384827 | #A6C978 |

## Visual identity

| Theme | Primary visual identity |
| --- | --- |
| **Tropical Rainforest** | emerald, dense foliage, humid tropical green |
| **Cloud Forest** | misty teal-green, blue-gray, fog |
| **Redwood Forest** | redwood bark, deep forest green |
| **Taiga** | spruce, cold blue, snow |
| **Savanna** | dry gold, warm grass, acacia green |
| **Tallgrass Prairie** | rich grass green, straw, open sky |
| **Sagebrush Steppe** | silver sage, wheat, dusty earth |
| **Chaparral** | gray-green, sage, terracotta |
| **Heathland** | heather purple, muted green |
| **Arctic Tundra** | lichen, icy gray-blue, stone |
| **Alpine Meadow** | mountain green, wildflowers, cool blue |
| **Sand Desert** | sand, amber, burnt orange |
| **Palm Oasis** | palm green, turquoise water, warm sand |
| **Peat Bog** | peat brown, sphagnum green, cranberry-purple |
| **Reed Marsh** | reed green, yellow-green, shallow water |
| **Mangrove** | dark tropical green, muddy teal |
| **Mountain River** | cold blue, stone, evergreen |
| **Alpine Lake** | clear blue, granite, snow |
| **Coastal Estuary** | blue-green, sand, silt |
| **Salt Marsh** | sea green, straw, tidal blue |
| **Rocky Shore** | slate, seafoam, dark water |
| **Open Ocean** | deep blue, bright cyan |
| **Coral Reef** | turquoise, coral, tropical blue |
| **Kelp Forest** | kelp olive, marine green, deep water |

## Differentiation guidance

The palettes should be reviewed as one complete system rather than as 24 independent themes. Related habitats may share broad color families, but each should occupy different visual territory.

- **Tropical Rainforest** is saturated and emerald. **Cloud Forest** is cooler, mistier, and more teal-gray.
- **Redwood Forest** uses red-brown bark tones to separate it from the green forest themes.
- **Taiga** emphasizes cold spruce and blue rather than generic forest green.
- **Savanna** emphasizes dry gold. **Tallgrass Prairie** is fresher and greener. **Sagebrush Steppe** is muted silver-sage and earth.
- **Chaparral** combines restrained sage with warm earth. **Heathland** owns the heather-purple region.
- **Arctic Tundra** emphasizes ice, lichen, and stone. **Alpine Meadow** is greener and more botanical.
- **Sand Desert** is warm and mineral. **Palm Oasis** contrasts it with turquoise water and palm green.
- **Peat Bog** uses peat and cranberry-purple. **Reed Marsh** is yellow-green. **Mangrove** is darker and more tropical.
- **Mountain River** is cool moving water with stone and evergreen. **Alpine Lake** is clearer blue with granite and snow.
- **Coastal Estuary** is muted blue-green and silt. **Salt Marsh** shifts toward sea green and straw.
- **Rocky Shore** is slate and seafoam. **Open Ocean** owns the deepest blue/cyan identity.
- **Coral Reef** owns coral and tropical turquoise.
- **Kelp Forest** uses olive kelp and marine green rather than terrestrial forest green.

Ecological realism is secondary to clear theme identity. A palette should evoke its habitat while remaining visually distinct from the other available themes.

## Contrast and validation

The house contrast target is 5.5:1 for foreground/background text pairs. Palette review should test actual semantic uses rather than assuming that a color is accessible merely because the palette looks balanced.

Before these values become authoritative:

1. Test every foreground/background text pair produced by the implementation.
2. Test Accent on Canvas and Surface in both modes.
3. Test any text or icon use of Secondary against its actual background.
4. Test links, buttons, selected states, focus indicators, validation states, and disabled states in context.
5. Review the 24 themes side by side in both modes for visual duplication.
6. Adjust individual colors where contrast or distinctiveness requires it, while preserving the theme's intended identity.

Decorative colors do not need to satisfy the text contrast target merely because they appear in the palette, but they should remain visibly distinguishable from adjacent UI surfaces.

## Initial theme set

1. Tropical Rainforest
2. Cloud Forest
3. Redwood Forest
4. Taiga
5. Savanna
6. Tallgrass Prairie
7. Sagebrush Steppe
8. Chaparral
9. Heathland
10. Arctic Tundra
11. Alpine Meadow
12. Sand Desert
13. Palm Oasis
14. Peat Bog
15. Reed Marsh
16. Mangrove
17. Mountain River
18. Alpine Lake
19. Coastal Estuary
20. Salt Marsh
21. Rocky Shore
22. Open Ocean
23. Coral Reef
24. Kelp Forest
