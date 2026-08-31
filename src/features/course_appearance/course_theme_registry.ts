// course_theme_registry.ts - exhaustive course-theme anchors and accessible design tokens.

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
  readonly card: string;
  readonly action: string;
  readonly actionHover: string;
  readonly onAction: string;
  readonly onSecondary: string;
  readonly link: string;
  readonly focus: string;
  readonly border: string;
}

interface CourseThemeDefinition {
  readonly name: string;
  readonly anchors: ThemeAnchors;
  readonly action: string;
  readonly onSecondary: string;
  readonly ink?: string;
  readonly muted?: string;
  readonly link?: string;
}

/* Theme projection recipe. These percentages are the single tuning point for
 * normal presentation; the stored three-color palettes remain unchanged. */
const THEME_MIX = {
  surfaceCanvas: 58,
  softSurfaceCanvas: 78,
  cardCanvas: 30,
  inkAction: 22,
  mutedAction: 10,
  linkAction: 57,
  hoverAction: 94,
  borderAction: 24,
} as const;

function mix(first: string, firstShare: number, second: string): string {
  return `color-mix(in srgb, ${first} ${String(firstShare)}%, ${second})`;
}

function theme(definition: CourseThemeDefinition): CourseThemeTokens {
  const { action, anchors } = definition;
  return {
    name: definition.name,
    anchors,
    ink: definition.ink ?? mix(action, THEME_MIX.inkAction, "#485260"),
    muted: definition.muted ?? mix(action, THEME_MIX.mutedAction, "#4c5663"),
    surface: mix(anchors.canvas, THEME_MIX.surfaceCanvas, "white"),
    surfaceSoft: mix(anchors.canvas, THEME_MIX.softSurfaceCanvas, "white"),
    card: mix(anchors.canvas, THEME_MIX.cardCanvas, "white"),
    action,
    actionHover: mix(action, THEME_MIX.hoverAction, "#172033"),
    onAction: "#ffffff",
    onSecondary: definition.onSecondary,
    link: definition.link ?? mix(action, THEME_MIX.linkAction, "#485260"),
    focus: action,
    border: mix(action, THEME_MIX.borderAction, anchors.canvas),
  };
}

/**
 * The closed browser registry for the Rust-owned theme union.
 *
 * Raw anchors are decorative design inputs. `action` and `link` are darker
 * derived colors where a raw brand or habitat anchor cannot carry normal text
 * at the project's 5.5:1 target.
 */
export const COURSE_THEME_REGISTRY = {
  tundra: theme({
    name: "Tundra",
    anchors: { canvas: "#e3e1da", secondary: "#725e72", accent: "#485b3c" },
    action: "#725e72",
    onSecondary: "#ffffff",
    link: "#635263",
  }),
  forest: theme({
    name: "Forest",
    anchors: { canvas: "#e4ebdd", secondary: "#166747", accent: "#aa831a" },
    action: "#166747",
    onSecondary: "#ffffff",
  }),
  desert: theme({
    name: "Desert",
    anchors: { canvas: "#f3e2bd", secondary: "#c07a3b", accent: "#68402a" },
    action: "#68402a",
    onSecondary: "#000000",
  }),
  grass: theme({
    name: "Grass",
    anchors: { canvas: "#bddeb1", secondary: "#73c167", accent: "#008852" },
    action: "#006b40",
    onSecondary: "#172033",
    ink: "#315047",
    muted: "#36535a",
    link: "#005c38",
  }),
  arctic: theme({
    name: "Arctic",
    anchors: { canvas: "#e5f5f8", secondary: "#7cbed1", accent: "#1f5d78" },
    action: "#1f5d78",
    onSecondary: "#172033",
  }),
  ocean: theme({
    name: "Ocean",
    anchors: { canvas: "#ddeff5", secondary: "#0b6c88", accent: "#123c69" },
    action: "#0a627b",
    onSecondary: "#ffffff",
  }),
  tropical: theme({
    name: "Tropical",
    anchors: { canvas: "#e4f2d6", secondary: "#1b7646", accent: "#8a1976" },
    action: "#1b7646",
    onSecondary: "#ffffff",
  }),
  "coral-reef": theme({
    name: "Coral reef",
    anchors: { canvas: "#e8f6f1", secondary: "#006d68", accent: "#b52d3d" },
    action: "#006d68",
    onSecondary: "#ffffff",
  }),
  swamp: theme({
    name: "Swamp",
    anchors: { canvas: "#e8e5c9", secondary: "#4e5f23", accent: "#4b3426" },
    action: "#4e5f23",
    onSecondary: "#ffffff",
  }),
  underground: theme({
    name: "Underground",
    anchors: { canvas: "#e6e0d8", secondary: "#59504a", accent: "#c9732c" },
    action: "#59504a",
    onSecondary: "#ffffff",
  }),
  "salt-marsh": theme({
    name: "Salt marsh",
    anchors: { canvas: "#e8f0df", secondary: "#1e6a6d", accent: "#76511f" },
    action: "#1e6a6d",
    onSecondary: "#ffffff",
  }),
  wetland: theme({
    name: "Wetland",
    anchors: { canvas: "#e4eee7", secondary: "#466f59", accent: "#3b648c" },
    action: "#466f59",
    onSecondary: "#ffffff",
  }),
  "sea-floor": theme({
    name: "Sea floor",
    anchors: { canvas: "#dee8ed", secondary: "#344e62", accent: "#086a72" },
    action: "#344e62",
    onSecondary: "#ffffff",
  }),
  magma: theme({
    name: "Magma",
    anchors: { canvas: "#f5e0cf", secondary: "#a92720", accent: "#3b2928" },
    action: "#a92720",
    onSecondary: "#ffffff",
  }),
  beach: theme({
    name: "Beach",
    anchors: { canvas: "#f3e7c9", secondary: "#56a8b0", accent: "#8a3d24" },
    action: "#8a3d24",
    onSecondary: "#172033",
  }),
} as const satisfies Readonly<Record<CourseThemeId, CourseThemeTokens>>;

export interface CourseThemeOption {
  readonly id: CourseThemeId;
  readonly tokens: CourseThemeTokens;
}

/** Theme choices in the Rust-owned registry order used by the native radio group. */
export const COURSE_THEME_OPTIONS: ReadonlyArray<CourseThemeOption> = Object.entries(
  COURSE_THEME_REGISTRY,
).map(([id, tokens]) => ({ id: id as CourseThemeId, tokens }));

/** Resolves one reviewed theme and rejects transport drift without a fallback. */
export function courseThemeTokens(themeId: CourseThemeId): CourseThemeTokens {
  const candidate: CourseThemeTokens | undefined = COURSE_THEME_REGISTRY[themeId];
  if (candidate === undefined) {
    throw new Error(`Unknown course theme: ${String(themeId)}`);
  }
  return candidate;
}

/** Serializes only registry-owned constants into a course-local style attribute. */
export function courseThemeStyle(tokens: CourseThemeTokens): string {
  return [
    `--ple-theme-canvas: ${tokens.anchors.canvas}`,
    `--ple-theme-secondary: ${tokens.anchors.secondary}`,
    `--ple-theme-accent: ${tokens.anchors.accent}`,
    `--ple-surface: ${tokens.surface}`,
    `--ple-surface-soft: ${tokens.surfaceSoft}`,
    `--ple-card-surface: ${tokens.card}`,
    `--ple-ink: ${tokens.ink}`,
    `--ple-muted: ${tokens.muted}`,
    `--ple-accent: ${tokens.action}`,
    `--ple-accent-strong: ${tokens.link}`,
    `--ple-action-hover: ${tokens.actionHover}`,
    `--ple-on-action: ${tokens.onAction}`,
    `--ple-theme-on-secondary: ${tokens.onSecondary}`,
    `--ple-focus: ${tokens.focus}`,
    `--ple-border: ${tokens.border}`,
  ].join("; ");
}
