// Core teaching sections remain available across Course and Assessment workflows.

import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import { openRibbonShellEvidencePage } from "./ribbon_shell_helpers.mjs";

const TEACHING_TASK_SENTENCE = "Assessment authoring exposes its core teaching tasks.";

async function loadQuestionEditorBundle() {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL("../support/bloom_classification_workflow_harness.tsx", import.meta.url).pathname,
    ],
    format: "iife",
    globalName: "BloomClassificationWorkflow",
    outfile: "teaching_task_question_editor.js",
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
    throw new Error("Assessment Question editor bundle is missing JavaScript.");
  }
  return Buffer.from(javascript.contents).toString("utf8");
}

test(TEACHING_TASK_SENTENCE, { timeout: 180_000 }, async () => {
  const shell = await openRibbonShellEvidencePage();
  try {
    await shell.page.evaluate(() => window.ribbonShell.releaseSession());
    await shell.page.evaluate(() => window.ribbonShell.currentNavigate("/courses/CI7K3M2QAZ"));
    const course = shell.page.locator(
      '[data-m10-case="current-production"] [data-route-surface="courseInstance"]',
    );
    await course.getByRole("heading", { name: "Assessments", exact: true }).waitFor({
      state: "visible",
    });
    await shell.page.waitForFunction(() => window.ribbonShell.scopeRequestCount("CI7K3M2QAZ") >= 1);
    await shell.page.evaluate(() => window.ribbonShell.releaseCourseScope("CI7K3M2QAZ"));

    await shell.page.evaluate(() =>
      window.ribbonShell.currentNavigate("/instructor/courses/CI7K3M2QAZ/assessments/new"),
    );
    const create = shell.page.locator(
      '[data-m10-case="current-production"] [data-route-surface="assessmentCreate"]',
    );
    await create.getByRole("heading", { name: "Create an Assessment", exact: true }).waitFor({
      state: "visible",
    });
    await create.getByText("How do you want to start?", { exact: true }).waitFor({
      state: "visible",
    });
    await create.locator("#assessment-title").waitFor({ state: "visible" });
    const questionBundle = await loadQuestionEditorBundle();
    await shell.page.evaluate(() => {
      const host = document.createElement("div");
      host.id = "bloom-workflow";
      document.body.append(host);
    });
    await shell.page.addScriptTag({ content: questionBundle });
    await shell.page.waitForFunction(
      () => typeof window.BloomClassificationWorkflow?.mountBloomAssessmentWorkflow === "function",
    );
    await shell.page.evaluate(() => {
      const target = document.getElementById("bloom-workflow");
      if (!(target instanceof HTMLElement)) {
        throw new Error("Assessment Question editor root is missing.");
      }
      window.BloomClassificationWorkflow.mountBloomAssessmentWorkflow(target);
    });
    const questionEditor = shell.page.locator("#bloom-workflow");
    await questionEditor
      .getByRole("heading", { name: "Assessment Question Editor", exact: true })
      .waitFor();
    await shell.page.waitForFunction(() => {
      const list = document.querySelector(
        '#bloom-workflow ol[aria-label="Ordered Assessment Entries"]',
      );
      return list !== null && list.querySelectorAll(":scope > li").length >= 2;
    });
    const editor = await questionEditor.evaluate((root) => ({
      panelHeadings: [...root.querySelectorAll(".assessment-editor-panel h2")].map((heading) =>
        (heading.textContent ?? "").trim(),
      ),
      entryCount: root.querySelectorAll('ol[aria-label="Ordered Assessment Entries"] > li').length,
    }));
    assert.ok(editor.panelHeadings.length >= 2);
    assert.ok(editor.entryCount >= 2);
    const addIndex = editor.panelHeadings.indexOf("Available published Questions");
    const poolIndex = editor.panelHeadings.indexOf("Import a reusable Question Pool");
    assert.ok(addIndex >= 0 && poolIndex > addIndex);

    assert.deepEqual(shell.pageErrors, []);
    assert.deepEqual(shell.consoleErrors, []);
  } finally {
    await shell.browser.close();
    await shell.harnessServer.close();
  }
});
