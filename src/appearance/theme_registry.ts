// theme_registry.ts - the shared, closed Theme palette registry.

import type { DisplayMode } from "../../generated/api/DisplayMode";
import type { Theme } from "../../generated/api/Theme";

export interface ThemePalette {
  readonly canvas: string;
  readonly surface: string;
  readonly secondary: string;
  readonly accent: string;
  readonly highlight: string;
}

export interface ThemeTokens {
  readonly name: string;
  readonly palette: ThemePalette;
  readonly ink: string;
  readonly muted: string;
  readonly action: string;
  readonly actionHover: string;
  readonly onAction: string;
  readonly onSecondary: string;
  readonly onHighlight: string;
  readonly onInk: string;
  readonly link: string;
  readonly focus: string;
  readonly border: string;
  readonly borderStrong: string;
}

export interface ThemeDefinition {
  readonly name: string;
  readonly light: ThemePalette;
  readonly dark: ThemePalette;
}

function palette(
  canvas: string,
  surface: string,
  secondary: string,
  accent: string,
  highlight: string,
): ThemePalette {
  return { canvas, surface, secondary, accent, highlight };
}

function rgb(hex: string): readonly [number, number, number] {
  return [
    Number.parseInt(hex.slice(1, 3), 16),
    Number.parseInt(hex.slice(3, 5), 16),
    Number.parseInt(hex.slice(5, 7), 16),
  ];
}

function mix(first: string, firstShare: number, second: string): string {
  const left = rgb(first);
  const right = rgb(second);
  const channel = (index: 0 | 1 | 2): string =>
    Math.round(left[index] * firstShare + right[index] * (1 - firstShare))
      .toString(16)
      .padStart(2, "0");
  return `#${channel(0)}${channel(1)}${channel(2)}`;
}

function luminance(value: string): number {
  const channelLuminance = (channel: number): number => {
    const normalized = channel / 255;
    return normalized <= 0.03928 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
  };
  const [red, green, blue] = rgb(value);
  return (
    0.2126 * channelLuminance(red) +
    0.7152 * channelLuminance(green) +
    0.0722 * channelLuminance(blue)
  );
}

function readableText(background: string): string {
  const darkInk = "#172033";
  const white = "#ffffff";
  const contrast = (foreground: string): number => {
    const backgroundLuminance = luminance(background);
    const foregroundLuminance = luminance(foreground);
    return (
      (Math.max(backgroundLuminance, foregroundLuminance) + 0.05) /
      (Math.min(backgroundLuminance, foregroundLuminance) + 0.05)
    );
  };
  return contrast(darkInk) >= contrast(white) ? darkInk : white;
}

/** The single derivation used by every theme and display mode. */
export function deriveThemeTokens(definition: ThemeDefinition, mode: DisplayMode): ThemeTokens {
  const paletteValue = definition[mode];
  const ink = readableText(paletteValue.canvas);
  const onAction = readableText(paletteValue.accent);
  const actionHoverTarget = onAction === "#ffffff" ? "#172033" : "#ffffff";
  const link = mix(paletteValue.accent, 0.55, ink);
  const focus = mix(paletteValue.accent, 0.72, ink);
  return {
    name: definition.name,
    palette: paletteValue,
    ink,
    muted: mix(ink, 0.94, paletteValue.canvas),
    action: paletteValue.accent,
    actionHover: mix(paletteValue.accent, 0.84, actionHoverTarget),
    onAction,
    onSecondary: readableText(paletteValue.secondary),
    onHighlight: readableText(paletteValue.highlight),
    onInk: readableText(ink),
    link,
    focus,
    border: mix(paletteValue.secondary, 0.28, paletteValue.canvas),
    borderStrong: focus,
  };
}

/** Five source colors for each Theme look. */
export const THEME_REGISTRY = {
  tundra: {
    name: "Tundra",
    light: palette("#d8d0dc", "#eee8ef", "#d5c5d9", "#485b3c", "#e5d7e6"),
    dark: palette("#24212a", "#332d39", "#413447", "#93ad83", "#493b50"),
  },
  forest: {
    name: "Forest",
    light: palette("#d8eadc", "#f4fbf4", "#c2e3c8", "#17643b", "#b8e3a5"),
    dark: palette("#102319", "#183226", "#254a36", "#a7e39d", "#315c3d"),
  },
  desert: {
    name: "Desert",
    light: palette("#f7e6c5", "#fff9ee", "#eacb8e", "#744117", "#f3d9a7"),
    dark: palette("#261c10", "#382816", "#59401e", "#ffd18a", "#6a4b20"),
  },
  grass: {
    name: "Grassland",
    light: palette("#cfe0a6", "#e7f0ca", "#d8e7b5", "#008852", "#edf4d1"),
    dark: palette("#202919", "#2e3b25", "#394d2d", "#55bd88", "#465b38"),
  },
  arctic: {
    name: "Arctic",
    light: palette("#dceff5", "#f6fcfe", "#b9dce8", "#1d5e78", "#c6e8f5"),
    dark: palette("#10232a", "#18343e", "#27505d", "#9adcf2", "#315f70"),
  },
  ocean: {
    name: "Ocean",
    light: palette("#c6e1ef", "#e0f0f6", "#cfe6f0", "#123c69", "#d8ecf4"),
    dark: palette("#172b36", "#233e4a", "#2c5362", "#76a9dd", "#315565"),
  },
  tropical: {
    name: "Tropical",
    light: palette("#d5e6ae", "#edf4ce", "#ddefbb", "#8a1976", "#e6c6df"),
    dark: palette("#251f2b", "#372b3a", "#433348", "#cf76bd", "#513b50"),
  },
  "coral-reef": {
    name: "Coral reef",
    light: palette("#c5e7df", "#dcf3ed", "#cfece4", "#b52d3d", "#f0cbd0"),
    dark: palette("#1d2b2a", "#2a3d3a", "#344c49", "#e97883", "#4d393d"),
  },
  swamp: {
    name: "Swamp",
    light: palette("#d8d5ac", "#eeeac4", "#e2deb6", "#4b3426", "#e7cdb2"),
    dark: palette("#2d2b1f", "#403d28", "#514c31", "#b89474", "#5a4635"),
  },
  underground: {
    name: "Underground",
    light: palette("#dbd1c5", "#eee5da", "#e3d8cb", "#c9732c", "#efd1b8"),
    dark: palette("#29231f", "#3c312a", "#4c3c32", "#e99b59", "#594238"),
  },
  "salt-marsh": {
    name: "Salt marsh",
    light: palette("#ccdcca", "#e4eee1", "#d5e5d4", "#76511f", "#d8e8d8"),
    dark: palette("#202c28", "#2e4039", "#385149", "#c49a61", "#40584f"),
  },
  wetland: {
    name: "Wetland",
    light: palette("#d1e1c8", "#e7efdf", "#dae8d4", "#3b648c", "#d0dfeb"),
    dark: palette("#202b24", "#2e4034", "#394f40", "#7da7d0", "#40546a"),
  },
  "sea-floor": {
    name: "Sea floor",
    light: palette("#cbd8df", "#e0e9ed", "#d5e1e7", "#086a72", "#cae4e4"),
    dark: palette("#1c2b31", "#2a3e46", "#354f59", "#58adb2", "#3b5960"),
  },
  magma: {
    name: "Magma",
    light: palette("#f6ded6", "#fff8f5", "#f0b9a9", "#7a261f", "#f1c3a4"),
    dark: palette("#281514", "#3a201e", "#5d302b", "#ffae8e", "#733a32"),
  },
  beach: {
    name: "Beach",
    light: palette("#e7d7ab", "#f3e7c6", "#eddfb9", "#8a3d24", "#cae8e3"),
    dark: palette("#2d291e", "#403929", "#514730", "#d38568", "#365451"),
  },
} as const satisfies Readonly<Record<Theme, ThemeDefinition>>;

export interface ThemeOption {
  readonly id: Theme;
  readonly definition: ThemeDefinition;
}

export const THEME_OPTIONS: ReadonlyArray<ThemeOption> = Object.entries(THEME_REGISTRY).map(
  ([id, definition]) => ({ id: id as Theme, definition }),
);

export function themeDefinition(themeId: Theme): ThemeDefinition {
  const definition = THEME_REGISTRY[themeId];
  if (definition === undefined) throw new Error(`Unknown theme: ${String(themeId)}`);
  return definition;
}

export function themeTokens(themeId: Theme, mode: DisplayMode = "light"): ThemeTokens {
  return deriveThemeTokens(themeDefinition(themeId), mode);
}

/** Serializes registry-owned concrete tokens for existing scoped consumers. */
export function themeStyle(tokens: ThemeTokens): string {
  const { palette: colors } = tokens;
  return [
    `--ple-theme-canvas: ${colors.canvas}`,
    `--ple-theme-surface: ${colors.surface}`,
    `--ple-theme-secondary: ${colors.secondary}`,
    `--ple-theme-accent: ${colors.accent}`,
    `--ple-page-background: ${colors.canvas}`,
    `--ple-surface: ${colors.surface}`,
    `--ple-surface-soft: ${colors.secondary}`,
    `--ple-card-surface: ${colors.surface}`,
    `--ple-highlight: ${colors.highlight}`,
    `--ple-ink: ${tokens.ink}`,
    `--ple-text: ${tokens.ink}`,
    `--ple-muted: ${tokens.muted}`,
    `--ple-accent: ${tokens.action}`,
    `--ple-theme-primary: ${tokens.action}`,
    `--ple-accent-strong: ${tokens.link}`,
    `--ple-action-hover: ${tokens.actionHover}`,
    `--ple-on-action: ${tokens.onAction}`,
    `--ple-theme-on-secondary: ${tokens.onSecondary}`,
    `--ple-on-highlight: ${tokens.onHighlight}`,
    `--ple-on-ink: ${tokens.onInk}`,
    `--ple-focus: ${tokens.focus}`,
    `--ple-border: ${tokens.border}`,
    `--ple-border-strong: ${tokens.borderStrong}`,
  ].join("; ");
}
