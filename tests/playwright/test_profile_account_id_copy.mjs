// Profile displays and copies only a canonical Account ID.

import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";

async function loadBundle() {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("./support/profile_account_id_harness.tsx", import.meta.url).pathname],
    format: "iife",
    globalName: "ProfileAccountIdCopy",
    outfile: "profile_account_id_harness.js",
    platform: "browser",
    write: false,
    plugins: [
      solidPlugin({ solid: { generate: "dom", hydratable: false } }),
      {
        name: "ignore-css",
        setup(bundle) {
          bundle.onResolve({ filter: /\.css$/ }, (args) => ({
            path: args.path,
            namespace: "ignore-css",
          }));
          bundle.onLoad({ filter: /.*/, namespace: "ignore-css" }, () => ({
            contents: "export {}",
            loader: "js",
          }));
        },
      },
    ],
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) {
    throw new Error("Profile Account ID bundle is missing JavaScript.");
  }
  return Buffer.from(javascript.contents).toString("utf8");
}

test("Profile displays and copies only the canonical Account ID", async () => {
  const bundle = await loadBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
    await page.setContent('<!doctype html><html><body><div id="profile-id"></div></body></html>', {
      waitUntil: "load",
    });
    await page.addScriptTag({ content: bundle });
    await page.evaluate(() => {
      window.copiedAccountIds = [];
      navigator.clipboard = {
        writeText: async (value) => {
          window.copiedAccountIds.push(value);
        },
      };
      const target = document.getElementById("profile-id");
      window.ProfileAccountIdCopy.mountProfileAccountId(target, "U0000035E");
    });
    const shown = await page.locator("code").innerText();
    assert.equal(shown, "U0000035E");
    await page.getByRole("button", { name: "Copy Account ID U0000035E", exact: true }).click();
    await page.waitForFunction(() => window.copiedAccountIds.length === 1);
    assert.deepEqual(await page.evaluate(() => window.copiedAccountIds), ["U0000035E"]);
    await page.evaluate(() => {
      const target = document.getElementById("profile-id");
      target.replaceChildren();
      window.ProfileAccountIdCopy.mountProfileAccountId(target, "u0000035e");
    });
    assert.equal(await page.locator("code").count(), 0);
    assert.equal(await page.getByRole("button", { name: /Copy Account ID/ }).count(), 0);
    assert.deepEqual(await page.evaluate(() => window.copiedAccountIds), ["U0000035E"]);
    await page.evaluate(() => {
      const target = document.getElementById("profile-id");
      target.replaceChildren();
      window.ProfileAccountIdCopy.mountProfileAccountId(
        target,
        "00000000-0000-0000-0000-000000000001",
      );
    });
    assert.equal(await page.locator("code").count(), 0);
    assert.equal(await page.getByRole("button", { name: /Copy Account ID/ }).count(), 0);
    assert.deepEqual(await page.evaluate(() => window.copiedAccountIds), ["U0000035E"]);
  } finally {
    await browser.close();
  }
});
