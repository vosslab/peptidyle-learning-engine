// Browser font URLs are declared by @font-face CSS and served from this root.

export const BROWSER_FONT_ROOT = "/assets/fonts/";

const FONT_PATH_SEGMENT = /^[a-z0-9][a-z0-9_.-]*$/u;
const FONT_URL_PATTERN = /url\(\s*(["']?)([^"'()\s]+)\1\s*\)/gu;

/**
 * Returns the validated root-relative font URLs declared in @font-face CSS.
 *
 * @param {string} stylesheet emitted or compiled CSS
 * @returns {string[]} ordered unique local font asset URLs
 */
export function browserFontUrlsFromStylesheet(stylesheet) {
  const fontFaceBlocks = stylesheet.match(/@font-face\s*\{[^}]*\}/gu) ?? [];
  const urls = new Set();
  for (const block of fontFaceBlocks) {
    const faceUrls = [...block.matchAll(FONT_URL_PATTERN)];
    if (faceUrls.length === 0) {
      throw new Error("Each @font-face must declare a local .woff2 font URL.");
    }
    for (const match of faceUrls) {
      const url = match[2];
      if (url === undefined) continue;
      urls.add(validateBrowserFontUrl(url));
    }
  }
  return [...urls];
}

/**
 * Converts a declared font URL to its safe path below src/assets/fonts/.
 *
 * @param {string} url CSS @font-face URL
 * @returns {string} path relative to assets/fonts
 */
export function browserFontPathFromUrl(url) {
  return validateBrowserFontUrl(url).slice(BROWSER_FONT_ROOT.length);
}

function validateBrowserFontUrl(url) {
  // ASVS 2.2.1 and V5.3.2: validate and contain CSS-derived paths before file I/O.
  if (
    !url.startsWith(BROWSER_FONT_ROOT) ||
    url.includes("?") ||
    url.includes("#") ||
    url.includes("\\")
  ) {
    throw new Error(`@font-face URL must be a root-relative ${BROWSER_FONT_ROOT} path: ${url}`);
  }
  const relativePath = url.slice(BROWSER_FONT_ROOT.length);
  const segments = relativePath.split("/");
  if (segments.length < 2 || segments.some((segment) => !FONT_PATH_SEGMENT.test(segment))) {
    throw new Error(`@font-face URL contains an unsafe font path: ${url}`);
  }
  if (!relativePath.endsWith(".woff2")) {
    throw new Error(`@font-face URL must name a .woff2 font file: ${url}`);
  }
  return url;
}
