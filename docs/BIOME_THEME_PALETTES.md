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

A Course created from a Blueprint starts with the Blueprint's Theme. The Instructor can then
change the Course Theme independently. The starting copy is not an ongoing Theme dependency.

There is no System or Auto display-mode option, Theme-strength setting, or per-Course display mode.

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

## Semantic projection

The five source colors define each look. Interface colors derived from them
must use one shared semantic projection across all Themes. Do not add
Theme-specific derived-color exceptions.

The current shared projection is implemented in
[`src/appearance/theme_registry.ts`](../src/appearance/theme_registry.ts).
For each look, it chooses `ink` between `#172033` and white for the stronger
contrast against Canvas. It derives muted text, links, focus, strong and
ordinary borders, and hover from shared Canvas, Secondary, Accent, and readable
text blends; Accent supplies actions. Text on Accent, Secondary, Highlight, and
ink is independently chosen between `#172033` and white for stronger contrast
against that color. The exact blend shares belong to the shared implementation
rule and may change when rendered evidence requires a correction.

This is one implementation rule for all 30 looks. The all-look contrast gate
and independent rendered review accepted the palette catalog. A later
correction belongs in the shared rule or five source colors, never in a
Theme-specific derived token.

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
is the palette authority for the completed whole-interface Theme implementation.

| Field | Current evidence |
| --- | --- |
| Registry | 15 durable Theme IDs in Rust, SQL, generated TypeScript, and the browser registry |
| Default | `grass` |
| Source colors | Canvas, Surface, Secondary, Accent, and Highlight for every Theme and display mode |
| Token projection | One shared browser derivation for all semantic tokens |
| Palette values | All 15 Light/Dark pairs are current implementation values |
| Contrast gate | One behavior-level contrast test iterates all 30 looks and verifies normal text, text on color, focus, and control boundaries |
| Visual acceptance | Independent review accepted the 16 Forest, Arctic, Magma, and Desert Light/Dark workspace/editor pilots and the final 30-look Course-workspace corpus |

Each row gives the five source colors in Canvas / Surface / Secondary / Accent /
Highlight order. They are source data, not a claim that a look has passed the
whole-interface acceptance criteria.

## Registry

| Theme ID | Name | Light | Dark |
| --- | --- | --- | --- |
| `tundra` | Tundra | `#d8d0dc` / `#eee8ef` / `#d5c5d9` / `#485b3c` / `#e5d7e6` | `#24212a` / `#332d39` / `#413447` / `#93ad83` / `#493b50` |
| `forest` | Forest | `#d8eadc` / `#f4fbf4` / `#c2e3c8` / `#17643b` / `#b8e3a5` | `#102319` / `#183226` / `#254a36` / `#a7e39d` / `#315c3d` |
| `desert` | Desert | `#f7e6c5` / `#fff9ee` / `#eacb8e` / `#744117` / `#f3d9a7` | `#261c10` / `#382816` / `#59401e` / `#ffd18a` / `#6a4b20` |
| `grass` | Grassland | `#cfe0a6` / `#e7f0ca` / `#d8e7b5` / `#008852` / `#edf4d1` | `#202919` / `#2e3b25` / `#394d2d` / `#55bd88` / `#465b38` |
| `arctic` | Arctic | `#dceff5` / `#f6fcfe` / `#b9dce8` / `#1d5e78` / `#c6e8f5` | `#10232a` / `#18343e` / `#27505d` / `#9adcf2` / `#315f70` |
| `ocean` | Ocean | `#c6e1ef` / `#e0f0f6` / `#cfe6f0` / `#123c69` / `#d8ecf4` | `#172b36` / `#233e4a` / `#2c5362` / `#76a9dd` / `#315565` |
| `tropical` | Tropical | `#d5e6ae` / `#edf4ce` / `#ddefbb` / `#8a1976` / `#e6c6df` | `#251f2b` / `#372b3a` / `#433348` / `#cf76bd` / `#513b50` |
| `coral-reef` | Coral reef | `#c5e7df` / `#dcf3ed` / `#cfece4` / `#b52d3d` / `#f0cbd0` | `#1d2b2a` / `#2a3d3a` / `#344c49` / `#e97883` / `#4d393d` |
| `swamp` | Swamp | `#d8d5ac` / `#eeeac4` / `#e2deb6` / `#4b3426` / `#e7cdb2` | `#2d2b1f` / `#403d28` / `#514c31` / `#b89474` / `#5a4635` |
| `underground` | Underground | `#dbd1c5` / `#eee5da` / `#e3d8cb` / `#c9732c` / `#efd1b8` | `#29231f` / `#3c312a` / `#4c3c32` / `#e99b59` / `#594238` |
| `salt-marsh` | Salt marsh | `#ccdcca` / `#e4eee1` / `#d5e5d4` / `#76511f` / `#d8e8d8` | `#202c28` / `#2e4039` / `#385149` / `#c49a61` / `#40584f` |
| `wetland` | Wetland | `#d1e1c8` / `#e7efdf` / `#dae8d4` / `#3b648c` / `#d0dfeb` | `#202b24` / `#2e4034` / `#394f40` / `#7da7d0` / `#40546a` |
| `sea-floor` | Sea floor | `#cbd8df` / `#e0e9ed` / `#d5e1e7` / `#086a72` / `#cae4e4` | `#1c2b31` / `#2a3e46` / `#354f59` / `#58adb2` / `#3b5960` |
| `magma` | Magma | `#f6ded6` / `#fff8f5` / `#f0b9a9` / `#7a261f` / `#f1c3a4` | `#281514` / `#3a201e` / `#5d302b` / `#ffae8e` / `#733a32` |
| `beach` | Beach | `#e7d7ab` / `#f3e7c6` / `#eddfb9` / `#8a3d24` / `#cae8e3` | `#2d291e` / `#403929` / `#514730` / `#d38568` / `#365451` |
