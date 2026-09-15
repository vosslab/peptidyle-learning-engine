# PLE provided-avatar source catalog

This directory is the editable, first-party source for the PLE-provided avatar
catalog. The twelve SVGs in `svg/` are original abstract toy-brick, color, and
pattern illustrations made for PLE. They are licensed [CC-BY-4.0](https://creativecommons.org/licenses/by/4.0/).
They contain no LEGO mark, copied minifigure art, external resource, script,
font, filter, image, or embedded data.

## Source contract

- `manifest.json` is the canonical inventory. Its IDs are stable lowercase ASCII
  values; generator input follows its explicit `checksumInputOrder`. Each
  generated per-asset SHA-256 is over the exact UTF-8 bytes of the named SVG,
  without newline normalization; selection metadata is read in manifest order.
- Each source SVG has `viewBox="0 0 128 128"`, `preserveAspectRatio="xMidYMid meet"`,
  semantic drawing-order groups, and only local primitives or paths.
- SVG roots use `aria-hidden="true"` and `focusable="false"`: the host must expose
  the nonempty `name`/`description` from the manifest as the text alternative.
- The source grammar is intentionally bounded to `svg`, `g`, `rect`, `circle`,
  `ellipse`, `polygon`, `polyline`, `line`, and `path`, with local coordinate,
  transform, `id`, and presentation attributes only. It permits no URL
  reference, style element, event attribute, external href, script, animation,
  or foreign content.

## Construction notes

The family starts with readable circles, rectangles, polygons, and simple
silhouettes, following *Drawing Mentor*, lines 630--680: communicate an object
as basic shapes before surface detail. A shared 128-square `viewBox` follows
*Mastering SVG*, lines 635--700, so the same source coordinates scale cleanly
from a small picker tile to a large one. Meaningful drawing-order groups follow
*Generative Art with JavaScript and SVG*, lines 2349--2405, which describes
groups and symbols as organization and reuse tools.

## Render record

All twelve source files were rendered on a white picker surface at `24x24`,
`64x64`, and `128x128` pixels with librsvg. The fixed square viewBox filled each
square without clipping. At 24 pixels the main silhouette and high-contrast
color blocks remained distinguishable; 64 and 128 pixels retained the internal
patterns. Each source SVG is decorative, has no accessible name, and depends on
the manifest host label by design.
