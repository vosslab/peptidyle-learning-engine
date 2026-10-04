// Bloom Classification supports Question Library search and Assessment item sorting.

import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";

import { startHarnessServer } from "./ribbon_harness_server.mjs";

const COGNITIVE_ORDER = ["Remember", "Understand", "Apply", "Analyze", "Evaluate", "Create"];

async function loadBundle() {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL("../support/bloom_classification_workflow_harness.tsx", import.meta.url).pathname,
    ],
    format: "iife",
    globalName: "BloomClassificationWorkflow",
    outfile: "bloom_classification_workflow_harness.js",
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
    throw new Error("Bloom classification workflow bundle is missing JavaScript.");
  }
  return Buffer.from(javascript.contents).toString("utf8");
}

function guideOrder(entries) {
  return [...entries]
    .map((entry, initialIndex) => ({ ...entry, initialIndex }))
    .sort((left, right) => {
      const cognitive =
        COGNITIVE_ORDER.indexOf(left.cognitiveProcess) -
        COGNITIVE_ORDER.indexOf(right.cognitiveProcess);
      return cognitive === 0 ? left.initialIndex - right.initialIndex : cognitive;
    });
}

async function orderedEntries(page) {
  return page.locator('ol[aria-label="Ordered Assessment Entries"] > li').evaluateAll((items) =>
    items.map((item) => ({
      id: item.getAttribute("data-record-id"),
      title: item.querySelector("h3")?.textContent ?? "",
    })),
  );
}

test(
  "Bloom Classification supports Question Library search and Assessment item sorting",
  { timeout: 120_000 },
  async () => {
    const bundle = await loadBundle();
    const harnessServer = await startHarnessServer(
      `<!doctype html>
<html>
<head><style>body { margin: 0; font: 16px/1.4 sans-serif; }</style></head>
<body><div id="bloom-workflow"></div>
<script src="/bloom_classification_workflow_harness.js"></script>
</body>
</html>`,
      "",
      new Map([
        [
          "/bloom_classification_workflow_harness.js",
          { body: bundle, contentType: "text/javascript; charset=utf-8" },
        ],
      ]),
    );
    const browser = await chromium.launch({ headless: true });
    try {
      const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
      const pageErrors = [];
      page.on("pageerror", (error) => pageErrors.push(error.message));
      await page.goto(harnessServer.evidenceUrl);
      await page.waitForFunction(
        () =>
          typeof window.BloomClassificationWorkflow?.mountBloomAssessmentWorkflow === "function",
      );
      await page.evaluate(() => {
        const target = document.getElementById("bloom-workflow");
        window.BloomClassificationWorkflow.mountBloomAssessmentWorkflow(target);
      });
      await page
        .getByRole("heading", { name: "Assessment Question Editor", exact: true })
        .waitFor();
      const sortButton = page.getByRole("button", {
        name: "Sort by Bloom Classification",
        exact: true,
      });
      await sortButton.waitFor();
      await page.waitForFunction(() => {
        const button = [...document.querySelectorAll("button")].find(
          (candidate) => candidate.textContent === "Sort by Bloom Classification",
        );
        return button instanceof HTMLButtonElement && !button.disabled;
      });
      const fixture = await page.evaluate(() => window.bloomAssessmentEntries);
      const before = await orderedEntries(page);
      assert.deepEqual(
        before.map((entry) => entry.id),
        fixture.map((entry) => entry.id),
      );
      assert.deepEqual(
        before.map((entry) => entry.title),
        fixture.map((entry) => entry.title),
      );

      await sortButton.click();
      await page
        .getByText("Entries sorted by Bloom Classification. Save Questions when ready.", {
          exact: true,
        })
        .waitFor();
      const expected = guideOrder(fixture);
      const sorted = await orderedEntries(page);
      assert.deepEqual(
        sorted.map((entry) => entry.id),
        expected.map((entry) => entry.id),
      );
      assert.deepEqual(
        sorted.map((entry) => entry.title),
        expected.map((entry) => entry.title),
      );

      await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
      await page
        .getByText("Questions and order saved. Review Assessment Properties when you are ready.", {
          exact: true,
        })
        .waitFor();
      const saved = await page.evaluate(() => window.bloomAssessmentSaves[0]);
      assert.equal(saved.courseInstanceId, "CI7K3M2QAZ");
      assert.equal(saved.assessmentId, "A8H4N6PA6");
      assert.equal(saved.expectedAssessmentEditNumber, "1");
      assert.deepEqual(
        saved.entryIds,
        expected.map((entry) => entry.id),
      );

      await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
      await page
        .getByText(
          "This assessment changed elsewhere. Reload latest assessment before saving; " +
            "your current Entries remain here.",
          { exact: true },
        )
        .waitFor();
      const conflict = await page.evaluate(() => window.bloomAssessmentSaves[1]);
      assert.equal(conflict.expectedAssessmentEditNumber, "2");
      assert.deepEqual(
        conflict.entryIds,
        expected.map((entry) => entry.id),
      );

      await page.getByRole("button", { name: "Reload latest Assessment", exact: true }).click();
      await page
        .getByText("Latest assessment loaded. Review its complete ordered Entries.", {
          exact: true,
        })
        .waitFor();
      const reloaded = await orderedEntries(page);
      assert.deepEqual(
        reloaded.map((entry) => entry.id),
        expected.map((entry) => entry.id),
      );
      assert.deepEqual(
        reloaded.map((entry) => entry.title),
        expected.map((entry) => entry.title),
      );

      await page.evaluate(() => {
        const target = document.getElementById("bloom-workflow");
        window.BloomClassificationWorkflow.mountBloomLibraryBrowse(target);
      });
      await page.getByRole("heading", { name: "Browse Question Library", exact: true }).waitFor();
      await page.getByText("No published questions match these filters", { exact: true }).waitFor();
      await page.waitForFunction(() =>
        window.bloomLibrarySearches.some(
          (search) =>
            search.bloomCognitiveProcess === null && search.bloomKnowledgeDimension === null,
        ),
      );
      await page.getByText("Retrieve relevant knowledge", { exact: true }).waitFor();
      await page
        .getByText("Terminology, specific details, and discrete elements", { exact: true })
        .waitFor();
      const bloomFilters = page.getByRole("group", { name: "Bloom filters", exact: true });
      const cognitiveProcess = bloomFilters.getByRole("combobox", { name: "Cognitive Process" });
      const knowledgeDimension = bloomFilters.getByRole("combobox", {
        name: "Knowledge Dimension",
      });
      await cognitiveProcess.selectOption("Remember");
      await cognitiveProcess.evaluate((element) => {
        element.dispatchEvent(new Event("change", { bubbles: true }));
      });
      await page.waitForFunction(() =>
        window.bloomLibrarySearches.some((search) => search.bloomCognitiveProcess === "Remember"),
      );
      await page.getByRole("heading", { name: "Cell division", exact: true }).waitFor();
      await knowledgeDimension.selectOption("Factual Knowledge");
      await knowledgeDimension.evaluate((element) => {
        element.dispatchEvent(new Event("change", { bubbles: true }));
      });
      await page.waitForFunction(() =>
        window.bloomLibrarySearches.some(
          (search) =>
            search.bloomCognitiveProcess === "Remember" &&
            search.bloomKnowledgeDimension === "Factual Knowledge",
        ),
      );
      await page.getByRole("heading", { name: "Cell division", exact: true }).waitFor();
      assert.deepEqual(pageErrors, []);
    } finally {
      await browser.close();
      await harnessServer.close();
    }
  },
);
