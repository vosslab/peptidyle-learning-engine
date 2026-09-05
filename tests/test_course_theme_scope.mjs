// Course Appearance browser registry and route-scope tests.

import assert from "node:assert/strict";
import test from "node:test";

import { decodeCourseAppearanceView } from "../src/api/decoders.ts";
import {
  courseBannerImageAlternativeText, // Decode the closed banner accessibility union.
} from "../src/features/course_appearance/course_banner_alternative_text.ts";
import {
  COURSE_THEME_REGISTRY,
  courseThemeStyle,
  courseThemeTokens,
} from "../src/features/course_appearance/course_theme_registry.ts";

const THEME_IDS = [
  "tundra",
  "forest",
  "desert",
  "grass",
  "arctic",
  "ocean",
  "tropical",
  "coral-reef",
  "swamp",
  "underground",
  "salt-marsh",
  "wetland",
  "sea-floor",
  "magma",
  "beach",
];

function mixedHex(first, second, firstShare) {
  const channels = (hex) =>
    hex
      .slice(1)
      .match(/.{2}/gu)
      .map((channel) => Number.parseInt(channel, 16));
  const left = channels(first);
  const right = channels(second === "white" ? "#ffffff" : second);
  return `#${left
    .map((channel, index) =>
      Math.round(channel * firstShare + right[index] * (1 - firstShare))
        .toString(16)
        .padStart(2, "0"),
    )
    .join("")}`;
}

function resolvedHex(color) {
  if (/^#[0-9a-f]{6}$/u.test(color)) return color;
  const match = /^color-mix\(in srgb, (#[0-9a-f]{6}) ([0-9]+)%, (#[0-9a-f]{6}|white)\)$/u.exec(
    color,
  );
  assert.notEqual(match, null, `unsupported reviewed color token: ${color}`);
  return mixedHex(match[1], match[3], Number(match[2]) / 100);
}

function relativeLuminance(color) {
  const hex = resolvedHex(color);
  const channels = hex
    .slice(1)
    .match(/.{2}/gu)
    .map((channel) => Number.parseInt(channel, 16) / 255)
    .map((channel) => (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4));
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrast(first, second) {
  const lighter = Math.max(relativeLuminance(first), relativeLuminance(second));
  const darker = Math.min(relativeLuminance(first), relativeLuminance(second));
  return (lighter + 0.05) / (darker + 0.05);
}

test("every reviewed theme resolves to complete, contrast-safe course tokens", () => {
  assert.deepEqual(Object.keys(COURSE_THEME_REGISTRY), THEME_IDS);
  for (const id of THEME_IDS) {
    const tokens = courseThemeTokens(id);
    const textPairs = [
      [tokens.ink, tokens.anchors.canvas],
      [tokens.ink, tokens.surface],
      [tokens.ink, tokens.card],
      [tokens.muted, tokens.anchors.canvas],
      [tokens.muted, tokens.surface],
      [tokens.muted, tokens.card],
      [tokens.link, tokens.anchors.canvas],
      [tokens.link, tokens.surface],
      [tokens.link, tokens.card],
      [tokens.onAction, tokens.action],
      [tokens.onAction, tokens.actionHover],
      [tokens.onSecondary, tokens.anchors.secondary],
    ];
    for (const [foreground, background] of textPairs) {
      assert.ok(
        contrast(foreground, background) >= 5.5,
        `${id}: ${foreground} on ${background} must meet 5.5:1`,
      );
    }
    for (const [role, foreground] of [
      ["ink", tokens.ink],
      ["muted", tokens.muted],
      ["link", tokens.link],
    ]) {
      for (const background of [tokens.anchors.canvas, tokens.card]) {
        assert.ok(
          contrast(foreground, background) <= 8.25,
          `${id}: standard ${role} contrast should preserve the palette instead of` +
            " approaching black on white",
        );
      }
    }
    for (const background of [tokens.anchors.canvas, tokens.surface, tokens.card]) {
      assert.ok(contrast(tokens.focus, background) >= 3, `${id}: focus must meet 3:1`);
    }
    const style = courseThemeStyle(tokens);
    for (const [property, value] of [
      ["--ple-theme-canvas", tokens.anchors.canvas],
      ["--ple-theme-secondary", tokens.anchors.secondary],
      ["--ple-theme-accent", tokens.anchors.accent],
      ["--ple-card-surface", tokens.card],
      ["--ple-action-hover", tokens.actionHover],
      ["--ple-on-action", tokens.onAction],
      ["--ple-theme-on-secondary", tokens.onSecondary],
    ]) {
      assert.ok(
        style.includes(`${property}: ${value}`),
        `${id}: ${property} must retain its token`,
      );
    }
  }
});

test("Grass uses the Roosevelt-inspired anchors and accessible derived actions", () => {
  const grass = courseThemeTokens("grass");
  assert.deepEqual(grass.anchors, {
    canvas: "#bddeb1",
    secondary: "#73c167",
    accent: "#008852",
  });
  assert.equal(grass.action, "#006b40");
  assert.equal(grass.link, "#005c38");
  assert.ok(contrast(grass.onAction, grass.action) >= 5.5);
  assert.ok(contrast(grass.link, grass.anchors.canvas) >= 5.5);
});

test("unknown theme IDs fail closed instead of selecting a default", () => {
  assert.throws(() => courseThemeTokens("woodland"), /Unknown course theme/u);
  assert.throws(() =>
    decodeCourseAppearanceView({ theme: "woodland", revision: "1", banner: null }),
  );
  assert.throws(() => decodeCourseAppearanceView({ theme: "grass", revision: "01", banner: null }));
});

test("course banners preserve their closed decorative or informative treatment", () => {
  const bannerReference = "00000000-0000-0000-0000-000000000007";
  const decorative = decodeCourseAppearanceView({
    theme: "grass",
    revision: "1",
    banner: { reference: bannerReference, alternativeText: { kind: "decorative" } },
  });
  const informative = decodeCourseAppearanceView({
    theme: "grass",
    revision: "1",
    banner: {
      reference: bannerReference,
      alternativeText: { kind: "informative", text: "Forest canopy" },
    },
  });

  assert.equal(courseBannerImageAlternativeText(decorative.banner.alternativeText), "");
  assert.equal(
    courseBannerImageAlternativeText(informative.banner.alternativeText),
    "Forest canopy",
  );
  for (const alternativeText of [
    { kind: "decorative", text: "must not be accepted" },
    { kind: "informative", text: "   " },
    { kind: "informative", text: "\u03b2".repeat(161) },
  ]) {
    assert.throws(() =>
      decodeCourseAppearanceView({
        theme: "grass",
        revision: "1",
        banner: { reference: bannerReference, alternativeText },
      }),
    );
  }
  assert.throws(() =>
    decodeCourseAppearanceView({
      theme: "grass",
      revision: "1",
      banner: { id: bannerReference, alternativeText: { kind: "decorative" } },
    }),
  );
});
