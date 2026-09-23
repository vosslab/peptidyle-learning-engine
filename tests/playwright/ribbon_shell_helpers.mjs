// Shared compiled-harness helpers for application-shell evidence.

import assert from "node:assert/strict";

import { chromium } from "playwright";

import { bundleRibbonShellHarness } from "../support/ribbon_shell_loader.ts";
import { mountRibbonHarness, startHarnessServer } from "./ribbon_harness_server.mjs";

export async function openRibbonShellEvidencePage(options = {}) {
  const bundle = await bundleRibbonShellHarness();
  const markup = [
    "<!doctype html><html><head><style>",
    bundle.stylesheet,
    '</style></head><body><div id="root"></div></body></html>',
  ].join("\n");
  const harnessServer = await startHarnessServer(markup, bundle.stylesheet);
  const browser = await chromium.launch({ headless: options.headless ?? true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const pageErrors = [];
  const consoleErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  await page.goto(harnessServer.evidenceUrl);
  const typography = await page.evaluate(() => {
    const code = document.createElement("pre");
    code.textContent = "semantic code typography probe";
    document.body.append(code);
    const normal = getComputedStyle(document.body).fontFamily;
    const monospace = getComputedStyle(code).fontFamily;
    code.remove();
    return { normal, monospace };
  });
  assert.match(typography.normal, /Atkinson Hyperlegible Next/u);
  assert.match(typography.monospace, /Atkinson Hyperlegible Mono/u);
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

export async function assertOneStableRibbon(page, evidenceCase, initialRibbon) {
  const evidence = await caseLocator(page, evidenceCase).evaluate(
    (root, initial) => ({
      sameRibbon: root.querySelector(".ple-app-ribbon") === initial,
      roots: root.querySelectorAll(".ple-app-ribbon").length,
      shellGrids: root.querySelectorAll(".ple-ribbon-shell-grid").length,
      landmarks: root.querySelectorAll('[aria-label="PLE application Ribbon"]').length,
      topFrames: root.querySelectorAll('section[data-ribbon-row-frame="top"]').length,
      topRows: root.querySelectorAll('section[data-ribbon-row="top"]').length,
      taskFrames: root.querySelectorAll('section[data-ribbon-row-frame="tasks"]').length,
      taskRows: root.querySelectorAll('nav[data-ribbon-row="tasks"]').length,
      tabs: root.querySelectorAll('nav[aria-label="Ribbon tabs"]').length,
      tasks: root.querySelectorAll('nav[aria-label="Ribbon tasks"]').length,
      legacyCourse: root.querySelectorAll('nav[aria-label="Course management"]').length,
      legacyPrimary: root.querySelectorAll('nav[aria-label="Primary navigation"]').length,
    }),
    initialRibbon,
  );
  assert.deepEqual(evidence, {
    sameRibbon: true,
    roots: 1,
    shellGrids: 1,
    landmarks: 1,
    topFrames: 1,
    topRows: 1,
    taskFrames: 1,
    taskRows: 1,
    tabs: 1,
    tasks: 1,
    legacyCourse: 0,
    legacyPrimary: 0,
  });
}
