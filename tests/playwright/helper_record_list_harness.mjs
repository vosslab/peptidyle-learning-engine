// Shared HTTP/browser setup for RecordList contract evidence.

import { chromium } from "playwright";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { resolveTheme } from "../../src/appearance/appearance_rules.ts";
import { themeStyle, themeTokens } from "../../src/appearance/theme_registry.ts";
import { bundleRecordListHarness } from "../support/record_list_harness_loader.ts";
import { startHarnessServer } from "./ribbon_harness_server.mjs";

export async function openRecordListHarness(options = {}) {
  const headless = options.headless ?? true;
  const bundle = await bundleRecordListHarness();
  const markup = [
    "<!doctype html><html><head><style>",
    bundle.stylesheet,
    '</style></head><body><div id="root"></div></body></html>',
  ].join("\n");
  const harnessServer = await startHarnessServer(
    markup,
    bundle.stylesheet,
    new Map([
      [
        "/assets/avatar_catalog/svg/amber-arch.svg",
        {
          body: readFileSync(
            new URL("../../assets/avatar_catalog/svg/amber-arch.svg", import.meta.url),
          ),
          contentType: "image/svg+xml",
        },
      ],
    ]),
  );
  const browser = await chromium.launch({ headless });
  const page = await browser.newPage({ viewport: { width: 1024, height: 768 } });
  page.setDefaultTimeout(5_000);
  const pageErrors = [];
  const consoleErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  await page.goto(harnessServer.evidenceUrl);
  const theme = resolveTheme({ signedInInstructor: false });
  await page.evaluate(
    ({ declarations, themeId }) => {
      const root = document.documentElement;
      for (const entry of declarations.split(";")) {
        const separator = entry.indexOf(":");
        if (separator <= 0) continue;
        root.style.setProperty(entry.slice(0, separator).trim(), entry.slice(separator + 1).trim());
      }
      root.dataset.theme = themeId;
      root.dataset.displayMode = "light";
    },
    { declarations: themeStyle(themeTokens(theme)), themeId: theme },
  );
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
  await page.addScriptTag({ content: Buffer.from(bundle.javascript).toString("utf8") });
  await page.waitForFunction(
    () => typeof window.PleRecordListHarness?.mountRecordListHarness === "function",
    undefined,
    { timeout: 5_000 },
  );
  await page.evaluate(() => {
    const root = document.querySelector("#root");
    if (!(root instanceof HTMLElement)) throw new Error("RecordList harness root is missing.");
    window.recordListHarness = window.PleRecordListHarness.mountRecordListHarness(root);
  });
  await page.locator('[data-record-list-case="one-variant"]').waitFor({ state: "visible" });
  return { browser, consoleErrors, harnessServer, page, pageErrors };
}

export async function recordIds(list) {
  return list
    .locator("[data-record-id]")
    .evaluateAll((rows) => rows.map((row) => row.getAttribute("data-record-id")));
}
