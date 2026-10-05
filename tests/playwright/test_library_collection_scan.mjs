// Large Question Library results stay scannable rows that can be compared.

import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";

async function loadBundle() {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("./support/library_scan_harness.tsx", import.meta.url).pathname],
    format: "iife",
    globalName: "LibraryScan",
    outfile: "library_scan_harness.js",
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
    throw new Error("Library scan bundle is missing JavaScript.");
  }
  return Buffer.from(javascript.contents).toString("utf8");
}

test("large collections stay scannable rows with comparable fields", async () => {
  const bundle = await loadBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    await page.setContent('<!doctype html><html><body><div id="library"></div></body></html>', {
      waitUntil: "load",
    });
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));
    await page.addScriptTag({ content: bundle });
    await page.evaluate(() => {
      window.LibraryScan.mountLibraryScan(document.getElementById("library"));
    });
    if (pageErrors.length > 0) {
      throw new Error(pageErrors.join("\n"));
    }
    const list = page.getByRole("list", { name: "Question Library results", exact: true });
    await list.waitFor();
    assert.equal(
      await list.evaluate((element) => element.classList.contains("record-list--gallery")),
      false,
    );
    const records = list.locator(".record-list__row--semantic");
    assert.equal(await records.count(), 2);
    const text = await list.innerText();
    for (const fact of [
      "Enzyme kinetics",
      "ABCD-XEFG",
      "Biology",
      "Ada Lovelace",
      "Inhibitor binding",
      "7K3M-79QP",
      "Chemistry",
      "Marie Curie",
    ]) {
      assert.match(text, new RegExp(fact));
    }
  } finally {
    await browser.close();
  }
});
