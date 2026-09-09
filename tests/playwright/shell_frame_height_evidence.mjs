// shell_frame_height_evidence.mjs - viewport ownership evidence.
// Selector contract: src/application_shell.tsx owns .ple-shell-frame and the two
// shell shapes; src/style.css owns its viewport floor; src/ribbon/app_ribbon.css
// replaces the first grid track only for a Ribbon; course_theme_variables.tsx
// paints a themed .shell within the same grid item.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";
import { chromium } from "playwright";

import { bundledAppRibbonCss, loadAppRibbonForSsr } from "../support/ribbon_component_ssr.ts";
import { M6_RIBBON_FIXTURES } from "../support/ribbon_model_fixtures.ts";

const globalCss = readFileSync(new URL("../../src/style.css", import.meta.url), "utf8");
const accessibilityCss = readFileSync(
  new URL("../../src/styles/accessibility.css", import.meta.url),
  "utf8",
);
const scopeStylesSource = readFileSync(
  new URL("../../src/features/course_appearance/course_theme_scope_styles.ts", import.meta.url),
  "utf8",
);
const variablesStylesSource = readFileSync(
  new URL("../../src/features/course_appearance/course_theme_variables.tsx", import.meta.url),
  "utf8",
);
const componentCss = await bundledAppRibbonCss();
const RealAppRibbon = await loadAppRibbonForSsr();

function styleTemplate(source, declaration) {
  const expression = new RegExp("(?:export )?const " + declaration + " = `([\\s\\S]*?)`;");
  const match = source.match(expression);
  if (match?.[1] === undefined) throw new Error(`Missing ${declaration} source style template.`);
  return match[1];
}

const courseThemeCss = [
  styleTemplate(scopeStylesSource, "COURSE_THEME_SCOPE_STYLES"),
  styleTemplate(variablesStylesSource, "COURSE_THEME_VARIABLE_SHELL_STYLES"),
].join("\n");

function shellMarkup({ ribbon, tall, themed, legacy = false }) {
  const fixtureName = [
    ribbon ? "ribbon" : "header",
    tall ? "tall" : "short",
    themed ? "theme" : "plain",
  ].join("-");
  const contentClass = tall ? "shell-frame-proof-content--tall" : "shell-frame-proof-content";
  const content = `<main class="shell"><section id="main-content"><div class="${contentClass}">Proof content</div></section></main>`;
  const header = `<header class="site-header"><a class="brand" href="/" aria-label="Peptidyle home"><span class="brand-mark" aria-hidden="true">P</span><span>Peptidyle</span></a></header>`;
  const ribbonMarkup = renderToString(() =>
    createComponent(RealAppRibbon, { model: M6_RIBBON_FIXTURES.courseInstructor }),
  );
  const frameClass = legacy
    ? "shell-frame-legacy-ribbon"
    : `ple-shell-frame${ribbon ? " ple-ribbon-shell-grid" : ""}`;
  const frameChildren = ribbon ? `${ribbonMarkup}${content}` : `${header}${content}`;
  const frame = `<div class="${frameClass}" data-shell-frame-fixture="${fixtureName}">${frameChildren}</div>`;
  const frameWithLegacyHeader = legacy ? `${header}${frame}` : frame;
  if (!themed) return frameWithLegacyHeader;
  return `<div class="course-theme-scope" data-course-theme="ocean" style="--ple-theme-canvas: rgb(12, 34, 56); --ple-theme-secondary: #2d8a92; --ple-theme-accent: #d6a13d;">${frameWithLegacyHeader}</div>`;
}

const fixedFixtures = [
  { ribbon: false, tall: false, themed: false },
  { ribbon: true, tall: false, themed: false },
  { ribbon: false, tall: true, themed: false },
  { ribbon: true, tall: true, themed: false },
  { ribbon: false, tall: false, themed: true },
  { ribbon: true, tall: false, themed: true },
  { ribbon: false, tall: true, themed: true },
  { ribbon: true, tall: true, themed: true },
].map((fixture) => ({ ...fixture, markup: shellMarkup(fixture) }));
const legacyFixture = shellMarkup({ ribbon: true, tall: false, themed: false, legacy: true });
const proofCss = [
  "html,body{margin:0;inline-size:100%;}",
  ".shell-frame-proof-content{min-block-size:1px;}",
  ".shell-frame-proof-content--tall{min-block-size:200dvh;}",
  ".shell-frame-legacy-ribbon{display:grid;grid-template-rows:var(--ple-ribbon-block-size) minmax(0,1fr);}.shell-frame-legacy-ribbon .shell{min-block-size:calc(100vh - var(--ple-header-block-size));}",
].join("");
function documentMarkup(fixtureMarkup) {
  return [
    "<!doctype html><html><head><style>",
    globalCss,
    accessibilityCss,
    componentCss,
    courseThemeCss,
    proofCss,
    "</style></head><body>",
    fixtureMarkup,
    "</body></html>",
  ].join("\n");
}
const legacyDocumentMarkup = [
  "<!doctype html><html><head><style>",
  globalCss,
  accessibilityCss,
  componentCss,
  proofCss,
  "</style></head><body>",
  legacyFixture,
  "</body></html>",
].join("\n");

function near(actual, expected, message) {
  assert.ok(Math.abs(actual - expected) < 0.25, `${message}: ${actual}px != ${expected}px`);
}

async function measure(page, profile, fixture) {
  await page.setViewportSize(profile);
  await page.setContent(documentMarkup(fixture.markup));
  return page.evaluate(() => {
    const frame = document.querySelector("[data-shell-frame-fixture]");
    const content = frame?.querySelector(
      ".shell-frame-proof-content, .shell-frame-proof-content--tall",
    );
    const shell = frame?.querySelector(".shell");
    if (
      !(frame instanceof HTMLElement) ||
      !(content instanceof HTMLElement) ||
      !(shell instanceof HTMLElement)
    ) {
      throw new Error("Height evidence fixture is missing its frame, content, or shell.");
    }
    const themed = frame.closest(".course-theme-scope") !== null;
    const frameBox = frame.getBoundingClientRect();
    const shellBox = shell.getBoundingClientRect();
    return {
      viewportHeight: window.innerHeight,
      name: frame.getAttribute("data-shell-frame-fixture"),
      themed,
      tall: content.classList.contains("shell-frame-proof-content--tall"),
      documentScrollHeight: document.documentElement.scrollHeight,
      documentClientHeight: document.documentElement.clientHeight,
      frameHeight: frameBox.height,
      shellLeft: shellBox.left,
      shellRight: shellBox.right,
      shellBackground: getComputedStyle(shell).backgroundColor,
    };
  });
}

async function legacyShortRibbonScrolls(page, profile) {
  await page.setViewportSize(profile);
  await page.setContent(legacyDocumentMarkup);
  return page.evaluate(
    () => document.documentElement.scrollHeight > document.documentElement.clientHeight,
  );
}

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  for (const profile of [
    { width: 900, height: 800 },
    { width: 320, height: 800 },
  ]) {
    const legacyScrolls = await legacyShortRibbonScrolls(page, profile);
    assert.equal(
      legacyScrolls,
      true,
      `${profile.width}px legacy short Ribbon fixture records the pre-frame overflow regression`,
    );
    for (const fixture of fixedFixtures) {
      const result = await measure(page, profile, fixture);
      if (result.tall) {
        assert.ok(
          result.documentScrollHeight > result.documentClientHeight,
          `${profile.width}px ${result.name} grows normally for tall content`,
        );
      } else {
        assert.ok(
          result.documentScrollHeight <= result.documentClientHeight,
          `${profile.width}px ${result.name} has no short-page vertical scrollbar`,
        );
        near(
          result.frameHeight,
          result.viewportHeight,
          `${profile.width}px ${result.name} frame floor`,
        );
      }
      if (result.themed) {
        assert.equal(
          result.shellBackground,
          "rgb(12, 34, 56)",
          `${result.name} paints its course canvas`,
        );
        near(result.shellLeft, 0, `${result.name} theme paint starts at the viewport edge`);
        near(
          result.shellRight,
          profile.width,
          `${result.name} theme paint ends at the viewport edge`,
        );
      }
    }
  }
  process.stdout.write(
    "Shell-frame height evidence: legacy Ribbon overflow reproduced; both shell shapes grow " +
      "without short-page scrolling, including edge-to-edge themed content.\n",
  );
} finally {
  await browser.close();
}
