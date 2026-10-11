// Shared compiled-harness helpers for application-shell evidence.

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
  await mountRibbonHarness(page, bundle, pageErrors, consoleErrors);
  return { browser, page, harnessServer, pageErrors, consoleErrors };
}

export function caseLocator(page, evidenceCase) {
  return page.locator(`[data-m10-case="${evidenceCase}"]`);
}
