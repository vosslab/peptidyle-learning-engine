// A Course Instance Assessment delivers its Question to the Student on the Attempt page.

import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";

const ATTEMPT_ID = "11111111-1111-4111-8111-666666666666";
const PROMPT = "Which membrane lipid forms the bilayer?";

function pageDocument() {
  return `<!doctype html>
<html>
<head><style>body { margin: 0; font: 16px/1.4 sans-serif; }</style></head>
<body><div id="assessment-attempt"></div></body>
</html>`;
}

async function loadBundle() {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL("../support/assessment_attempt_delivery_harness.tsx", import.meta.url).pathname,
    ],
    format: "iife",
    globalName: "AssessmentAttemptDelivery",
    outfile: "assessment_attempt_delivery_harness.js",
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
    throw new Error("Assessment Attempt delivery bundle is missing JavaScript.");
  }
  return Buffer.from(javascript.contents).toString("utf8");
}

test("a course instance assessment delivers its question to the student", async () => {
  const bundle = await loadBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
    await page.setContent(pageDocument(), { waitUntil: "load" });
    await page.addScriptTag({ content: bundle });
    await page.evaluate((prompt) => {
      const target = document.getElementById("assessment-attempt");
      window.AssessmentAttemptDelivery.mountAssessmentAttemptDelivery(target, prompt);
    }, PROMPT);

    await page.getByRole("heading", { name: "Membrane review", exact: true }).waitFor();
    await page.getByText("Weekly Assignment · Attempt 1", { exact: true }).waitFor();
    await page.getByText(PROMPT, { exact: true }).waitFor();
    const openAttemptText = await page.locator("#assessment-attempt").innerText();
    assert.doesNotMatch(
      openAttemptText,
      /Marked correct|Marked not correct|Your score|Correct answer|points earned/i,
    );
    const calls = await page.evaluate(() => ({
      context: window.assessmentAttemptContextIds,
      progress: window.assessmentAttemptProgressIds,
      presentation: window.assessmentAttemptPresentationCalls,
    }));
    assert.deepEqual(calls.context, [ATTEMPT_ID]);
    assert.deepEqual(calls.progress, [ATTEMPT_ID]);
    assert.deepEqual(calls.presentation, [{ assessmentAttemptId: ATTEMPT_ID, position: 1 }]);
  } finally {
    await browser.close();
  }
});

test("an assessment attempt has a time limit", async () => {
  const bundle = await loadBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
    await page.setContent(pageDocument(), { waitUntil: "load" });
    await page.addScriptTag({ content: bundle });
    await page.evaluate(
      ({ prompt, timing }) => {
        const target = document.getElementById("assessment-attempt");
        window.AssessmentAttemptDelivery.mountAssessmentAttemptDelivery(target, prompt, timing);
      },
      {
        prompt: PROMPT,
        timing: {
          expiresAt: Date.parse("2026-09-29T18:00:00Z"),
          timerRemainingMilliseconds: 15 * 60 * 1000,
        },
      },
    );
    await page.getByRole("heading", { name: "Membrane review", exact: true }).waitFor();
    await page.waitForFunction(() => {
      const text = document.querySelector('[role="timer"]')?.textContent ?? "";
      return text.includes("remaining") || text.includes("time limit");
    });
    const timerText = await page.getByRole("timer").innerText();
    assert.equal(timerText.includes("Untimed"), false);
    await page.getByText("Saved responses submit automatically at").waitFor();
  } finally {
    await browser.close();
  }
});
