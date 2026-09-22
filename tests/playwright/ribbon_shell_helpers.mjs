// Shared compiled-harness helpers for application-shell evidence.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { bundleRibbonShellHarness } from "../support/ribbon_shell_loader.ts";
import { mountRibbonHarness, startHarnessServer } from "./ribbon_harness_server.mjs";

export async function openRibbonShellEvidencePage() {
  const globalCss = [
    readFileSync(new URL("../../src/style.css", import.meta.url), "utf8"),
    readFileSync(new URL("../../src/style_responsive.css", import.meta.url), "utf8"),
  ].join("\n");
  const accessibilityCss = readFileSync(
    new URL("../../src/styles/accessibility.css", import.meta.url),
    "utf8",
  );
  const bundle = await bundleRibbonShellHarness();
  const markup = [
    "<!doctype html><html><head><style>",
    globalCss,
    accessibilityCss,
    bundle.stylesheet,
    'html,body{margin:0;min-inline-size:0}</style></head><body><div id="root"></div></body></html>',
  ].join("\n");
  const harnessServer = await startHarnessServer(markup);
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const pageErrors = [];
  const consoleErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  await page.goto(harnessServer.evidenceUrl);
  await mountRibbonHarness(page, bundle, pageErrors, consoleErrors);
  return { browser, page, harnessServer, pageErrors, consoleErrors };
}

export async function flush(page) {
  await page.evaluate(() => new Promise((resolve) => queueMicrotask(resolve)));
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
}

export function caseLocator(page, evidenceCase) {
  return page.locator(`[data-m10-case="${evidenceCase}"]`);
}

export async function assertBreadcrumbPreludeStyle(prelude, profile) {
  const style = await prelude.evaluate((element) => {
    const computed = getComputedStyle(element);
    const trail = element.querySelector("ol");
    if (!(trail instanceof HTMLElement)) throw new Error("breadcrumb trail is missing");
    return {
      backgroundColor: computed.backgroundColor,
      paddingInlineStart: computed.paddingInlineStart,
      paddingInlineEnd: computed.paddingInlineEnd,
      trailGap: getComputedStyle(trail).gap,
    };
  });
  assert.notEqual(
    style.backgroundColor,
    "rgba(0, 0, 0, 0)",
    `${profile} breadcrumb prelude has a resolved nontransparent surface`,
  );
  assert.deepEqual(
    {
      paddingInlineStart: style.paddingInlineStart,
      paddingInlineEnd: style.paddingInlineEnd,
      trailGap: style.trailGap,
    },
    {
      paddingInlineStart: profile === "narrow" ? "4px" : "16px",
      paddingInlineEnd: profile === "narrow" ? "4px" : "16px",
      trailGap: profile === "narrow" ? "4px" : "8px",
    },
    `${profile} breadcrumb prelude applies shared spacing tokens`,
  );
}

export async function waitForPath(page, evidenceCase, pathname) {
  if (evidenceCase === "current-production") {
    await page.waitForFunction(
      (expectedPath) => window.ribbonShell.currentPathname() === expectedPath,
      pathname,
      { timeout: 3_000 },
    );
    await flush(page);
    return;
  }
  const evidenceCaseLocator = caseLocator(page, evidenceCase);
  try {
    await evidenceCaseLocator
      .locator("[data-current-path]")
      .waitFor({ state: "attached", timeout: 3_000 });
  } catch (error) {
    const visibleText = await evidenceCaseLocator.innerText().catch(() => "case root missing");
    throw new Error(
      `${evidenceCase} did not mount its compiled-harness content boundary at ${pathname}: ` +
        visibleText,
      { cause: error },
    );
  }
  await page.waitForFunction(
    ({ evidenceCase: expectedCase, pathname: expectedPath }) =>
      document
        .querySelector(`[data-m10-case="${expectedCase}"] [data-current-path]`)
        ?.getAttribute("data-current-path") === expectedPath,
    { evidenceCase, pathname },
    { timeout: 3_000 },
  );
  await flush(page);
}

export async function assertOneStableRibbon(
  page,
  evidenceCase,
  initialRibbon,
  expectedTaskRowReserved,
) {
  const evidence = await caseLocator(page, evidenceCase).evaluate(
    (root, initial) => ({
      sameRibbon: root.querySelector(".ple-app-ribbon") === initial,
      roots: root.querySelectorAll(".ple-app-ribbon").length,
      shellGrids: root.querySelectorAll(".ple-ribbon-shell-grid").length,
      landmarks: root.querySelectorAll('[aria-label="PLE application Ribbon"]').length,
      tabs: root.querySelectorAll('nav[aria-label="Ribbon tabs"]').length,
      tasks: root.querySelectorAll('nav[aria-label="Ribbon tasks"]').length,
      ribbonTaskRow: root.querySelector(".ple-app-ribbon")?.getAttribute("data-ribbon-task-row"),
      frameTaskRow: root.querySelector(".ple-shell-frame")?.getAttribute("data-ribbon-task-row"),
      legacyCourse: root.querySelectorAll('nav[aria-label="Course management"]').length,
      legacyPrimary: root.querySelectorAll('nav[aria-label="Primary navigation"]').length,
    }),
    initialRibbon,
  );
  const expectedTaskRow = expectedTaskRowReserved ? "reserved" : "absent";
  assert.deepEqual(evidence, {
    sameRibbon: true,
    roots: 1,
    shellGrids: 1,
    landmarks: 1,
    tabs: 1,
    tasks: expectedTaskRowReserved ? 1 : 0,
    ribbonTaskRow: expectedTaskRow,
    frameTaskRow: expectedTaskRow,
    legacyCourse: 0,
    legacyPrimary: 0,
  });
}
