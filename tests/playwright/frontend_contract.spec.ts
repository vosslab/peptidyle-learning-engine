// frontend_contract.spec.ts - built-artifact proof for the WP-C9 reference slice.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";

declare global {
  interface Window {
    __PLE_ROUTE_FAILURE_TEST__?: () => boolean;
  }
}

const IDS = {
  course: "0198e000-0000-7000-8000-000000000014",
  assignment: "0198e000-0000-7000-8000-000000000006",
  run: "0198e000-0000-7000-8000-000000000023",
  problem: "0198e000-0000-7000-8000-000000000003",
  version: "0198e000-0000-7000-8000-000000000004",
  workspace: "0198e000-0000-7000-8000-000000000002",
} as const;

const SAVED_ATTEMPT_KEY =
  "ple:attempt:0198e000-0000-7000-8000-000000000001:0198e000-0000-7000-8000-000000000023:0198e000-0000-7000-8000-000000000033";
const SAVED_ATTEMPT_BUFFER = JSON.stringify({
  response: { kind: "multipleChoice", selected: ["carbonyl"] },
  idempotencyKey: "saved-response-key",
});

async function navigateWithinSpa(page: Page, pathname: string): Promise<void> {
  await page.evaluate((nextPath) => {
    history.pushState({}, "", nextPath);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, pathname);
}

async function tabTo(page: Page, target: ReturnType<Page["locator"]>, limit = 20): Promise<void> {
  for (let index = 0; index < limit; index += 1) {
    if (await target.evaluate((element) => document.activeElement === element)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error(`Tab did not reach ${await target.getAttribute("aria-label")}`);
}

async function expectNoBlockingAxeViolations(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).include("main").analyze();
  const blocking = results.violations
    .filter((violation) => violation.impact === "critical" || violation.impact === "serious")
    .map((violation) => ({
      id: violation.id,
      impact: violation.impact,
      targets: violation.nodes.flatMap((node) => node.target),
    }));
  expect(blocking).toEqual([]);
}

test("all product routes resolve inside the persistent shell", async ({ page }) => {
  const routes = [
    { path: "/", surface: "courses" },
    { path: `/courses/${IDS.course}`, surface: "courseAssignments" },
    {
      path: `/courses/${IDS.course}/assignments/${IDS.assignment}`,
      surface: "assignmentOverview",
    },
    { path: `/runs/${IDS.run}`, surface: "runAttempt" },
    { path: `/runs/${IDS.run}/summary`, surface: "runSummary" },
    { path: "/library", surface: "library" },
    {
      path: `/library/${IDS.problem}/versions/${IDS.version}`,
      surface: "problemDetail",
    },
    { path: "/workspace", surface: "workspaceList" },
    { path: `/workspace/${IDS.workspace}`, surface: "workspaceEditor" },
    {
      path: `/instructor/courses/${IDS.course}/assignments/${IDS.assignment}/edit`,
      surface: "assignmentEditor",
    },
    { path: `/instructor/courses/${IDS.course}/gradebook`, surface: "gradebook" },
    {
      path: `/instructor/courses/${IDS.course}/appearance`,
      surface: "courseAppearance",
    },
  ];

  await page.goto("/");
  await expect(page.getByRole("link", { name: "Peptidyle home" })).toBeVisible();

  for (const route of routes) {
    await navigateWithinSpa(page, route.path);
    if (route.surface.startsWith("workspace")) {
      await expect(
        page.getByRole("heading", {
          name: "Workspace authoring is not available for this account",
        }),
      ).toBeVisible();
    } else {
      await expect(page.locator(`[data-route-surface="${route.surface}"]`)).toBeVisible();
    }
    await expect(page.locator("header.site-header")).toBeVisible();
  }
});

test("student navigation stays learner-focused and link navigation focuses main content", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("link", { name: "Workspace" })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Courses" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Library" })).toBeVisible();

  await page.getByRole("link", { name: "Library" }).click();
  await expect(page.locator("#main-content")).toBeFocused();
  await expect(page.locator('[data-route-surface="library"]')).toBeVisible();

  await page.getByRole("link", { name: "Skip to learning content" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
});

test("manual student workspace navigation mounts no authoring transport", async ({ page }) => {
  const requests: string[] = [];
  page.on("request", (request) => requests.push(new URL(request.url()).pathname));

  await page.goto("/");
  requests.length = 0;
  await navigateWithinSpa(page, `/workspace/${IDS.workspace}`);

  await expect(
    page.getByRole("heading", { name: "Workspace authoring is not available for this account" }),
  ).toBeVisible();
  expect(
    requests.filter(
      (path) => path.startsWith("/api/workspaces") || path.includes("author-preview"),
    ),
  ).toEqual([]);
});

test("a route failure keeps the shell usable and omits raw exception details", async ({ page }) => {
  await page.addInitScript(() => {
    let shouldFail = true;
    window.__PLE_ROUTE_FAILURE_TEST__ = (): boolean => {
      const requested = shouldFail;
      shouldFail = false;
      return requested;
    };
  });
  await page.goto("/");

  await expect(page.locator("header.site-header")).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Primary navigation" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Try this page again" })).toBeVisible();
  await expect(page.getByText("route-boundary-test-sentinel")).toHaveCount(0);

  await page.getByRole("button", { name: "Try this page again" }).click();
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
});

test("header navigation leaves a failed route and renders the selected surface", async ({
  page,
}) => {
  await page.addInitScript(() => {
    let shouldFail = true;
    window.__PLE_ROUTE_FAILURE_TEST__ = (): boolean => {
      const requested = shouldFail;
      shouldFail = false;
      return requested;
    };
  });
  await page.goto("/");

  await expect(page.getByRole("button", { name: "Try this page again" })).toBeVisible();
  await page.getByRole("link", { name: "Library" }).click();
  await expect(page.locator("header.site-header")).toBeVisible();
  await expect(page.locator('[data-route-surface="library"]')).toBeVisible();
});

test("a student reaches, validates, submits, and advances through the generated reference response", async ({
  page,
}) => {
  const failedAssetRequests: string[] = [];
  page.on("response", (response) => {
    if (new URL(response.url()).pathname.startsWith("/api/assets/") && !response.ok()) {
      failedAssetRequests.push(`${response.status()} ${response.url()}`);
    }
  });

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();

  await page.getByRole("link", { name: "Open course" }).click();
  await expect(page.getByRole("heading", { name: "Assignments" })).toBeVisible();
  await page.getByRole("link", { name: "Review assignment" }).click();
  await expect(page.getByRole("heading", { name: "Peptide bond mastery" })).toBeVisible();
  await page.getByRole("button", { name: "Start or resume practice" }).click();

  await expect(
    page.getByRole("heading", { name: "Peptide bond resonance and planarity", exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Question", exact: true })).toBeVisible();
  await expect(page.getByText("Question content ready.")).toBeVisible();
  const images = page.locator("img.question-renderer__image");
  await expect(images).toHaveCount(2);
  await expect
    .poll(async () =>
      images.evaluateAll((nodes) =>
        nodes.every((image) => (image as HTMLImageElement).naturalWidth > 0),
      ),
    )
    .toBe(true);
  expect(failedAssetRequests).toEqual([]);
  const radios = page.getByRole("radio");
  await expect(radios).toHaveCount(3);
  await radios.nth(1).check();
  await expect(radios.nth(1)).toBeChecked();
  await expect(page.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );

  if (process.env["PLE_CAPTURE_VISUALS"] === "1") {
    fs.mkdirSync("generated/ui", { recursive: true });
    await page.screenshot({ path: "generated/ui/wp_c9_run_desktop.png", fullPage: true });
  }

  const selectedTarget = radios.nth(1).locator("xpath=ancestor::label");
  const box = await selectedTarget.boundingBox();
  expect(box?.height).toBeGreaterThanOrEqual(56);

  await page.getByRole("button", { name: "Submit answer" }).click();
  // Feedback focus timing is covered by the component acceptance fixture; the
  // integrated run flow verifies the panel mounts and remains actionable.
  await expect(page.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Continue" }).click();
  await expect(
    page.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Start another practice run" })).toBeVisible();
});

test("a student completes the primary platform-key course-to-answer path without a pointer", async ({
  page,
}) => {
  await page.goto("/");

  await tabTo(page, page.getByRole("link", { name: "Skip to learning content" }));
  await expect(page.getByRole("link", { name: "Skip to learning content" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();

  const openCourse = page.getByRole("link", { name: "Open course" });
  await tabTo(page, openCourse);
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();

  const reviewAssignment = page.getByRole("link", { name: "Review assignment" });
  await tabTo(page, reviewAssignment);
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();

  const start = page.getByRole("button", { name: "Start or resume practice" });
  await tabTo(page, start);
  await page.keyboard.press("Space");
  await expect(page.locator("#main-content")).toBeFocused();

  const radios = page.getByRole("radio");
  await tabTo(page, radios.first());
  await page.keyboard.press("Space");
  await expect(radios.first()).toBeChecked();
  await expect(page.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );

  const submit = page.getByRole("button", { name: "Submit answer" });
  await page.keyboard.press("Tab");
  await expect(submit).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(radios.first()).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(submit).toBeFocused();
  await page.keyboard.press("Space");

  const continueButton = page.getByRole("button", { name: "Continue" });
  await tabTo(page, continueButton);
  await page.keyboard.press("Space");
  await expect(
    page.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
  ).toBeVisible();

  const back = page.getByRole("button", { name: "Back to assignment" });
  await tabTo(page, back);
  await page.keyboard.press("Space");
  await expect(page.getByRole("heading", { name: "Peptide bond mastery" })).toBeVisible();
});

test("student question and feedback surfaces have no serious or critical axe violations", async ({
  page,
}) => {
  await page.goto("/");
  await navigateWithinSpa(page, `/runs/${IDS.run}`);
  await expect(page.locator('[data-route-surface="runAttempt"]')).toBeVisible();
  await expectNoBlockingAxeViolations(page);

  const radios = page.getByRole("radio");
  await radios.nth(1).check();
  await page.getByRole("button", { name: "Submit answer" }).click();
  await expect(page.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible();
  await expectNoBlockingAxeViolations(page);
});

test("session recovery stays editable, never writes the attempt to local storage, and clears on exit", async ({
  page,
}) => {
  await page.addInitScript(
    ({ key, buffer }) => {
      sessionStorage.setItem(key, buffer);
    },
    { key: SAVED_ATTEMPT_KEY, buffer: SAVED_ATTEMPT_BUFFER },
  );

  await page.goto("/");
  await navigateWithinSpa(page, `/runs/${IDS.run}`);
  const radios = page.getByRole("radio");
  await expect(radios.nth(1)).toBeChecked();
  const restoredStorage = await page.evaluate((key) => {
    return {
      localAttempt: localStorage.getItem(key),
      sessionAttempt: sessionStorage.getItem(key),
    };
  }, SAVED_ATTEMPT_KEY);
  expect(restoredStorage).toEqual({
    localAttempt: null,
    sessionAttempt: SAVED_ATTEMPT_BUFFER,
  });

  await radios.nth(2).check();
  await expect(radios.nth(2)).toBeChecked();
  await expect(radios.nth(1)).not.toBeChecked();
  await expect(page.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );
  const editedStorage = await page.evaluate((key) => {
    return {
      localAttempt: localStorage.getItem(key),
      sessionAttempt: sessionStorage.getItem(key),
    };
  }, SAVED_ATTEMPT_KEY);
  expect(editedStorage.localAttempt).toBeNull();
  expect(editedStorage.sessionAttempt).not.toBeNull();
  expect(editedStorage.sessionAttempt).not.toBe(SAVED_ATTEMPT_BUFFER);
  expect(editedStorage.sessionAttempt).toContain('"selected":["alpha-carbon"]');
  expect(editedStorage.sessionAttempt).toContain('"idempotencyKey":');

  await navigateWithinSpa(page, "/");
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
  const exitedStorage = await page.evaluate((key) => {
    return {
      localAttempt: localStorage.getItem(key),
      sessionAttempt: sessionStorage.getItem(key),
    };
  }, SAVED_ATTEMPT_KEY);
  expect(exitedStorage).toEqual({ localAttempt: null, sessionAttempt: null });
});

test("the reference response remains usable at the 320 CSS-pixel baseline", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await page.goto("/");
  await navigateWithinSpa(page, `/runs/${IDS.run}`);
  await expect(page.locator('[data-route-surface="runAttempt"]')).toBeVisible();

  const hasHorizontalOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
  );
  expect(hasHorizontalOverflow).toBe(false);
  await expect(page.getByRole("button", { name: "Submit answer" })).toBeVisible();
  if (process.env["PLE_CAPTURE_VISUALS"] === "1") {
    fs.mkdirSync("generated/ui", { recursive: true });
    await page.screenshot({ path: "generated/ui/wp_c9_run_320.png", fullPage: true });
  }
});
