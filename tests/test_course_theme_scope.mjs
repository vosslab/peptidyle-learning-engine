// Browser-registry and route-scope behavior for WP-CA5.

import assert from "node:assert/strict";
import test from "node:test";

import { decodeCourseAppearance } from "../src/api/decoders.ts";
import { courseThemeRouteRequest } from "../src/features/course_appearance/course_theme_route.ts";
import {
  COURSE_THEME_CATALOG,
  courseThemeStyle,
  courseThemeTokens,
} from "../src/features/course_appearance/theme_catalog.ts";

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

function relativeLuminance(hex) {
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
  assert.deepEqual(Object.keys(COURSE_THEME_CATALOG), THEME_IDS);
  for (const id of THEME_IDS) {
    const tokens = courseThemeTokens(id);
    const textPairs = [
      [tokens.ink, tokens.anchors.canvas],
      [tokens.ink, tokens.surface],
      [tokens.muted, tokens.anchors.canvas],
      [tokens.muted, tokens.surface],
      [tokens.link, tokens.anchors.canvas],
      [tokens.link, tokens.surface],
      [tokens.onAction, tokens.action],
      [tokens.onAction, tokens.actionHover],
    ];
    for (const [foreground, background] of textPairs) {
      assert.ok(
        contrast(foreground, background) >= 5.5,
        `${id}: ${foreground} on ${background} must meet 5.5:1`,
      );
    }
    for (const background of [tokens.anchors.canvas, tokens.surface]) {
      assert.ok(contrast(tokens.focus, background) >= 3, `${id}: focus must meet 3:1`);
      assert.ok(contrast(tokens.border, background) >= 3, `${id}: border must meet 3:1`);
    }
    const style = courseThemeStyle(tokens);
    for (const property of [
      "--ple-theme-canvas",
      "--ple-theme-secondary",
      "--ple-theme-accent",
      "--ple-card-surface",
      "--ple-action-hover",
    ]) {
      assert.match(style, new RegExp(`${property}: #[0-9a-f]{6}`, "u"));
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
  assert.throws(() => decodeCourseAppearance({ theme: "woodland", revision: "1", banner: null }));
  assert.throws(() => decodeCourseAppearance({ theme: "grass", revision: "01", banner: null }));
});

test("only course-owned executable routes request a theme scope", () => {
  const course = "0198e000-0000-7000-8000-000000000010";
  const assignment = "0198e000-0000-7000-8000-000000000020";
  const run = "0198e000-0000-7000-8000-000000000030";
  assert.deepEqual(courseThemeRouteRequest(`/courses/${course}`), {
    kind: "course",
    courseId: course,
  });
  assert.deepEqual(courseThemeRouteRequest(`/courses/${course}/assignments/${assignment}`), {
    kind: "course",
    courseId: course,
  });
  assert.deepEqual(
    courseThemeRouteRequest(`/instructor/courses/${course}/assignments/${assignment}/edit`),
    { kind: "course", courseId: course },
  );
  assert.deepEqual(courseThemeRouteRequest(`/instructor/courses/${course}/gradebook`), {
    kind: "course",
    courseId: course,
  });
  assert.deepEqual(courseThemeRouteRequest(`/instructor/courses/${course}/appearance`), {
    kind: "course",
    courseId: course,
  });
  assert.deepEqual(courseThemeRouteRequest(`/runs/${run}`), { kind: "runAttempt", runId: run });
  assert.deepEqual(courseThemeRouteRequest(`/runs/${run}/summary`), {
    kind: "runSummary",
    runId: run,
  });
  for (const path of ["/", "/library", "/workspace", `/library/${course}/versions/${assignment}`]) {
    assert.deepEqual(courseThemeRouteRequest(path), { kind: "global" });
  }
});
