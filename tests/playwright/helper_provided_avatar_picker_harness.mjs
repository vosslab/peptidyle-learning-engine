// Shared HTTP/browser setup for ProvidedAvatarPicker evidence.

import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { PROVIDED_AVATAR_CATALOG } from "../../src/features/profile_avatar/avatar_catalog_generated.ts";
import { bundleProvidedAvatarPickerHarness } from "../support/provided_avatar_picker_harness_loader.ts";
import { startHarnessServer } from "./ribbon_harness_server.mjs";

export async function openProvidedAvatarPickerHarness({ headless = true } = {}) {
  const bundle = await bundleProvidedAvatarPickerHarness();
  const additionalAssets = new Map([
    [
      "/provided_avatar_picker_harness.js",
      { body: bundle.javascript, contentType: "text/javascript; charset=utf-8" },
    ],
    [
      "/provided_avatar_picker_harness.css",
      { body: bundle.stylesheet, contentType: "text/css; charset=utf-8" },
    ],
    ...PROVIDED_AVATAR_CATALOG.map((entry) => [
      entry.assetPath,
      {
        body: readFileSync(new URL(`../../${entry.assetPath.slice(1)}`, import.meta.url)),
        contentType: "image/svg+xml",
      },
    ]),
  ]);
  const harnessServer = await startHarnessServer(
    `<!doctype html><html><head><link rel="stylesheet" href="/provided_avatar_picker_harness.css"></head>
      <body><div id="root"></div><script type="module">
        import { mountProvidedAvatarPickerHarness } from "/provided_avatar_picker_harness.js";
        mountProvidedAvatarPickerHarness(document.querySelector("#root"));
      </script></body></html>`,
    bundle.stylesheet,
    additionalAssets,
  );
  const browser = await chromium.launch({ headless });
  const page = await browser.newPage();
  const consoleErrors = [];
  const pageErrors = [];
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto(harnessServer.evidenceUrl);
  await page.getByRole("group", { name: "Choose an avatar" }).waitFor({ state: "visible" });
  return { browser, consoleErrors, harnessServer, page, pageErrors };
}
