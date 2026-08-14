// Reproducible visual and computed-palette evidence for WP-CA7.

import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

import { expect, test } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import {
  COURSE_THEME_CATALOG,
  type CourseThemeTokens,
} from "../../src/features/course_appearance/theme_catalog";
import {
  COURSE_ID,
  appearanceHeaders,
  bannerBytes,
  json,
  openAppearance,
  resolveCourseReference,
  session,
} from "./course_appearance_fixtures";

interface RenderedThemeMetrics {
  readonly canvas: string;
  readonly ink: string;
  readonly muted: string;
  readonly link: string;
  readonly actionForeground: string;
  readonly actionBackground: string;
  readonly hoverForeground: string;
  readonly hoverBackground: string;
  readonly secondaryForeground: string;
  readonly secondaryBackground: string;
  readonly surfaceForeground: string;
  readonly surfaceBackground: string;
  readonly focus: string;
  readonly border: string;
}

interface OklabColor {
  readonly lightness: number;
  readonly a: number;
  readonly b: number;
}

function channel(value: number): number {
  const normalized = value / 255;
  return normalized <= 0.04045 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
}

function luminance(cssColor: string): number {
  const values = cssColor
    .match(/[0-9.]+/gu)
    ?.slice(0, 3)
    .map(Number);
  if (values === undefined || values.length !== 3) {
    throw new Error(`Expected a computed RGB color, received ${cssColor}`);
  }
  const channels = cssColor.startsWith("color(srgb") ? values.map((value) => value * 255) : values;
  const [red, green, blue] = channels;
  if (red === undefined || green === undefined || blue === undefined) {
    throw new Error(`Expected three computed RGB channels, received ${cssColor}`);
  }
  return 0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue);
}

function contrast(first: string, second: string): number {
  const lighter = Math.max(luminance(first), luminance(second));
  const darker = Math.min(luminance(first), luminance(second));
  return (lighter + 0.05) / (darker + 0.05);
}

function oklab(hex: string): OklabColor {
  const red = channel(Number.parseInt(hex.slice(1, 3), 16));
  const green = channel(Number.parseInt(hex.slice(3, 5), 16));
  const blue = channel(Number.parseInt(hex.slice(5, 7), 16));
  const l = Math.cbrt(0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue);
  const m = Math.cbrt(0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue);
  const s = Math.cbrt(0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue);
  return {
    lightness: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  };
}

function deltaE(first: string, second: string): number {
  const left = oklab(first);
  const right = oklab(second);
  return Math.hypot(left.lightness - right.lightness, left.a - right.a, left.b - right.b) * 100;
}

function rounded(value: number): number {
  return Number(value.toFixed(2));
}

function renderedRatios(metrics: RenderedThemeMetrics): Record<string, number> {
  return {
    inkOnCanvas: rounded(contrast(metrics.ink, metrics.canvas)),
    mutedOnCanvas: rounded(contrast(metrics.muted, metrics.canvas)),
    linkOnCanvas: rounded(contrast(metrics.link, metrics.canvas)),
    actionText: rounded(contrast(metrics.actionForeground, metrics.actionBackground)),
    actionHoverText: rounded(contrast(metrics.hoverForeground, metrics.hoverBackground)),
    activeNavigationText: rounded(
      contrast(metrics.secondaryForeground, metrics.secondaryBackground),
    ),
    cardText: rounded(contrast(metrics.surfaceForeground, metrics.surfaceBackground)),
    focusOnCanvas: rounded(contrast(metrics.focus, metrics.canvas)),
    focusOnCard: rounded(contrast(metrics.focus, metrics.surfaceBackground)),
    borderOnCanvas: rounded(contrast(metrics.border, metrics.canvas)),
    borderOnCard: rounded(contrast(metrics.border, metrics.surfaceBackground)),
  };
}

async function measurePreview(
  page: import("@playwright/test").Page,
): Promise<RenderedThemeMetrics> {
  return page.locator(".course-appearance-preview-theme").evaluate((element) => {
    const probe = document.createElement("div");
    probe.innerHTML = `
      <p data-probe="ink">Body text</p>
      <p class="course-appearance-help" data-probe="muted">Supporting text</p>
      <a data-probe="link" href="#preview">Text link</a>
      <button class="primary-action" data-probe="action" type="button">Action</button>
      <span data-probe="hover" style="background: var(--ple-action-hover); color: var(--ple-on-action)">Hover</span>
      <span data-probe="secondary" style="background: var(--ple-theme-secondary); color: var(--ple-theme-on-secondary)">Active course navigation</span>
      <span data-probe="surface" style="background: var(--ple-card-surface); color: var(--ple-ink)">Card</span>
      <span data-probe="focus" style="border: 4px solid var(--ple-focus)">Focus</span>
      <span data-probe="border" style="border: 4px solid var(--ple-border)">Border</span>
    `;
    element.prepend(probe);
    const style = (name: string): CSSStyleDeclaration => {
      const target = probe.querySelector(`[data-probe="${name}"]`);
      if (target === null) throw new Error(`Missing rendered palette probe: ${name}`);
      return getComputedStyle(target);
    };
    const scope = getComputedStyle(element);
    const result = {
      canvas: scope.backgroundColor,
      ink: style("ink").color,
      muted: style("muted").color,
      link: style("link").color,
      actionForeground: style("action").color,
      actionBackground: style("action").backgroundColor,
      hoverForeground: style("hover").color,
      hoverBackground: style("hover").backgroundColor,
      secondaryForeground: style("secondary").color,
      secondaryBackground: style("secondary").backgroundColor,
      surfaceForeground: style("surface").color,
      surfaceBackground: style("surface").backgroundColor,
      focus: style("focus").borderTopColor,
      border: style("border").borderTopColor,
    };
    probe.remove();
    return result;
  });
}

function dedupTable(
  themes: ReadonlyArray<readonly [string, CourseThemeTokens]>,
): ReadonlyArray<Record<string, string | number | boolean>> {
  const rows: Array<Record<string, string | number | boolean>> = [];
  for (let left = 0; left < themes.length; left += 1) {
    const first = themes[left];
    if (first === undefined) continue;
    for (let right = left + 1; right < themes.length; right += 1) {
      const second = themes[right];
      if (second === undefined) continue;
      const secondary = deltaE(first[1].anchors.secondary, second[1].anchors.secondary);
      const accent = deltaE(first[1].anchors.accent, second[1].anchors.accent);
      const mean = (secondary + accent) / 2;
      const maximum = Math.max(secondary, accent);
      rows.push({
        first: first[0],
        second: second[0],
        secondaryDeltaE: rounded(secondary),
        accentDeltaE: rounded(accent),
        meanDeltaE: rounded(mean),
        maximumDeltaE: rounded(maximum),
        redundant: mean < 8 && maximum < 10,
      });
    }
  }
  return rows.sort((first, second) => Number(first.meanDeltaE) - Number(second.meanDeltaE));
}

test("captures the reviewed course-appearance visual and palette artifacts", async ({ page }) => {
  test.skip(
    process.env["PLE_CAPTURE_COURSE_APPEARANCE_VISUALS"] !== "1",
    "set PLE_CAPTURE_COURSE_APPEARANCE_VISUALS=1 to write the reviewed generated artifacts",
  );
  const artifactDirectory = resolve(
    process.env["PLE_COURSE_APPEARANCE_VISUALS_DIR"] ?? "generated/ui/course_appearance",
  );
  await mkdir(artifactDirectory, { recursive: true });
  const appearance = { theme: "grass", revision: "1", banner: null };
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    const navigation = resolveCourseReference(route, path);
    if (navigation !== null) return await navigation;
    if (path === `/api/courses/${COURSE_ID}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${COURSE_ID}/appearance`) {
      return await json(route, appearance, 200, appearanceHeaders("1"));
    }
    return await json(route, { error: "unexpected visual acceptance request" }, 500);
  });
  await page.setViewportSize({ width: 1_280, height: 800 });
  await openAppearance(page);
  await page.getByLabel("Choose a banner image").setInputFiles({
    name: "course-banner.png",
    mimeType: "image/png",
    buffer: bannerBytes,
  });
  await expect(page.locator("img.course-appearance-banner")).toHaveCount(2);

  await page.screenshot({
    path: resolve(artifactDirectory, "settings_1280x800.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 1_280, height: 800 });
  await page.emulateMedia({ forcedColors: "active", reducedMotion: "reduce" });
  await page.screenshot({
    path: resolve(artifactDirectory, "settings_forced_colors.png"),
    fullPage: true,
  });
  await page.emulateMedia({ forcedColors: "none", reducedMotion: "no-preference" });

  const entries = Object.entries(COURSE_THEME_CATALOG);
  const renderedThemes: Array<Record<string, unknown>> = [];
  const previews: string[] = [];
  for (const [id, tokens] of entries) {
    await page.getByRole("radio", { name: tokens.name, exact: true }).check();
    await expect(page.locator(".course-appearance-preview-theme")).toHaveAttribute(
      "data-preview-theme",
      id,
    );
    const metrics = await measurePreview(page);
    const ratios = renderedRatios(metrics);
    for (const [name, ratio] of Object.entries(ratios)) {
      if (name.startsWith("border")) continue;
      expect(ratio, `${id} ${name}: ${JSON.stringify(metrics)}`).toBeGreaterThanOrEqual(
        name.startsWith("focus") ? 3 : 5.5,
      );
      if (["inkOnCanvas", "mutedOnCanvas", "linkOnCanvas", "cardText"].includes(name)) {
        expect(ratio, `${id} ${name} standard-mode contrast ceiling`).toBeLessThanOrEqual(8.25);
      }
    }
    renderedThemes.push({
      id,
      name: tokens.name,
      anchors: tokens.anchors,
      computed: metrics,
      ratios,
    });
    previews.push(
      await page.locator(".course-appearance-preview-theme").evaluate((node) => node.outerHTML),
    );
  }

  const dedup = dedupTable(entries);
  expect(dedup.filter((row) => row.redundant)).toEqual([]);
  const metricsDocument = {
    formatVersion: 1,
    thresholds: {
      normalTextContrast: 5.5,
      standardTextContrastCeiling: 8.25,
      focusContrast: 3,
      redundantMeanDeltaE: 8,
      redundantMaximumDeltaE: 10,
    },
    renderedThemes,
    oklabDedup: dedup,
  };
  await writeFile(
    resolve(artifactDirectory, "palette_metrics.json"),
    `${JSON.stringify(metricsDocument, null, 2)}\n`,
    "utf8",
  );

  const componentStyles = await page
    .locator('section[data-route-surface="courseAppearance"] > style')
    .textContent();
  if (componentStyles === null)
    throw new Error("Course appearance component styles were not found");
  await page.setViewportSize({ width: 1_600, height: 1_000 });
  await page.locator("main").evaluate(
    (main, payload) => {
      main.innerHTML = `
        <style>${payload.componentStyles}</style>
        <style>
          #course-theme-contact-sheet { padding: 1.5rem; background: white; color: #231f20; }
          #course-theme-contact-sheet h1 { max-width: none; margin-bottom: 1rem; }
          .course-theme-contact-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 1rem; }
          .course-theme-contact-card { min-width: 0; padding: .75rem; border: 1px solid #686868; border-radius: .75rem; }
          .course-theme-contact-card h2 { margin: 0 0 .6rem; font-size: 1.15rem; }
          .course-theme-contact-card .course-appearance-preview-theme { padding: .7rem; }
          .course-theme-contact-card .course-appearance-preview { width: 100%; }
          .course-theme-contact-card .course-appearance-preview--narrow { display: none; }
          .course-theme-contact-card .course-appearance-banner { display: none; }
          .course-theme-contact-card .course-appearance-preview--wide::after {
            display: block;
            margin-top: .1rem;
            padding: .45rem .6rem .6rem;
            border-bottom: .22rem solid var(--ple-theme-accent);
            border-radius: .35rem;
            background: var(--ple-theme-secondary);
            color: var(--ple-theme-on-secondary);
            content: "Assignments  ·  Questions  ·  Gradebook";
            font-size: .8rem;
            font-weight: 750;
          }
          .course-theme-contact-card figcaption { font-size: .8rem; }
        </style>
        <section id="course-theme-contact-sheet">
          <h1>Course theme contact sheet</h1>
          <div class="course-theme-contact-grid">
            ${payload.previews
              .map(
                (preview, index) =>
                  `<article class="course-theme-contact-card"><h2>${payload.names[index]}</h2>${preview}</article>`,
              )
              .join("")}
          </div>
        </section>
      `;
    },
    { componentStyles, previews, names: entries.map(([, tokens]) => tokens.name) },
  );
  await page.locator("#course-theme-contact-sheet").screenshot({
    path: resolve(artifactDirectory, "theme_contact_sheet.png"),
  });
});
