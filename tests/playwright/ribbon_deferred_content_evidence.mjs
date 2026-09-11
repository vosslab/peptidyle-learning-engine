// ribbon_deferred_content_evidence.mjs - compiled-harness deferred-content evidence.
// It exercises current source composition in a controlled browser fixture; it
// does not build dist/ or replace real-stack browser acceptance.

import assert from "node:assert/strict";
import { once } from "node:events";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";

import { chromium } from "playwright";

import { bundleRibbonDeferredContentHarness } from "../support/ribbon_deferred_content_loader.ts";

const css = [
  readFileSync(new URL("../../src/style.css", import.meta.url), "utf8"),
  readFileSync(new URL("../../src/styles/accessibility.css", import.meta.url), "utf8"),
].join("\n");
const bundle = await bundleRibbonDeferredContentHarness();
const RIBBON_ROOT_SELECTOR = ".ple-app-ribbon";
const WORKSPACE_CASES = new Set(["policies", "workspace"]);
const RETIRED_NAVIGATION_SELECTOR = [
  '[aria-label="Course management"]',
  '[aria-label="Assignment workspace"]',
  ".course-management-nav",
  ".assignment-workspace-nav",
  ".course-management-frame",
  "[data-course-management-frame]",
].join(", ");
const server = createServer((_request, response) => {
  response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
  response.end(
    [
      "<!doctype html><html><head><style>",
      css,
      "\n",
      bundle.stylesheet,
      '</style></head><body><div id="root"></div></body></html>',
    ].join(""),
  );
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const address = server.address();
if (address === null || typeof address === "string")
  throw new Error("Deferred-content evidence server did not bind TCP.");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  page.setDefaultTimeout(5_000);
  const pageErrors = [];
  const consoleErrors = [];
  page.on("pageerror", (error) => {
    pageErrors.push(error.message);
    console.error(`Deferred-content evidence page error at ${page.url()}: ${error.message}`);
  });
  page.on("console", (message) => {
    if (message.type() === "error") {
      consoleErrors.push(message.text());
      console.error(
        `Deferred-content evidence browser console at ${page.url()}: ${message.text()}`,
      );
    }
  });
  await page.goto(`http://127.0.0.1:${String(address.port)}/`);
  await page.addScriptTag({ content: Buffer.from(bundle.javascript).toString("utf8") });
  await page.waitForFunction(
    () => typeof window.PleRibbonDeferredContent?.mountRibbonDeferredContentHarness === "function",
  );
  await page.evaluate(() => {
    const root = document.querySelector("#root");
    if (!(root instanceof HTMLElement))
      throw new Error("Deferred-content evidence root is missing.");
    window.ribbonDeferredContent =
      window.PleRibbonDeferredContent.mountRibbonDeferredContentHarness(root);
  });
  await page.waitForFunction(() => window.ribbonDeferredContent.ready());
  const assertHarnessRibbon = async (caseName, taskRowReserved) => {
    const ribbonRoot = page.locator(RIBBON_ROOT_SELECTOR);
    await ribbonRoot.waitFor({ state: "visible" });
    assert.equal(await ribbonRoot.count(), 1, `${caseName} has exactly one harness Ribbon root`);
    assert.equal(
      await page.getByRole("region", { name: "PLE application Ribbon" }).count(),
      1,
      `${caseName} exposes the harness Ribbon root as its labelled region`,
    );
    assert.equal(
      await page.getByRole("navigation", { name: "Ribbon tabs" }).count(),
      1,
      `${caseName} has exactly one Ribbon tabs navigation landmark`,
    );
    assert.equal(
      await page.getByRole("navigation", { name: "Ribbon tasks" }).count(),
      taskRowReserved ? 1 : 0,
      `${caseName} exposes the Task Row only when declared route topology reserves it`,
    );
    const expectedTaskRow = taskRowReserved ? "reserved" : "absent";
    assert.equal(
      await ribbonRoot.getAttribute("data-ribbon-task-row"),
      expectedTaskRow,
      `${caseName} reports its Ribbon Task Row topology`,
    );
    assert.equal(
      await page.locator(".ple-shell-frame").getAttribute("data-ribbon-task-row"),
      expectedTaskRow,
      `${caseName} shares the same topology with the shell frame`,
    );
  };
  // The settled M2 Product Courses route reserves its declared task row.
  await assertHarnessRibbon("initial route", true);

  const cases = [
    [
      "policies",
      "assignmentWorkspaceGate",
      "assignmentWorkspaceGate",
      "Loading assignment workspace...",
      undefined,
      "scopeCourse",
      "getLiveAssignmentWorkspace",
      1,
      1,
      true,
    ],
    [
      "preview",
      "assignmentPreview",
      "assignmentPreview",
      "Loading assignment delivery check...",
      "Assignment delivery check",
      "scopeCourse",
      "getLiveAssignmentPreview",
      0,
      1,
      false,
    ],
    [
      "workspace",
      "assignmentWorkspaceGate",
      "assignmentWorkspaceGate",
      "Loading assignment workspace...",
      undefined,
      "scopeCourse",
      "getLiveAssignmentWorkspace",
      1,
      1,
      true,
    ],
    [
      "roster",
      "courseRoster",
      "courseRoster",
      "Loading course roster...",
      "Students",
      "scopeCourse",
      "getLiveCourseRoster",
      1,
      1,
      false,
    ],
    [
      "teaching",
      "teachingOperations",
      "teachingOperations",
      "Loading teaching operations...",
      "Teaching operations",
      "scopeCourse",
      "listCourseInstructors",
      0,
      1,
      false,
    ],
  ];
  for (const [
    caseName,
    surface,
    pendingSurface,
    pending,
    heading,
    scopeCounter,
    downstream,
    expectedBeforeRelease,
    expectedAfterRelease,
    taskRowReserved,
  ] of cases) {
    await page.evaluate((name) => window.ribbonDeferredContent.navigate(name), caseName);
    const pendingStatus = page
      .locator(`[data-route-surface="${pendingSurface}"]`)
      .locator('[role="status"]')
      .filter({ hasText: pending });
    try {
      await pendingStatus.waitFor({ state: "visible", timeout: 5_000 });
    } catch (error) {
      const currentPath = await page.evaluate(() =>
        document.querySelector("[data-current-path]")?.getAttribute("data-current-path"),
      );
      const visibleText = await page.locator("body").innerText();
      throw new Error(
        `${caseName} did not expose its deferred surface at ${String(currentPath)}: ${visibleText}`,
        { cause: error },
      );
    }
    assert.equal(
      await pendingStatus.innerText(),
      pending,
      `${caseName} exposes the exact deferred status text in its current route surface`,
    );
    await assertHarnessRibbon(`${caseName} while deferred`, taskRowReserved);
    assert.equal(
      await page.locator(RETIRED_NAVIGATION_SELECTOR).count(),
      0,
      `${caseName} mounts no retired course or workspace navigation`,
    );
    assert.equal(
      await page.locator('[role="alert"]').count(),
      0,
      `${caseName} does not commit a denied/unavailable alert before scope release`,
    );
    assert.equal(
      await page.evaluate(
        ([name, counter]) => window.ribbonDeferredContent.count(name, counter),
        [caseName, downstream],
      ),
      expectedBeforeRelease,
      `${caseName} starts exactly its route-owned downstream work before scope release`,
    );
    assert.equal(
      await page.evaluate(
        ([name, counter]) => window.ribbonDeferredContent.count(name, counter),
        [caseName, scopeCounter],
      ),
      1,
      `${caseName} has one explicitly deferred scope request`,
    );
    // Workspace content owns its loader independently of the deferred Course
    // presentation scope; the stable shell must tolerate both loads together.
    await page.evaluate((name) => window.ribbonDeferredContent.release(name), caseName);
    await page.locator(`[data-route-surface="${surface}"]`).first().waitFor({ state: "attached" });
    if (WORKSPACE_CASES.has(caseName)) {
      const workspaceGate = page.locator('[data-route-surface="assignmentWorkspaceGate"]');
      const workspaceStatus = workspaceGate.getByRole("status");
      const workspaceEyebrow = workspaceGate.locator(".eyebrow");
      await workspaceStatus.waitFor({ state: "visible" });
      assert.equal(
        await workspaceStatus.innerText(),
        pending,
        `${caseName} exposes the exact inner workspace-gate status after course scope release`,
      );
      assert.equal(
        await workspaceEyebrow.textContent(),
        "Instructor assignment workspace",
        `${caseName} exposes the exact inner workspace-gate eyebrow after course scope release`,
      );
      assert.equal(
        await page.locator('[data-route-surface="assignmentWorkspace"] [role="status"]').count(),
        0,
        `${caseName} replaces the outer deferred scope fallback with its inner workspace gate`,
      );
      await page.waitForFunction(
        (name) => window.ribbonDeferredContent.count(name, "getLiveAssignmentWorkspace") === 1,
        caseName,
      );
      assert.equal(
        await page.evaluate(
          (name) => window.ribbonDeferredContent.count(name, "getLiveAssignmentWorkspace"),
          caseName,
        ),
        1,
        `${caseName} starts exactly one workspace-detail request after its route identity resolves`,
      );
    } else {
      await page.getByRole("heading", { name: heading }).first().waitFor({ state: "visible" });
    }
    await page.waitForFunction(
      ([name, counter, expectedCount]) =>
        window.ribbonDeferredContent.count(name, counter) === expectedCount,
      [caseName, downstream, expectedAfterRelease],
    );
    assert.equal(
      await page.evaluate(
        ([name, counter]) => window.ribbonDeferredContent.count(name, counter),
        [caseName, downstream],
      ),
      expectedAfterRelease,
      `${caseName} initializes exactly its expected downstream operation after release`,
    );
    if (caseName === "teaching") {
      assert.equal(
        await page.evaluate(() =>
          window.ribbonDeferredContent.count("teaching", "listInstructorCourseInvitations"),
        ),
        1,
        "teaching starts its paired invitation load once",
      );
    }
    await assertHarnessRibbon(`${caseName} after scope release`, taskRowReserved);
    assert.equal(
      await page.locator(RETIRED_NAVIGATION_SELECTOR).count(),
      0,
      `${caseName} retains no retired course or workspace navigation after scope release`,
    );
  }
  assert.deepEqual(pageErrors, [], "compiled-harness routed components produced no browser errors");
  assert.deepEqual(
    consoleErrors,
    [],
    "compiled-harness routed components produced no console errors",
  );
  console.log(
    "Compiled-harness deferred-content evidence passed: current-source page families preserve " +
      "their declared pre/post-scope initialization; not dist or real-stack browser acceptance.",
  );
} finally {
  await browser.close();
  server.close();
}
