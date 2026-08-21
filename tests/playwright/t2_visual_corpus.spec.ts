// Manifest-owned desktop visual evidence for the WP-PROF-T2 teaching operations surfaces.
// Selector contract: teaching_operations_page.tsx, assignment_access_page.tsx, and
// pending_co_instructor_invitations_page.tsx provide the named headings and regions below.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Locator, type Page } from "@playwright/test";

import {
  captureDocumentationScreenshot,
  type DocumentationScreenshotName,
} from "./docs_screenshot_capture";

const outputDirectory = process.env["PLE_INSTRUCTOR_PAGE_VISUALS_DIR"];
const courseReference = "C-1";
const assignmentReference = "A-1";

async function installTeachingOperationsFixture(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", { value: true });
    Object.defineProperty(window, "__PLE_MOCK_TEACHING_OPERATIONS_INSTRUCTOR__", { value: true });
  });
}

async function capture(
  page: Page,
  name: DocumentationScreenshotName,
  anchor?: Locator,
): Promise<void> {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.evaluate(async () => {
    await document.fonts.ready;
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });
  await captureDocumentationScreenshot(page, name, anchor, undefined, outputDirectory);
}

test("captures the WP-PROF-T2 instructor teaching-operations visual corpus", async ({ page }) => {
  test.skip(outputDirectory === undefined, "requires the dedicated instructor visual launcher");
  if (outputDirectory === undefined) return;
  test.setTimeout(120_000);
  await page.setViewportSize({ width: 1_280, height: 800 });
  await installTeachingOperationsFixture(page);

  await page.goto(`/instructor/courses/${courseReference}/teaching-operations`);
  await expect(page.getByRole("heading", { name: "Teaching operations" })).toBeVisible();
  const groups = page.getByRole("region", { name: "Groups and sections" });
  const team = page.getByRole("heading", { name: "Teaching team" }).locator("..");
  const retention = page.getByRole("heading", { name: "Record retention" }).locator("..");
  await expect(groups).toBeVisible();
  await expect(team).toBeVisible();
  await expect(retention).toBeVisible();
  const axe = await new AxeBuilder({ page }).include("main").analyze();
  expect(
    axe.violations.filter(
      (violation) => violation.impact === "critical" || violation.impact === "serious",
    ),
  ).toEqual([]);
  await capture(page, "teaching_operations_groups.png", groups);
  await capture(page, "teaching_operations_team.png", team);
  await capture(page, "teaching_operations_retention.png", retention);

  await page.goto(
    `/instructor/courses/${courseReference}/assignments/${assignmentReference}/access`,
  );
  const preview = page.getByRole("region", { name: "Preview a learner" });
  await expect(preview).toBeVisible();
  await preview.getByRole("combobox").selectOption("M-1");
  await expect(preview.getByText("Course time zone:")).toContainText("America/Chicago");
  await capture(page, "assignment_access_allowed_preview.png", preview);
  await preview.getByRole("combobox").selectOption("M-2");
  await expect(preview.getByText("This learner is not entitled to this assignment.")).toBeVisible();
  await capture(page, "assignment_access_denied_preview.png", preview);
});

test("captures the target-bound account invitation surface", async ({ page }) => {
  test.skip(outputDirectory === undefined, "requires the dedicated instructor visual launcher");
  if (outputDirectory === undefined) return;
  await page.setViewportSize({ width: 1_280, height: 800 });
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", { value: true });
    Object.defineProperty(window, "__PLE_MOCK_ACCOUNT_PENDING_INVITATION__", { value: true });
  });
  await page.goto("/account/co-instructor-invitations");
  const invitations = page.locator('[data-route-surface="accountPendingInvitations"]');
  await expect(invitations.getByRole("heading", { name: "Demo course" })).toBeVisible();
  await expect(page.getByText(/U-5|email|uuid/u)).toHaveCount(0);
  const axe = await new AxeBuilder({ page }).include("main").analyze();
  expect(
    axe.violations.filter(
      (violation) => violation.impact === "critical" || violation.impact === "serious",
    ),
  ).toEqual([]);
  await capture(page, "account_pending_co_instructor_invitation.png", invitations);
});
