// ribbon_icon.tsx - decorative same-origin glyph renderer for the Ribbon vocabulary.

import type { JSX } from "solid-js";

import { RIBBON_ICON_ASSET_PATH, type RibbonGlyphId } from "./ribbon_icons";

// `focusable` is an SVG accessibility attribute supported by browsers but is
// absent from the DOM library's SVG attribute type.
const NON_FOCUSABLE_SVG = { focusable: "false" } as unknown as JSX.SvgSVGAttributes<SVGSVGElement>;

/**
 * Decorative, same-origin glyph rendering for the closed Ribbon vocabulary.
 * The surrounding control supplies the accessible name.
 */
export function RibbonIcon(props: { readonly glyph: RibbonGlyphId }): JSX.Element {
  return (
    <svg
      class="ple-app-ribbon__icon"
      aria-hidden="true"
      {...NON_FOCUSABLE_SVG}
      data-ribbon-glyph={props.glyph}
    >
      <use href={`${RIBBON_ICON_ASSET_PATH}#${props.glyph}`} />
    </svg>
  );
}
