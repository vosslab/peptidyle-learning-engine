// A Course Instance Assessment carries delivery settings and is what a Student opens.

import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";

function pageDocument(targetId) {
  return `<!doctype html>
<html>
<head><style>body { margin: 0; font: 16px/1.4 sans-serif; }</style></head>
<body><div id="${targetId}"></div></body>
</html>`;
}

async function loadBundle(entryName, globalName) {
  const result = await build({
    bundle: true,
    entryPoints: [new URL(`../support/${entryName}`, import.meta.url).pathname],
    format: "iife",
    globalName,
    outfile: `${entryName}.js`,
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
  if (javascript === undefined) throw new Error(`${entryName} bundle is missing JavaScript.`);
  return Buffer.from(javascript.contents).toString("utf8");
}

test("a course instance assessment has due date release status and student availability", async () => {
  const bundle = await loadBundle("blueprint_incorporation_harness.tsx", "CourseInstanceDelivery");
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
    await page.setContent(pageDocument("course-delivery"), { waitUntil: "load" });
    await page.addScriptTag({ content: bundle });
    await page.evaluate(() => {
      const target = document.getElementById("course-delivery");
      window.CourseInstanceDelivery.mountCourseInstanceDelivery(target);
    });
    await page.getByRole("heading", { name: "Assessment and delivery", exact: true }).waitFor();
    await page.getByText("Release status", { exact: true }).waitFor();
    await page.getByText("Unreleased", { exact: true }).waitFor();
    const available = page.getByRole("group", { name: "Available date and time", exact: true });
    await available.getByRole("textbox", { name: "Available date", exact: false }).waitFor();
    const due = page.getByRole("group", { name: "Due date and time", exact: true });
    assert.equal(
      await due.getByRole("textbox", { name: "Due date", exact: false }).inputValue(),
      "2026-10-15",
    );
    await page.getByRole("group", { name: "Closes date and time", exact: true }).waitFor();
    await page.getByRole("button", { name: "Release assessment", exact: true }).waitFor();
    await page.getByRole("link", { name: "Open Student View", exact: true }).waitFor();
  } finally {
    await browser.close();
  }
});

test("assessment properties show the calculated time limit and save an instructor override", async () => {
  const bundle = await loadBundle("blueprint_incorporation_harness.tsx", "CourseInstanceDelivery");
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
    await page.setContent(pageDocument("course-delivery"), { waitUntil: "load" });
    await page.addScriptTag({ content: bundle });
    const calculatedDefault =
      "Calculated default: 2 minutes for 1 Questions (1.5 minutes per Question, rounded up).";
    const override = () =>
      page.getByRole("spinbutton", { name: /Assessment duration override in minutes/ });

    async function openProperties() {
      await page.evaluate(() => {
        const target = document.getElementById("course-delivery");
        window.CourseInstanceDelivery.mountAssessmentDurationOverride(target);
      });
      await page.getByRole("heading", { name: "Assessment and delivery", exact: true }).waitFor();
    }

    await openProperties();
    await page.getByText(calculatedDefault).waitFor();
    assert.equal(await override().inputValue(), "");

    await override().fill("20");
    await page.waitForFunction(() => window.assessmentDurationSaves.at(-1) === 1200, undefined, {
      timeout: 3000,
    });
    await page
      .getByText("Assessment Properties saved. Future Attempts use the current values.")
      .waitFor();

    await openProperties();
    await page.getByText(calculatedDefault).waitFor();
    assert.equal(await override().inputValue(), "20");

    await override().fill("");
    await page.waitForFunction(() => window.assessmentDurationSaves.at(-1) === null, undefined, {
      timeout: 3000,
    });

    await openProperties();
    await page.getByText(calculatedDefault).waitFor();
    assert.equal(await override().inputValue(), "");
    await page.getByText("Leave the override blank to use this calculated default.").waitFor();
  } finally {
    await browser.close();
  }
});

test("release validation explains missing questions, the question limit, and an unavailable question", async () => {
  const bundle = await loadBundle("blueprint_incorporation_harness.tsx", "CourseInstanceDelivery");
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));
    await page.setContent(pageDocument("course-delivery"), { waitUntil: "load" });
    await page.addScriptTag({ content: bundle });
    await page.evaluate(() => {
      const target = document.getElementById("course-delivery");
      window.CourseInstanceDelivery.mountAssessmentReleaseQuestionIssues(target);
    });
    await page.getByRole("heading", { name: "Assessment and delivery", exact: true }).waitFor();
    await page.getByRole("button", { name: "Check release readiness", exact: true }).click();
    await page.getByRole("heading", { name: "Release needs attention", exact: true }).waitFor();
    await page
      .getByText("Select and save at least one published Question.", { exact: true })
      .waitFor();
    await page
      .getByText(
        "Reduce the Assessment to at most 250 delivered Questions, counting each Pool's selected Questions.",
        { exact: true },
      )
      .waitFor();
    await page
      .getByText("A selected Question is unavailable. Review and save the Questions list.", {
        exact: true,
      })
      .waitFor();
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser.close();
  }
});
