// A Question Preview follows bounded resize messages from its content frame.

import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";

import { startHarnessServer } from "./ribbon_harness_server.mjs";

const PREVIEW_RESIZE_SENTENCE = "Question preview follows an accepted resize report.";

const PREVIEW_POSTER = `<!doctype html><meta charset="utf-8"><script>
let sentInvalid = false;
setInterval(() => {
  const height = sentInvalid ? 420 : 2000;
  sentInvalid = true;
  parent.postMessage({ kind: "ple.webwork.preview.resize", version: 1, height }, "*");
}, 30);
</script>`;

async function previewBundle() {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("../support/region_sizing_harness.tsx", import.meta.url).pathname],
    format: "iife",
    globalName: "RegionSizing",
    outfile: "region_sizing_harness.js",
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
  if (javascript === undefined) throw new Error("Region sizing bundle is missing JavaScript.");
  return Buffer.from(javascript.contents).toString("utf8");
}

test(PREVIEW_RESIZE_SENTENCE, { timeout: 30_000 }, async () => {
  const bundle = await previewBundle();
  const harnessServer = await startHarnessServer(
    `<!doctype html><html><body><div id="region-sizing"></div>
<script src="/region-sizing.js"></script></body></html>`,
    "",
    new Map([
      ["/preview-poster.html", { body: PREVIEW_POSTER, contentType: "text/html; charset=utf-8" }],
      ["/region-sizing.js", { body: bundle, contentType: "text/javascript; charset=utf-8" }],
    ]),
  );
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  try {
    await page.goto(harnessServer.evidenceUrl);
    await page.waitForFunction(
      () => typeof window.RegionSizing?.mountQuestionPreview === "function",
    );
    await page.evaluate(() => {
      const target = document.querySelector("#region-sizing");
      if (!(target instanceof HTMLElement)) throw new Error("Preview root is missing.");
      window.RegionSizing.mountQuestionPreview(target);
    });
    const frame = page.getByTitle("Question preview", { exact: true });
    await frame.waitFor({ state: "attached" });
    let previewBox = null;
    const deadline = Date.now() + 5_000;
    while (Date.now() < deadline) {
      previewBox = await frame.evaluate((element) => {
        return { styleHeight: element.style.height };
      });
      if (previewBox.styleHeight === "420px") break;
      await page.waitForTimeout(50);
    }
    assert.equal(
      previewBox?.styleHeight,
      "420px",
      `${PREVIEW_RESIZE_SENTENCE} ${JSON.stringify(previewBox)}`,
    );
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser.close();
    await harnessServer.close();
  }
});
