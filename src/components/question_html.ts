// Authored educational HTML is sanitized at the browser rendering boundary.
import createDOMPurify from "dompurify";

import type { QuestionImageAssetTuple } from "../../generated/api/QuestionImageAssetTuple";
import { isQuestionSourceRelativeName } from "../question_image_source";

type HtmlImageBinding = {
  readonly sourceRelativeName: string;
  readonly questionImageAssetTuple: QuestionImageAssetTuple;
};

const HTML_TAGS = [
  "a",
  "abbr",
  "b",
  "blockquote",
  "br",
  "caption",
  "center",
  "code",
  "col",
  "colgroup",
  "dd",
  "del",
  "div",
  "dl",
  "dt",
  "em",
  "font",
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "hr",
  "i",
  "img",
  "ins",
  "kbd",
  "li",
  "mark",
  "ol",
  "p",
  "pre",
  "q",
  "s",
  "samp",
  "small",
  "span",
  "strong",
  "sub",
  "sup",
  "table",
  "tbody",
  "td",
  "tfoot",
  "th",
  "thead",
  "tr",
  "u",
  "ul",
  "var",
  "math",
  "merror",
  "mfenced",
  "mfrac",
  "mi",
  "mmultiscripts",
  "mn",
  "mo",
  "mover",
  "mpadded",
  "mphantom",
  "mroot",
  "mrow",
  "ms",
  "mspace",
  "msqrt",
  "mstyle",
  "msub",
  "msubsup",
  "msup",
  "mtable",
  "mtd",
  "mtext",
  "mtr",
  "munder",
  "munderover",
  "none",
];
const HTML_ATTRIBUTES = [
  "alt",
  "title",
  "href",
  "src",
  "style",
  "colspan",
  "rowspan",
  "scope",
  "headers",
  "width",
  "height",
  "align",
  "valign",
  "bgcolor",
  "border",
  "cellpadding",
  "cellspacing",
  "color",
  "face",
  "size",
  "start",
  "reversed",
  "span",
  "accent",
  "accentunder",
  "bevelled",
  "columnalign",
  "columnlines",
  "columnspacing",
  "columnspan",
  "depth",
  "displaystyle",
  "display",
  "fence",
  "form",
  "largeop",
  "length",
  "linethickness",
  "lspace",
  "mathbackground",
  "mathcolor",
  "mathsize",
  "mathvariant",
  "maxsize",
  "minsize",
  "movablelimits",
  "notation",
  "rowalign",
  "rowlines",
  "rowspacing",
  "rspace",
  "scriptlevel",
  "separator",
  "stretchy",
  "symmetric",
  "voffset",
  "xmlns",
  "open",
  "close",
  "separators",
];

// Diagram placement remains inside the renderer's isolated layout and paint containment.
// Resource-bearing properties are filtered through the function allowlist below.
const CSS_PROPERTIES = new Set([
  "color",
  "background",
  "background-color",
  "background-image",
  "background-size",
  "background-position",
  "background-repeat",
  "border",
  "border-top",
  "border-right",
  "border-bottom",
  "border-left",
  "border-color",
  "border-style",
  "border-width",
  "border-radius",
  "border-top-color",
  "border-right-color",
  "border-bottom-color",
  "border-left-color",
  "border-top-style",
  "border-right-style",
  "border-bottom-style",
  "border-left-style",
  "border-top-width",
  "border-right-width",
  "border-bottom-width",
  "border-left-width",
  "border-collapse",
  "border-spacing",
  "caption-side",
  "empty-cells",
  "table-layout",
  "font",
  "font-family",
  "font-size",
  "font-style",
  "font-weight",
  "font-variant",
  "line-height",
  "letter-spacing",
  "word-spacing",
  "text-align",
  "text-decoration",
  "text-decoration-color",
  "text-decoration-line",
  "text-decoration-style",
  "text-indent",
  "text-transform",
  "vertical-align",
  "white-space",
  "overflow-wrap",
  "word-break",
  "width",
  "height",
  "min-width",
  "max-width",
  "min-height",
  "max-height",
  "padding",
  "padding-top",
  "padding-right",
  "padding-bottom",
  "padding-left",
  "margin",
  "margin-top",
  "margin-right",
  "margin-bottom",
  "margin-left",
  "list-style-type",
  "list-style-position",
  "display",
  "position",
  "top",
  "right",
  "bottom",
  "left",
  "inset",
  "z-index",
  "visibility",
  "opacity",
  "box-sizing",
  "box-shadow",
  "text-shadow",
  "overflow",
  "overflow-x",
  "overflow-y",
  "float",
  "clear",
  "transform",
  "transform-origin",
  "flex",
  "flex-basis",
  "flex-direction",
  "flex-grow",
  "flex-shrink",
  "flex-wrap",
  "align-items",
  "align-content",
  "align-self",
  "justify-content",
  "justify-items",
  "justify-self",
  "gap",
  "row-gap",
  "column-gap",
  "order",
  "grid",
  "grid-template",
  "grid-template-columns",
  "grid-template-rows",
  "grid-column",
  "grid-row",
  "grid-area",
  "grid-auto-flow",
  "grid-auto-columns",
  "grid-auto-rows",
  "object-fit",
  "object-position",
  "aspect-ratio",
]);
const CSS_FUNCTIONS = new Set([
  "rgb",
  "rgba",
  "hsl",
  "hsla",
  "hwb",
  "lab",
  "lch",
  "oklab",
  "oklch",
  "color",
  "color-mix",
  "calc",
  "min",
  "max",
  "clamp",
  "linear-gradient",
  "radial-gradient",
  "conic-gradient",
  "repeating-linear-gradient",
  "repeating-radial-gradient",
  "repeating-conic-gradient",
  "translate",
  "translatex",
  "translatey",
  "translatez",
  "translate3d",
  "scale",
  "scalex",
  "scaley",
  "scalez",
  "scale3d",
  "rotate",
  "rotatex",
  "rotatey",
  "rotatez",
  "rotate3d",
  "skew",
  "skewx",
  "skewy",
  "matrix",
  "matrix3d",
  "perspective",
  "repeat",
  "minmax",
  "fit-content",
]);

/** ASVS 1.3.5: CSSOM parses declarations; only contained, non-resource presentation survives. */
export function sanitizeQuestionInlineStyle(source: string): string {
  const parsed = document.createElement("span").style;
  const safe = document.createElement("span").style;
  parsed.cssText = source;
  for (let index = 0; index < parsed.length; index += 1) {
    const property = parsed.item(index);
    if (!CSS_PROPERTIES.has(property)) continue;
    const value = parsed.getPropertyValue(property);
    if (property === "position" && !["static", "relative", "absolute"].includes(value)) continue;
    // Escapes and custom-property substitution are unnecessary for teaching presentation.
    // Permit only named CSS color/math functions, including nested functions.
    if (
      value.includes("\\") ||
      Array.from(value.matchAll(/([a-zA-Z_-][a-zA-Z0-9_-]*)\s*\(/gu)).some(
        (match) => !CSS_FUNCTIONS.has(match[1]?.toLowerCase() ?? ""),
      )
    )
      continue;
    safe.setProperty(property, value);
  }
  return safe.cssText;
}

function safeQuestionLink(value: string): boolean {
  try {
    const url = new URL(value);
    return (
      (url.protocol === "https:" || url.protocol === "mailto:") &&
      url.username === "" &&
      url.password === ""
    );
  } catch {
    return false;
  }
}

/**
 * ASVS 1.3.1: DOMPurify owns HTML parsing, namespace checks, and mutation-XSS protection.
 * Image URLs are supplied only by the caller's existing authenticated exact-asset resolver.
 * No HTML URL or externalResources inventory entry can authorize a fetch.
 */
export function sanitizeQuestionHtml(
  html: string,
  images: ReadonlyArray<HtmlImageBinding>,
  resolveImage: (image: QuestionImageAssetTuple) => string,
): DocumentFragment {
  const bindings = new Map(
    images.map((image) => [image.sourceRelativeName, image.questionImageAssetTuple]),
  );
  // A local instance keeps hooks isolated from other renderers and author-script runtimes.
  const purifier = createDOMPurify(window);
  purifier.addHook("uponSanitizeAttribute", (node, attribute) => {
    if (attribute.attrName === "style") {
      attribute.attrValue = sanitizeQuestionInlineStyle(attribute.attrValue);
      attribute.keepAttr = attribute.attrValue !== "";
    } else if (attribute.attrName === "src") {
      const source = attribute.attrValue;
      const binding = bindings.get(source);
      if (
        node.nodeName === "IMG" &&
        isQuestionSourceRelativeName(source) &&
        binding !== undefined
      ) {
        attribute.attrValue = resolveImage(binding);
      } else {
        attribute.keepAttr = false;
      }
    } else if (attribute.attrName === "href") {
      attribute.keepAttr = node.nodeName === "A" && safeQuestionLink(attribute.attrValue);
    }
  });
  const fragment = purifier.sanitize(html, {
    ALLOWED_TAGS: HTML_TAGS,
    ALLOWED_ATTR: HTML_ATTRIBUTES,
    ALLOW_ARIA_ATTR: false,
    ALLOW_DATA_ATTR: false,
    RETURN_DOM_FRAGMENT: true,
  });
  // An unbound image has no fetch route and should not appear as a broken diagram.
  for (const image of fragment.querySelectorAll("img:not([src])")) image.remove();
  return fragment;
}

/** Readable labels for announcements and native text-only option controls, including figure alt text. */
export function questionHtmlText(html: string): string {
  const purifier = createDOMPurify(window);
  const fragment = purifier.sanitize(html, {
    ALLOWED_TAGS: HTML_TAGS,
    ALLOWED_ATTR: ["alt"],
    ALLOW_ARIA_ATTR: false,
    ALLOW_DATA_ATTR: false,
    RETURN_DOM_FRAGMENT: true,
  });
  for (const image of fragment.querySelectorAll("img")) {
    image.replaceWith(document.createTextNode(` ${image.getAttribute("alt") ?? ""} `));
  }
  for (const element of fragment.querySelectorAll("br, p, div, li, tr, td, th"))
    element.appendChild(document.createTextNode(" "));
  return (fragment.textContent ?? "").replace(/\s+/gu, " ").trim();
}
