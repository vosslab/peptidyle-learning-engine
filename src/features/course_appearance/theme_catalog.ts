// theme_catalog.ts - exhaustive course-theme anchors and accessible design tokens.

import type { CourseThemeId } from "../../../generated/api/CourseThemeId";

export interface ThemeAnchors {
  readonly canvas: string;
  readonly secondary: string;
  readonly accent: string;
}

export interface CourseThemeTokens {
  readonly name: string;
  readonly anchors: ThemeAnchors;
  readonly ink: string;
  readonly muted: string;
  readonly surface: string;
  readonly surfaceSoft: string;
  readonly action: string;
  readonly actionHover: string;
  readonly onAction: string;
  readonly link: string;
  readonly focus: string;
  readonly border: string;
}

const SHARED_TOKENS = {
  ink: "#231f20",
  muted: "#414142",
  surface: "#ffffff",
  onAction: "#ffffff",
  actionHover: "#231f20",
  focus: "#414142",
  border: "#686868",
} as const;

function theme(
  name: string,
  anchors: ThemeAnchors,
  action: string,
  link = action,
): CourseThemeTokens {
  return {
    name,
    anchors,
    ...SHARED_TOKENS,
    surfaceSoft: anchors.canvas,
    action,
    link,
  };
}

/**
 * The closed browser registry for the Rust-owned theme union.
 *
 * Raw anchors are decorative design inputs. `action` and `link` are darker
 * derived colors where a raw brand or habitat anchor cannot carry normal text
 * at the project's 5.5:1 target.
 */
export const COURSE_THEME_CATALOG = {
  tundra: theme(
    "Tundra",
    { canvas: "#e3e1da", secondary: "#725e72", accent: "#485b3c" },
    "#725e72",
    "#635263",
  ),
  forest: theme(
    "Forest",
    { canvas: "#e4ebdd", secondary: "#166747", accent: "#aa831a" },
    "#166747",
  ),
  desert: theme(
    "Desert",
    { canvas: "#f3e2bd", secondary: "#c07a3b", accent: "#68402a" },
    "#68402a",
  ),
  grass: theme(
    "Grass",
    { canvas: "#bddeb1", secondary: "#73c167", accent: "#008852" },
    "#006b40",
    "#005c38",
  ),
  arctic: theme(
    "Arctic",
    { canvas: "#e5f5f8", secondary: "#7cbed1", accent: "#1f5d78" },
    "#1f5d78",
  ),
  ocean: theme("Ocean", { canvas: "#ddeff5", secondary: "#0b6c88", accent: "#123c69" }, "#123c69"),
  tropical: theme(
    "Tropical",
    { canvas: "#e4f2d6", secondary: "#1b7646", accent: "#8a1976" },
    "#1b7646",
    "#196c40",
  ),
  "coral-reef": theme(
    "Coral reef",
    { canvas: "#e8f6f1", secondary: "#006d68", accent: "#b52d3d" },
    "#006d68",
  ),
  swamp: theme("Swamp", { canvas: "#e8e5c9", secondary: "#4e5f23", accent: "#4b3426" }, "#4e5f23"),
  underground: theme(
    "Underground",
    { canvas: "#e6e0d8", secondary: "#59504a", accent: "#c9732c" },
    "#59504a",
  ),
  "salt-marsh": theme(
    "Salt marsh",
    { canvas: "#e8f0df", secondary: "#1e6a6d", accent: "#76511f" },
    "#1e6a6d",
    "#1e686b",
  ),
  wetland: theme(
    "Wetland",
    { canvas: "#e4eee7", secondary: "#466f59", accent: "#3b648c" },
    "#466f59",
    "#406551",
  ),
  "sea-floor": theme(
    "Sea floor",
    { canvas: "#dee8ed", secondary: "#344e62", accent: "#086a72" },
    "#344e62",
  ),
  magma: theme(
    "Magma",
    { canvas: "#f5e0cf", secondary: "#a92720", accent: "#3b2928" },
    "#a92720",
    "#a82720",
  ),
  beach: theme("Beach", { canvas: "#f3e7c9", secondary: "#56a8b0", accent: "#8a3d24" }, "#8a3d24"),
} as const satisfies Readonly<Record<CourseThemeId, CourseThemeTokens>>;

export interface CourseThemeOption {
  readonly id: CourseThemeId;
  readonly tokens: CourseThemeTokens;
}

/** Theme choices in the Rust-owned catalog order used by the native radio group. */
export const COURSE_THEME_OPTIONS: ReadonlyArray<CourseThemeOption> = Object.entries(
  COURSE_THEME_CATALOG,
).map(([id, tokens]) => ({ id: id as CourseThemeId, tokens }));

/** Resolves one reviewed theme and rejects transport drift without a fallback. */
export function courseThemeTokens(themeId: CourseThemeId): CourseThemeTokens {
  const candidate: CourseThemeTokens | undefined = COURSE_THEME_CATALOG[themeId];
  if (candidate === undefined) {
    throw new Error(`Unknown course theme: ${String(themeId)}`);
  }
  return candidate;
}

/** Serializes only catalog-owned constants into a course-local style attribute. */
export function courseThemeStyle(tokens: CourseThemeTokens): string {
  return [
    `--ple-theme-canvas: ${tokens.anchors.canvas}`,
    `--ple-theme-secondary: ${tokens.anchors.secondary}`,
    `--ple-theme-accent: ${tokens.anchors.accent}`,
    `--ple-surface: ${tokens.surface}`,
    `--ple-surface-soft: ${tokens.surfaceSoft}`,
    `--ple-card-surface: ${tokens.surface}`,
    `--ple-ink: ${tokens.ink}`,
    `--ple-muted: ${tokens.muted}`,
    `--ple-accent: ${tokens.action}`,
    `--ple-accent-strong: ${tokens.link}`,
    `--ple-action-hover: ${tokens.actionHover}`,
    `--ple-focus: ${tokens.focus}`,
    `--ple-border: ${tokens.border}`,
  ].join("; ");
}
