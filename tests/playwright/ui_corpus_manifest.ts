// ui_corpus_manifest.ts - stable browser-context policy for visual UI corpus checks.

import type { BrowserContextOptions } from "@playwright/test";

/** Required accessibility preferences for Ribbon visual and interaction checks. */
export const RIBBON_PLAYWRIGHT_CONTEXT_OPTIONS = {
  forcedColors: "active",
  reducedMotion: "reduce",
} satisfies Pick<BrowserContextOptions, "forcedColors" | "reducedMotion">;

export type RibbonResponsiveProfileId = "instructor_desktop" | "portrait_tablet" | "narrow_phone";

export interface RibbonResponsiveProfile {
  readonly id: RibbonResponsiveProfileId;
  readonly contextOptions: BrowserContextOptions;
}

/**
 * Stable evidence profiles. Touch-capable contexts exercise the Ribbon's
 * coarse-pointer profile while preserving forced-colors and reduced-motion
 * evidence in every viewport.
 */
export const RIBBON_RESPONSIVE_PROFILES: ReadonlyArray<RibbonResponsiveProfile> = [
  {
    id: "instructor_desktop",
    contextOptions: {
      ...RIBBON_PLAYWRIGHT_CONTEXT_OPTIONS,
      viewport: { width: 1280, height: 800 },
    },
  },
  {
    id: "portrait_tablet",
    contextOptions: {
      ...RIBBON_PLAYWRIGHT_CONTEXT_OPTIONS,
      hasTouch: true,
      isMobile: true,
      viewport: { width: 768, height: 1024 },
    },
  },
  {
    id: "narrow_phone",
    contextOptions: {
      ...RIBBON_PLAYWRIGHT_CONTEXT_OPTIONS,
      hasTouch: true,
      isMobile: true,
      viewport: { width: 320, height: 640 },
    },
  },
];
