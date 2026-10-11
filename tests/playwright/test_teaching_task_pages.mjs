// Course Instructors can reach the Assessment creation form.

import assert from "node:assert/strict";
import test from "node:test";

import { openRibbonShellEvidencePage } from "./ribbon_shell_helpers.mjs";

const TEACHING_TASK_SENTENCE = "A Course Instructor can open Assessment creation.";

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

    await course.getByRole("link", { name: "Create Assessment", exact: true }).click();
    const create = shell.page.locator(
      '[data-m10-case="current-production"] [data-route-surface="assessmentCreate"]',
    );
    await create.getByRole("heading", { name: "Create an Assessment", exact: true }).waitFor({
      state: "visible",
    });
    await create.getByRole("textbox", { name: "Assessment title", exact: true }).waitFor({
      state: "visible",
    });

    assert.deepEqual(shell.pageErrors, []);
    assert.deepEqual(shell.consoleErrors, []);
  } finally {
    await shell.browser.close();
    await shell.harnessServer.close();
  }
});
