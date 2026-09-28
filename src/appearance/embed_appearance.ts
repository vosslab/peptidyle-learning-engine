// embed_appearance.ts - closed cosmetic appearance record for isolated Question documents.

import type { DisplayMode } from "../../generated/api/DisplayMode";
import type { ResolvedAppearance } from "./appearance_context";

export const EMBED_APPEARANCE_KIND = "ple.embed.appearance";
export const EMBED_APPEARANCE_VERSION = 1;
const EMBED_APPEARANCE_PREFIX = `${EMBED_APPEARANCE_KIND}:`;

/**
 * The only colors PLE supplies to a Question document. These carry no account,
 * Course, Question, or response data and map to a fixed receiver property list.
 */
export interface EmbedAppearanceColors {
  readonly background: string;
  readonly foreground: string;
  readonly surface: string;
  readonly secondary: string;
  readonly accent: string;
  readonly highlight: string;
  readonly muted: string;
  readonly border: string;
  readonly onAccent: string;
  readonly link: string;
}

export interface EmbedAppearanceMessage {
  readonly kind: typeof EMBED_APPEARANCE_KIND;
  readonly version: typeof EMBED_APPEARANCE_VERSION;
  readonly mode: DisplayMode;
  readonly colors: EmbedAppearanceColors;
}

/** Builds the closed cosmetic record from the document owner's resolved appearance. */
export function embedAppearanceMessage(appearance: ResolvedAppearance): EmbedAppearanceMessage {
  const { palette } = appearance.tokens;
  return Object.freeze({
    kind: EMBED_APPEARANCE_KIND,
    version: EMBED_APPEARANCE_VERSION,
    mode: appearance.mode,
    colors: Object.freeze({
      background: palette.canvas,
      foreground: appearance.tokens.ink,
      surface: palette.surface,
      secondary: palette.secondary,
      accent: appearance.tokens.action,
      highlight: palette.highlight,
      muted: appearance.tokens.muted,
      // Form controls consume the contrast-qualified strong border; `accent`
      // remains the action color for backend-owned controls.
      border: appearance.tokens.borderStrong,
      onAccent: appearance.tokens.onAction,
      link: appearance.tokens.link,
    }),
  });
}

/** Posts only the exact appearance record to this iframe's current document. */
export function postEmbedAppearance(
  frame: HTMLIFrameElement | undefined,
  appearance: ResolvedAppearance,
  targetOrigin: string,
): void {
  // Some Question renderers inspect every message as a string. Keep the
  // closed cosmetic record encoded behind a prefix so it cannot disrupt their
  // legacy message handlers.
  frame?.contentWindow?.postMessage(
    `${EMBED_APPEARANCE_PREFIX}${JSON.stringify(embedAppearanceMessage(appearance))}`,
    targetOrigin,
  );
}
