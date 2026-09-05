// ribbon_m11_deferred_content_evidence.mjs - compiled-harness deferred-content evidence.
// It exercises current source composition in a controlled browser fixture; it
// does not build dist/ or replace real-stack browser acceptance.

import assert from "node:assert/strict";
import { once } from "node:events";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";

import { chromium } from "playwright";

import { bundleM11Harness } from "../support/ribbon_m11_deferred_content_loader.ts";

const css = [
  readFileSync(new URL("../../src/style.css", import.meta.url), "utf8"),
  readFileSync(new URL("../../src/styles/accessibility.css", import.meta.url), "utf8"),
].join("\n");
const bundle = await bundleM11Harness();
const RIBBON_ROOT_SELECTOR = ".ple-app-ribbon";
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
    () =>
      typeof window.PleRibbonM11DeferredContent?.mountRibbonM11DeferredContentHarness ===
      "function",
  );
  await page.evaluate(() => {
    const root = document.querySelector("#root");
    if (!(root instanceof HTMLElement))
      throw new Error("Deferred-content evidence root is missing.");
    window.ribbonM11 =
      window.PleRibbonM11DeferredContent.mountRibbonM11DeferredContentHarness(root);
  });
  await page.waitForFunction(() => window.ribbonM11.ready());
  const assertHarnessRibbon = async (caseName) => {
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
      1,
      `${caseName} has exactly one Ribbon tasks navigation landmark`,
    );
  };
  await assertHarnessRibbon("initial route");

  const cases = [
    [
      "summary",
      "assignmentAttemptSummary",
      "assignmentAttemptSummary",
      "Loading your recorded responses...",
      "Assignment Attempt summary",
      "scopeSummary",
      "postMountAssignmentAttemptSummary",
      0,
    ],
    [
      "preview",
      "assignmentPreview",
      "assignmentPreview",
      "Loading assignment delivery check...",
      "Assignment delivery check",
      "scopeCourse",
      "resolveNavigation",
      1,
    ],
    [
      "workspace",
      "assignmentWorkspaceGate",
      "assignmentWorkspace",
      "Loading assignment workspace...",
      undefined,
      "scopeCourse",
      "resolveNavigation",
      1,
    ],
    [
      "roster",
      "courseRoster",
      "courseRoster",
      "Loading course roster...",
      "Students",
      "scopeCourse",
      "listCourseRoster",
      1,
    ],
    [
      "teaching",
      "teachingOperations",
      "teachingOperations",
      "Loading teaching operations...",
      "Teaching operations",
      "scopeCourse",
      "listCourseInstructors",
      1,
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
    expected,
  ] of cases) {
    await page.evaluate((name) => window.ribbonM11.navigate(name), caseName);
    const pendingStatus = page
      .locator(`[data-route-surface="${pendingSurface}"]`)
      .locator('[role="status"]')
      .filter({ hasText: pending });
    await pendingStatus.waitFor({ state: "visible" });
    assert.equal(
      await pendingStatus.innerText(),
      pending,
      `${caseName} exposes the exact deferred status text in its current route surface`,
    );
    await assertHarnessRibbon(`${caseName} while deferred`);
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
        ([name, counter]) => window.ribbonM11.count(name, counter),
        [caseName, downstream],
      ),
      0,
      `${caseName} starts no downstream page transport before scope release`,
    );
    assert.equal(
      await page.evaluate(
        ([name, counter]) => window.ribbonM11.count(name, counter),
        [caseName, scopeCounter],
      ),
      1,
      `${caseName} has one explicitly deferred scope request`,
    );
    if (caseName === "workspace") {
      assert.equal(
        await page.locator('[data-route-surface="assignmentWorkspaceGate"]').count(),
        0,
        "workspace does not mount its inner workspace gate before the course scope releases",
      );
    }
    await page.evaluate((name) => window.ribbonM11.release(name), caseName);
    await page.locator(`[data-route-surface="${surface}"]`).first().waitFor({ state: "attached" });
    if (caseName === "workspace") {
      const workspaceGate = page.locator('[data-route-surface="assignmentWorkspaceGate"]');
      const workspaceStatus = workspaceGate.getByRole("status");
      const workspaceEyebrow = workspaceGate.locator(".eyebrow");
      await workspaceStatus.waitFor({ state: "visible" });
      assert.equal(
        await workspaceStatus.innerText(),
        pending,
        "workspace exposes the exact inner workspace-gate status after course scope release",
      );
      assert.equal(
        await workspaceEyebrow.textContent(),
        "Instructor assignment workspace",
        "workspace exposes the exact inner workspace-gate eyebrow after course scope release",
      );
      assert.equal(
        await page.locator('[data-route-surface="assignmentWorkspace"] [role="status"]').count(),
        0,
        "workspace replaces the outer deferred scope fallback with its inner workspace gate",
      );
      await page.waitForFunction(
        (name) =>
          window.ribbonM11.count(name, "resolveNavigation") === 1 &&
          window.ribbonM11.count(name, "getAssignmentWorkspace") === 1,
        caseName,
      );
      assert.equal(
        await page.evaluate(
          (name) => window.ribbonM11.count(name, "getAssignmentWorkspace"),
          caseName,
        ),
        1,
        "workspace starts exactly one workspace-detail request after its route identity resolves",
      );
    } else {
      await page.getByRole("heading", { name: heading }).first().waitFor({ state: "visible" });
    }
    await page.waitForFunction(
      ([name, counter, expectedCount]) => window.ribbonM11.count(name, counter) === expectedCount,
      [caseName, downstream, expected],
    );
    assert.equal(
      await page.evaluate(
        ([name, counter]) => window.ribbonM11.count(name, counter),
        [caseName, downstream],
      ),
      expected,
      `${caseName} initializes exactly its expected downstream operation after release`,
    );
    if (caseName === "teaching") {
      assert.equal(
        await page.evaluate(() =>
          window.ribbonM11.count("teaching", "listInstructorCourseInvitations"),
        ),
        1,
        "teaching starts its paired invitation load once",
      );
    }
    await assertHarnessRibbon(`${caseName} after scope release`);
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
    "Compiled-harness deferred-content evidence passed: five current-source page families " +
      "wait for scope before one local initialization; not dist or real-stack browser acceptance.",
  );
} finally {
  await browser.close();
  server.close();
}
