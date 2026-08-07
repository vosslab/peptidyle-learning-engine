// frontend_contract.spec.ts - built-artifact proof for the WP-C9 reference slice.

import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";

const IDS = {
  course: "0198e000-0000-7000-8000-000000000014",
  assignment: "0198e000-0000-7000-8000-000000000006",
  run: "0198e000-0000-7000-8000-000000000023",
  problem: "0198e000-0000-7000-8000-000000000003",
  version: "0198e000-0000-7000-8000-000000000004",
  workspace: "0198e000-0000-7000-8000-000000000002",
} as const;

async function navigateWithinSpa(page: Page, pathname: string): Promise<void> {
  await page.evaluate((nextPath) => {
    history.pushState({}, "", nextPath);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, pathname);
}

test("all eleven product routes resolve inside the persistent shell", async ({ page }) => {
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
  ];

  await page.goto("/");
  await expect(page.getByRole("link", { name: "Peptidyle home" })).toBeVisible();

  for (const route of routes) {
    await navigateWithinSpa(page, route.path);
    await expect(page.locator(`[data-route-surface="${route.surface}"]`)).toBeVisible();
    await expect(page.locator("header.site-header")).toBeVisible();
  }
});

test("a student reaches, validates, and submits the reference response without an API server", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();

  await page.getByRole("link", { name: "Open course" }).click();
  await expect(page.getByRole("heading", { name: "Assignments" })).toBeVisible();
  await page.getByRole("link", { name: "Review assignment" }).click();
  await expect(page.getByRole("heading", { name: "Peptide bond mastery" })).toBeVisible();
  await page.getByRole("button", { name: "Start or resume practice" }).click();

  await expect(
    page.getByRole("heading", { name: "Peptide bond resonance and planarity" }),
  ).toBeVisible();
  const radios = page.getByRole("radio");
  await expect(radios).toHaveCount(3);
  await radios.first().focus();
  await page.keyboard.press("2");
  await expect(radios.nth(1)).toBeChecked();
  await expect(radios.nth(1)).toBeFocused();
  await expect(page.getByRole("status")).toContainText("ready to submit");

  if (process.env["PLE_CAPTURE_VISUALS"] === "1") {
    fs.mkdirSync("generated/ui", { recursive: true });
    await page.screenshot({ path: "generated/ui/wp_c9_run_desktop.png", fullPage: true });
  }

  const selectedTarget = radios.nth(1).locator("xpath=ancestor::label");
  const box = await selectedTarget.boundingBox();
  expect(box?.height).toBeGreaterThanOrEqual(56);

  await page.keyboard.press("Enter");
  await expect(page.getByRole("status")).toContainText("Answer submitted");
  await expect(page.locator(".response-widget")).toHaveAttribute("data-phase", "submitted");
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
