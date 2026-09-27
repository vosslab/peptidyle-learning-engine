# Whole-interface Theme palettes


# Whole-interface Theme palettes

## Purpose

This document defines the permanent color model and palette catalog for PLE
Themes. It is the authority for Theme IDs, display names, source colors,
Light and Dark appearances, semantic color roles, and accessibility
requirements.

## Theme model

PLE provides 15 fixed biome and habitat Themes. Each Theme has a Light and
Dark appearance, producing 30 Theme/mode looks.

An Instructor has a personal Theme for global Instructor pages and
independently controls the Theme assigned to each Course. Course pages use
the Course Theme. Global Instructor pages use the Instructor's personal
Theme.

Display mode is independent of Theme. Light and Dark are the only selectable
modes. When no explicit preference is saved, the interface follows the
browser preference.

Changing display mode never changes the selected Theme. Changing a Theme
never changes display mode.

The default Theme is `grass` (Grassland).

## Source colors

Every Theme/mode look is defined by exactly five source colors:

| Color | Purpose |
| --- | --- |
| Canvas | Main page background and other largest background areas |
| Surface | Primary content surfaces, panels, cards, inputs, and major interface regions |
| Secondary | Supporting surfaces such as secondary navigation, headers, and quiet fills |
| Accent | Actions, links, focus, and other prominent interactive cues |
| Highlight | Selection, current items, badges, callouts, and other emphasized states |

Canvas and Surface must carry enough of the Theme's color identity that
Themes remain visibly distinct across major page areas. A Theme must not
depend primarily on thin rails, borders, links, or buttons for its identity.

## Registry

[keep the 15-theme table here, without the Review state column]

## Semantic projection

The five source colors define each look. Interface colors derived from them
must use one shared semantic projection across all Themes. Do not add
Theme-specific derived-color exceptions.

[Document the FINAL shared derivation rules here once settled.]

## Accessibility

Theme colors must meet accessibility requirements in their actual rendered
uses, not merely as isolated palette values.

Normal text and text on colored surfaces must meet at least 4.5:1 contrast.
Meaningful non-text controls, focus indicators, and required control
boundaries must meet at least 3:1 against adjacent colors.

Check text, controls, borders, focus, selection, disabled states, validation,
and other meaningful interaction states against their rendered backgrounds
in both Light and Dark modes.

Color must not be the only cue for selection, focus, saved status,
validation, or results.

## Theme stability

Theme IDs are durable identifiers. Changing a Theme's display name or colors
does not require a new ID.

The Theme catalog contains the 15 Themes defined here. New Themes or changes
to the five-color model require an intentional update to this specification.

## Status

This document records the current closed runtime registry from
[`src/appearance/theme_registry.ts`](../src/appearance/theme_registry.ts). It
is the palette authority for the active whole-interface Theme plan.

| Field | Current evidence |
| --- | --- |
| Registry | 15 durable Theme IDs in Rust, SQL, generated TypeScript, and the browser registry |
| Default | `grass` |
| Source colors | Canvas, Surface, Secondary, Accent, and Highlight for every Theme and display mode |
| Token projection | One shared browser derivation for all semantic tokens |
| Settled pilot pairs | Forest, Arctic, Magma, and Desert in Light and Dark |
| Other 11 pairs | Interim values pending rendered review and contrast acceptance |
| Acceptance | All-30 rendered contrast, browser, screenshot, and full-gate evidence remains pending |

Each row gives the five source colors in Canvas / Surface / Secondary / Accent /
Highlight order. They are source data, not a claim that a look has passed the
whole-interface acceptance criteria.

## Registry

| Theme ID | Name | Light | Dark | Review state |
| --- | --- | --- | --- | --- |
| `tundra` | Tundra | `#e3e1da` / `#fcfbfb` / `#725e72` / `#485b3c` / `#faf9f8` | `#202126` / `#2c2d34` / `#a68da6` / `#93ad83` / `#41414a` | Interim |
| `forest` | Forest | `#d8eadc` / `#f4fbf4` / `#c2e3c8` / `#17643b` / `#b8e3a5` | `#102319` / `#183226` / `#254a36` / `#a7e39d` / `#315c3d` | Pilot settled; acceptance pending |
| `desert` | Desert | `#f7e6c5` / `#fff9ee` / `#eacb8e` / `#744117` / `#f3d9a7` | `#261c10` / `#382816` / `#59401e` / `#ffd18a` / `#6a4b20` | Pilot settled; acceptance pending |
| `grass` | Grassland | `#bddeb1` / `#f7fbf6` / `#73c167` / `#008852` / `#f4faf2` | `#17271e` / `#23362a` / `#8dcc75` / `#55bd88` / `#314a37` | Interim |
| `arctic` | Arctic | `#dceff5` / `#f6fcfe` / `#b9dce8` / `#1d5e78` / `#c6e8f5` | `#10232a` / `#18343e` / `#27505d` / `#9adcf2` / `#315f70` | Pilot settled; acceptance pending |
| `ocean` | Ocean | `#ddeff5` / `#fbfdfe` / `#0b6c88` / `#123c69` / `#f9fcfd` | `#142630` / `#203641` / `#4ca5c0` / `#76a9dd` / `#2b4957` | Interim |
| `tropical` | Tropical | `#e4f2d6` / `#fcfdfa` / `#1b7646` / `#8a1976` / `#f9fcf5` | `#1c2b20` / `#293b2c` / `#5fb77c` / `#cf76bd` / `#3b523d` | Interim |
| `coral-reef` | Coral reef | `#e8f6f1` / `#fcfefd` / `#006d68` / `#b52d3d` / `#fafdfb` | `#172b2a` / `#243b39` / `#50b5aa` / `#e97883` / `#34504d` | Interim |
| `swamp` | Swamp | `#e8e5c9` / `#fcfcf9` / `#4e5f23` / `#4b3426` / `#faf9f4` | `#28291c` / `#373824` / `#94a65b` / `#b89474` / `#4b4b30` | Interim |
| `underground` | Underground | `#e6e0d8` / `#fcfbfa` / `#59504a` / `#c9732c` / `#faf8f6` | `#27231f` / `#36302b` / `#a69a91` / `#e99b59` / `#49413a` | Interim |
| `salt-marsh` | Salt marsh | `#e8f0df` / `#fcfdfb` / `#1e6a6d` / `#76511f` / `#fafcf8` | `#1d2925` / `#2a3933` / `#65afb0` / `#c49a61` / `#3b4c45` | Interim |
| `wetland` | Wetland | `#e4eee7` / `#fcfdfc` / `#466f59` / `#3b648c` / `#f9fbfa` | `#1d2923` / `#2a3930` / `#83ad91` / `#7da7d0` / `#3a4c40` | Interim |
| `sea-floor` | Sea floor | `#dee8ed` / `#fbfcfd` / `#344e62` / `#086a72` / `#f9fafb` | `#19262d` / `#26353e` / `#819bad` / `#58adb2` / `#364852` | Interim |
| `magma` | Magma | `#f6ded6` / `#fff8f5` / `#f0b9a9` / `#7a261f` / `#f1c3a4` | `#281514` / `#3a201e` / `#5d302b` / `#ffae8e` / `#733a32` | Pilot settled; acceptance pending |
| `beach` | Beach | `#f3e7c9` / `#fefcf9` / `#56a8b0` / `#8a3d24` / `#fcfaf4` | `#2c271d` / `#3b3529` / `#83c5ca` / `#d38568` / `#4d4635` | Interim |

## Application model

`Theme` is shared appearance data. An Instructor has a personal Theme for
global Instructor pages and independently controls the Theme assigned to each
Course. Course pages use the Course Theme. Global Instructor pages use the
Instructor's personal Theme. `grass` is the default Theme.

Display mode is independent of Theme. It has two displayed values: Light and
Dark. When the user has not explicitly selected either mode, the interface
follows the browser preference. Changing display mode never changes the
selected Theme, and changing a Theme never changes display mode.

One document-level appearance owner resolves the active Theme and display mode
and applies them consistently across the interface.

No Theme inherits from another Theme. There is no Theme strength setting,
per-Course display mode, or separate System/Auto mode.

## Palette model

Each Theme has five source colors in both Light and Dark modes:

- Canvas
- Surface
- Secondary
- Accent
- Highlight

These five colors define the visual identity of the Theme. Canvas and Surface
must carry enough of the palette that Themes remain visibly different across
major page areas, rather than differing only through thin accents, rails,
links, or buttons.

The five source colors are the stable palette contract. Additional interface
colors may be derived from them through one shared implementation recipe.
Exact derivation formulas are implementation details and may be adjusted when
rendered testing shows a need. Do not add per-Theme derived-token exceptions.
