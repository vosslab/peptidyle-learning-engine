// An Instructor saves 1.5X and 2X time for one active Student.

import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";

async function loadBundle() {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL("../support/blueprint_incorporation_harness.tsx", import.meta.url).pathname,
    ],
    format: "iife",
    globalName: "AssessmentStudentTime",
    outfile: "assessment_student_time_harness.js",
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
    throw new Error("Student time bundle is missing JavaScript.");
  }
  return Buffer.from(javascript.contents).toString("utf8");
}

test("an instructor saves 1.5X and 2X time for one student", async () => {
  const bundle = await loadBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1100, height: 900 } });
    await page.setContent(
      '<!doctype html><html><body><div id="student-time"></div></body></html>',
      { waitUntil: "load" },
    );
    await page.addScriptTag({ content: bundle });
    await page.evaluate(() => {
      const target = document.getElementById("student-time");
      window.AssessmentStudentTime.mountAssessmentStudentTime(target);
    });
    await page.getByRole("button", { name: "Manage Student time settings", exact: true }).click();
    await page
      .getByText(
        "Apply a time multiplier after the Assessment's base time limit. Effective time is rounded up to a whole second and capped at 24 hours.",
      )
      .waitFor();
    const roster = page.getByRole("combobox", { name: "Student roster ID", exact: true });
    await roster.waitFor();
    assert.deepEqual(await roster.locator("option").allTextContents(), [
      "Select an active Student",
      "AVERY",
    ]);
    await roster.selectOption("AVERY");
    await page.getByRole("group", { name: "Time for Student AVERY", exact: true }).waitFor();
    const multiplier = page.getByRole("combobox", { name: "Time multiplier", exact: true });
    assert.equal(await multiplier.inputValue(), "standard");
    await multiplier.selectOption("1.5");
    await page.getByRole("button", { name: "Save Student time settings", exact: true }).click();
    await page
      .getByText(
        "Student time settings saved. Existing active Attempts keep their original time limit and deadline.",
      )
      .waitFor();
    assert.equal(await multiplier.inputValue(), "1.5");
    await multiplier.selectOption("2");
    await page.getByRole("button", { name: "Save Student time settings", exact: true }).click();
    await page.waitForFunction(() => window.assessmentTimeSaves.length === 2);
    assert.equal(await multiplier.inputValue(), "2");
    assert.deepEqual(await page.evaluate(() => window.assessmentTimeSaves), [
      {
        courseInstanceId: "CI7K3M2QAZ",
        assessmentId: "A8H4N6PA6",
        rosterId: "AVERY",
        timeMultiplier: 1.5,
        expectedAccommodationEditNumber: null,
      },
      {
        courseInstanceId: "CI7K3M2QAZ",
        assessmentId: "A8H4N6PA6",
        rosterId: "AVERY",
        timeMultiplier: 2,
        expectedAccommodationEditNumber: "1",
      },
    ]);
  } finally {
    await browser.close();
  }
});
