// Theme registry and Course Appearance decoder contracts.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { THEME_VALUES } from "../generated/api/Theme.ts";
import { decodeCourseAppearanceView } from "../src/api/decoders.ts";
import { resolveDisplayMode, resolveTheme } from "../src/appearance/appearance_rules.ts";
import { THEME_REGISTRY, themeStyle, themeTokens } from "../src/appearance/theme_registry.ts";
import { courseBannerImageAlternativeText } from "../src/features/course_appearance/course_banner_alternative_text.ts";

function relativeLuminance(hex) {
  const channel = (offset) => {
    const normalized = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return normalized <= 0.03928 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

function contrastRatio(foreground, background) {
  const lighter = Math.max(relativeLuminance(foreground), relativeLuminance(background));
  const darker = Math.min(relativeLuminance(foreground), relativeLuminance(background));
  return (lighter + 0.05) / (darker + 0.05);
}

function assertContrast(foreground, background, minimum, label) {
  assert.ok(
    contrastRatio(foreground, background) >= minimum,
    `${label}: ${contrastRatio(foreground, background).toFixed(2)}:1 is below ${minimum}:1`,
  );
}

test("every Theme look has readable text and visible focus or control boundaries", () => {
  for (const theme of THEME_VALUES) {
    for (const mode of ["light", "dark"]) {
      const tokens = themeTokens(theme, mode);
      const { palette } = tokens;
      const label = `${theme}/${mode}`;

      for (const [backgroundName, background] of Object.entries({
        canvas: palette.canvas,
        surface: palette.surface,
        secondary: palette.secondary,
        highlight: palette.highlight,
      })) {
        assertContrast(tokens.ink, background, 4.5, `${label} normal text on ${backgroundName}`);
        assertContrast(tokens.muted, background, 4.5, `${label} muted text on ${backgroundName}`);
        assertContrast(tokens.link, background, 4.5, `${label} link on ${backgroundName}`);
      }

      assertContrast(tokens.onAction, tokens.action, 4.5, `${label} text on action`);
      assertContrast(tokens.onAction, tokens.actionHover, 4.5, `${label} text on action hover`);
      assertContrast(tokens.onSecondary, palette.secondary, 4.5, `${label} text on secondary`);
      assertContrast(tokens.onHighlight, palette.highlight, 4.5, `${label} text on highlight`);
      assertContrast(tokens.onInk, tokens.ink, 4.5, `${label} text on ink`);
      assertContrast(tokens.focus, palette.canvas, 3, `${label} focus boundary on canvas`);
      assertContrast(tokens.focus, palette.surface, 3, `${label} focus boundary on surface`);
      assertContrast(tokens.borderStrong, palette.canvas, 3, `${label} control boundary on canvas`);
      assertContrast(
        tokens.borderStrong,
        palette.surface,
        3,
        `${label} control boundary on surface`,
      );
    }
  }
});

test("unknown Theme IDs fail closed", () => {
  assert.throws(() => themeTokens("woodland"), /Unknown theme/u);
  assert.throws(() => decodeCourseAppearanceView({ theme: "woodland", banner: null }));
});

test("course banners preserve their closed decorative or informative treatment", () => {
  const bannerId = "00000000-0000-0000-0000-000000000007";
  const decorative = decodeCourseAppearanceView({
    theme: "grass",
    banner: { id: bannerId, alternativeText: { kind: "decorative" } },
  });
  const informative = decodeCourseAppearanceView({
    theme: "grass",
    banner: { id: bannerId, alternativeText: { kind: "informative", text: "Forest canopy" } },
  });
  assert.equal(courseBannerImageAlternativeText(decorative.banner.alternativeText), "");
  assert.equal(
    courseBannerImageAlternativeText(informative.banner.alternativeText),
    "Forest canopy",
  );
});

function biomeCatalogRows() {
  const text = readFileSync(new URL("../docs/BIOME_THEME_PALETTES.md", import.meta.url), "utf8");
  const pattern =
    /^\| `([^`]+)` \| ([^|]+?) \| ((?:`#[0-9a-f]{6}` \/ ){4}`#[0-9a-f]{6}`) \| ((?:`#[0-9a-f]{6}` \/ ){4}`#[0-9a-f]{6}`) \|$/u;
  const rows = [];
  for (const line of text.split("\n")) {
    const match = pattern.exec(line);
    if (match === null) continue;
    const colors = (cell) => [...cell.matchAll(/#[0-9a-f]{6}/gu)].map((item) => item[0]);
    rows.push({
      id: match[1],
      name: match[2].trim(),
      light: colors(match[3]),
      dark: colors(match[4]),
    });
  }
  return rows;
}

test("the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces", () => {
  const rows = biomeCatalogRows();
  assert.deepEqual(
    rows.map((row) => row.id),
    [...THEME_VALUES],
  );
  const lightCanvases = new Set();
  const darkCanvases = new Set();
  for (const row of rows) {
    const definition = THEME_REGISTRY[row.id];
    assert.equal(definition.name, row.name);
    for (const mode of ["light", "dark"]) {
      const palette = definition[mode];
      assert.deepEqual(
        [palette.canvas, palette.surface, palette.secondary, palette.accent, palette.highlight],
        row[mode],
      );
      const luminance = relativeLuminance(palette.canvas);
      if (mode === "light") {
        assert.ok(luminance >= 0.6, `${row.id} light canvas is ${luminance}`);
        lightCanvases.add(palette.canvas);
      } else {
        assert.ok(luminance <= 0.08, `${row.id} dark canvas is ${luminance}`);
        darkCanvases.add(palette.canvas);
      }
    }
    const style = themeStyle(themeTokens(row.id, "dark"));
    assert.match(style, new RegExp(`--ple-page-background: ${definition.dark.canvas}`, "u"));
    assert.match(style, new RegExp(`--ple-surface: ${definition.dark.surface}`, "u"));
    assert.match(style, new RegExp(`--ple-surface-soft: ${definition.dark.secondary}`, "u"));
    assert.match(style, new RegExp(`--ple-highlight: ${definition.dark.highlight}`, "u"));
    assert.match(style, new RegExp(`--ple-accent: ${definition.dark.accent}`, "u"));
  }
  assert.equal(lightCanvases.size, rows.length);
  assert.equal(darkCanvases.size, rows.length);
  assert.equal(THEME_REGISTRY.grass.name, "Grassland");
  assert.equal(
    resolveTheme({
      courseTheme: "ocean",
      instructorPersonalTheme: "forest",
      signedInInstructor: true,
    }),
    "ocean",
  );
  assert.equal(
    resolveTheme({ instructorPersonalTheme: "forest", signedInInstructor: true }),
    "forest",
  );
  assert.equal(resolveDisplayMode("dark", "light"), "dark");
  assert.equal(resolveDisplayMode(null, "light"), "light");
});
