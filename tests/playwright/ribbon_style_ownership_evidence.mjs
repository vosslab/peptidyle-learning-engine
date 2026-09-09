// Computed-style ownership evidence.
// Selector contract: src/ribbon/app_ribbon.tsx:400-429 supplies the two Ribbon
// nav rows and RibbonLink supplies .ple-app-ribbon__link; src/pages/
// course_instance_page.tsx:130 supplies the Course actions nav.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";
import { chromium } from "playwright";

import { bundledAppRibbonCss, loadAppRibbonForSsr } from "../support/ribbon_component_ssr.ts";
import { M6_RIBBON_FIXTURES } from "../support/ribbon_model_fixtures.ts";

const globalCss = readFileSync(new URL("../../src/style.css", import.meta.url), "utf8");
const courseInstanceCss = readFileSync(
  new URL("../../src/pages/course_instance_page.css", import.meta.url),
  "utf8",
);
const componentCss = await bundledAppRibbonCss();
const RealAppRibbon = await loadAppRibbonForSsr();

// The Ribbon owns component rules, while this small token fixture supplies the
// shared design-token vocabulary app_ribbon.css consumes in the live document.
const tokenCss = `
:root {
  --ple-surface-soft: #eef3f6;
  --ple-card-surface: #ffffff;
  --ple-ink: #344054;
  --ple-muted: #536171;
  --ple-accent: #17628a;
  --ple-accent-strong: #164f70;
  --ple-on-action: #ffffff;
  --ple-border: #c8d0d6;
  --ple-space-1: 0.25rem;
  --ple-space-2: 0.5rem;
  --ple-space-3: 0.75rem;
  --ple-space-4: 1rem;
  --ple-space-5: 1.25rem;
  --ple-space-6: 1.75rem;
  --ple-radius-control: 0.45rem;
  --ple-control-min-height: 2.25rem;
}
`;

// This is the removed source block captured as the course-actions baseline.
// It is injected before the action-link rules to preserve the historical
// cascade, then compared with the page-owned replacement at a desktop width.
const legacyNavCss = `
nav { display:flex; align-items:center; gap:0.1rem; }
nav a { position:relative; display:inline-grid; min-height:var(--ple-control-min-height); padding:0 0.65rem; place-items:center; border:0; border-radius:var(--ple-radius-control); background:transparent; color:var(--ple-muted); font-size:0.92rem; font-weight:650; text-decoration:none; }
nav a::after { position:absolute; right:0.65rem; bottom:0.12rem; left:0.65rem; height:2px; border-radius:2px; background:transparent; content:""; }
nav a:hover, nav a.active { background:var(--ple-surface-soft); color:var(--ple-accent-strong); }
nav a.active::after { background:var(--ple-accent); }
`;
const legacyGlobalCss = globalCss.replace(
  ".degraded-banner {",
  `${legacyNavCss}\n.degraded-banner {`,
);

function styleDocument(stylesheet, body) {
  return [
    "<!doctype html><html><head><style>",
    stylesheet,
    "html,body{margin:0}",
    "</style></head><body>",
    body,
    "</body></html>",
  ].join("\n");
}

function ribbonMarkup() {
  const model = M6_RIBBON_FIXTURES.courseInstructor;
  return renderToString(() => createComponent(RealAppRibbon, { model }));
}

async function ribbonStyles(page, stylesheet) {
  await page.setContent(styleDocument(stylesheet, ribbonMarkup()));
  return page.evaluate(() => {
    const targets = [
      ...document.querySelectorAll("nav[data-ribbon-row]"),
      ...document.querySelectorAll(".ple-app-ribbon__link"),
    ];
    return targets.map((target) => {
      const style = getComputedStyle(target);
      return {
        selector: target.matches("nav")
          ? `nav:${target.getAttribute("data-ribbon-row")}`
          : `link:${target.getAttribute("data-ribbon-control")}`,
        display: style.display,
        gap: style.gap,
        padding: style.padding,
        color: style.color,
        fontWeight: style.fontWeight,
      };
    });
  });
}

async function courseActionStyles(page, stylesheet) {
  const markup = `
    <nav class="course-card-actions course-instance-page__actions" aria-label="Course actions">
      <a class="primary-link" href="/students">Open Students</a>
      <a class="quiet-link" href="/assignments/new">Create Assignment</a>
    </nav>`;
  await page.setContent(styleDocument(stylesheet, markup));
  return page.evaluate(() => {
    const targets = [
      document.querySelector(".course-instance-page__actions"),
      document.querySelector(".primary-link"),
      document.querySelector(".quiet-link"),
    ];
    return targets.map((target) => {
      if (!(target instanceof HTMLElement)) throw new Error("Course action fixture is incomplete.");
      const style = getComputedStyle(target);
      return {
        display: style.display,
        gap: style.gap,
        alignItems: style.alignItems,
        minHeight: style.minHeight,
        padding: style.padding,
        color: style.color,
        fontWeight: style.fontWeight,
        textDecorationLine: style.textDecorationLine,
      };
    });
  });
}

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1024, height: 700 } });
  const ribbonOnly = await ribbonStyles(page, `${tokenCss}\n${componentCss}`);
  const ribbonWithFullStyles = await ribbonStyles(page, `${globalCss}\n${componentCss}`);
  assert.deepEqual(
    ribbonWithFullStyles,
    ribbonOnly,
    "Ribbon nav rows and links retain their component-owned presentation without global nav rules",
  );

  const courseBaseline = await courseActionStyles(page, `${legacyGlobalCss}\n${courseInstanceCss}`);
  const courseCurrent = await courseActionStyles(page, `${globalCss}\n${courseInstanceCss}`);
  assert.deepEqual(
    courseCurrent,
    courseBaseline,
    "Course actions retain their captured desktop presentation after nav ownership moves local",
  );
  process.stdout.write(
    "Ribbon style ownership evidence: component-only Ribbon and preserved Course actions passed.\n",
  );
} finally {
  await browser.close();
}
