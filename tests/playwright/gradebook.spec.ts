// MOD-UI-GRADEBOOK browser proof for accessible compact and expanded states.

import { expect, test } from "@playwright/test";

const courseId = "0198e000-0000-7000-8000-000000000014";
const gradebookPath = `/instructor/courses/${courseId}/gradebook`;

async function openGradebook(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/");
  await page.evaluate((nextPath) => {
    history.pushState({}, "", nextPath);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, gradebookPath);
}

test("gradebook presents compact progress and loads history only when requested", async ({
  page,
}) => {
  await openGradebook(page);

  await expect(page.locator('[data-route-surface="gradebook"]')).toBeVisible();
  await expect(page.getByRole("heading", { name: "Gradebook" })).toBeVisible();
  await expect(page.getByRole("table")).toBeVisible();
  await expect(page.getByRole("cell", { name: "100%" }).first()).toBeVisible();
  await expect(page.getByText("Peptide bond mastery", { exact: true })).toBeVisible();
  await expect(page.getByRole("region", { name: /run history/i })).toHaveCount(0);

  const historyButton = page.getByRole("button", { name: "View run history" });
  await historyButton.focus();
  await page.keyboard.press("Enter");
  await expect(historyButton).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByRole("region", { name: /run history for learner/i })).toBeVisible();
  await expect(page.getByText(/Run 1:/)).toBeVisible();
  await expect(page.locator("[aria-live='polite']")).toContainText(
    /Run history updated|Loading run history/,
  );
});

test("gradebook reflows into labeled records on a narrow viewport", async ({ page }) => {
  await page.setViewportSize({ width: 420, height: 900 });
  await openGradebook(page);

  await expect(page.locator(".gradebook-row")).toBeVisible();
  await expect(page.locator(".gradebook-table th[data-label='Assignment']")).toContainText(
    "Peptide bond mastery",
  );
  await expect(page.getByRole("button", { name: "View run history" })).toBeVisible();
});
