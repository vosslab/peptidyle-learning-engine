# Palette contrast audit

This audit separates the global base palette from the 15 course themes. It
uses the project's 5.5:1 target for normal text and 3:1 for focus indicators
and boundaries; the method is defined in
[COLOR_CONTRAST_ACCESSIBILITY.md](COLOR_CONTRAST_ACCESSIBILITY.md).

## Base palette

The color-accessibility-expert generator inspected `src/style.css` in this
run against `#ffffff` at 5.5:1. It found 16 literal occurrences and seven
distinct color/source pairs. These are the global text and status colors, not
the course-theme anchors.

| Source file     | Hex       | Ratio vs `#ffffff` | Status |
| --------------- | --------- | -----------------: | ------ |
| `src/style.css` | `#0f3b5c` |            11.66:1 | Pass   |
| `src/style.css` | `#172033` |            16.27:1 | Pass   |
| `src/style.css` | `#174f78` |             8.66:1 | Pass   |
| `src/style.css` | `#176b3a` |             6.56:1 | Pass   |
| `src/style.css` | `#4d5a68` |             7.05:1 | Pass   |
| `src/style.css` | `#7a4b00` |             7.41:1 | Pass   |
| `src/style.css` | `#a12027` |             7.63:1 | Pass   |

Reproduce this narrow source-literal measurement without overwriting this
combined audit:

```bash
python3 /Users/vosslab/nsh/vosslab-skills/skills/color-accessibility-expert/scripts/generate_palette_audit.py \
  -i src/style.css \
  -o /private/tmp/ple_base_palette_audit.md \
  -b '#ffffff' \
  -r 5.5
```

## Course-theme catalog

The closed catalog is
[`src/features/course_appearance/theme_catalog.ts`](../src/features/course_appearance/theme_catalog.ts).
It contains these 15 current themes. Each row records the raw canvas,
secondary, and accent anchors exactly as the catalog declares them.

| Theme       | Canvas    | Secondary | Accent    |
| ----------- | --------- | --------- | --------- |
| Tundra      | `#e3e1da` | `#725e72` | `#485b3c` |
| Forest      | `#e4ebdd` | `#166747` | `#aa831a` |
| Desert      | `#f3e2bd` | `#c07a3b` | `#68402a` |
| Grass       | `#bddeb1` | `#73c167` | `#008852` |
| Arctic      | `#e5f5f8` | `#7cbed1` | `#1f5d78` |
| Ocean       | `#ddeff5` | `#0b6c88` | `#123c69` |
| Tropical    | `#e4f2d6` | `#1b7646` | `#8a1976` |
| Coral reef  | `#e8f6f1` | `#006d68` | `#b52d3d` |
| Swamp       | `#e8e5c9` | `#4e5f23` | `#4b3426` |
| Underground | `#e6e0d8` | `#59504a` | `#c9732c` |
| Salt marsh  | `#e8f0df` | `#1e6a6d` | `#76511f` |
| Wetland     | `#e4eee7` | `#466f59` | `#3b648c` |
| Sea floor   | `#dee8ed` | `#344e62` | `#086a72` |
| Magma       | `#f5e0cf` | `#a92720` | `#3b2928` |
| Beach       | `#f3e7c9` | `#56a8b0` | `#8a3d24` |

Raw anchors are decorative design inputs, not universal text foregrounds.
In particular, Grass preserves its Roosevelt-inspired `#bddeb1`, `#73c167`,
and `#008852` anchors. Where an anchor cannot meet the normal-text target,
the catalog provides the darker `action` and `link` tokens instead; no raw
anchor is silently remapped or reported as a text-pair pass.

## Executable rendered-pair evidence

Contrast for course themes is measured from browser-computed colors because
the relevant foreground/background pairs are composed from catalog tokens,
course-scope CSS, and component styles. The durable behavior gates are:

- [`tests/test_course_theme_scope.mjs`](../tests/test_course_theme_scope.mjs)
  checks the exact 15 IDs, complete tokens, 5.5:1 text pairs, and 3:1
  focus/boundary pairs directly from catalog values.
- The canonical real-stack `learner_delivery` scenario saves and reloads a
  course appearance through the production HTTPS origin. Screenshot publication
  is a separate deliberate operation, not a parallel browser suite.

Run the durable behavior gates:

```bash
node --import tsx --test tests/test_course_theme_scope.mjs
./run_playwright_tests.sh --scenario learner_delivery
```

When a fresh published screenshot corpus is deliberately required, run:

```bash
./capture_screenshots.sh
```

`./all_test.sh` validates the current system without publishing or rewriting screenshots. The
earlier 15-theme rendered comparison is accepted historical evidence, not a generated verifier or
current Validation command.

This document therefore does not claim a single white-background ratio for
every raw course swatch. The catalog table preserves the source palette; the
tests above are the current numerical oracle for the rendered pairs users see.
