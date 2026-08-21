// Built assignment-access journey. Selector contract: AssignmentAccessPage and ModifierDialog use
// native labelled controls and dialogs; PolicyPreview exposes the server-derived safe projection.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

const COURSE = "C-1";
const ASSIGNMENT = "A-1";
const accessPath = `/instructor/courses/${COURSE}/assignments/${ASSIGNMENT}/access`;

async function installInstructorMock(page: Page, conflictOnce = false): Promise<void> {
  await page.addInitScript((conflict) => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", { value: true });
    Object.defineProperty(window, "__PLE_MOCK_TEACHING_OPERATIONS_INSTRUCTOR__", { value: true });
    Object.defineProperty(window, "__PLE_MOCK_TEACHING_MODIFIER_CONFLICT_ONCE__", {
      value: conflict,
    });
  }, conflictOnce);
}

function accessDialog(page: Page, name: string): ReturnType<Page["getByRole"]> {
  return page.getByRole("dialog", { name });
}

async function revealProvenance(preview: ReturnType<Page["getByRole"]>): Promise<void> {
  const details = preview.locator("details");
  if ((await details.getAttribute("open")) === null) {
    await preview.getByText("Field provenance").click();
  }
}

async function acceptNamedRemoval(
  page: Page,
  dialog: ReturnType<typeof accessDialog>,
): Promise<void> {
  page.once("dialog", (confirmation): void => {
    void confirmation.accept();
  });
  await dialog.getByRole("button", { name: "Remove named modifier" }).click();
  await expect(
    page.getByText("Modifier saved. The assignment revision was updated."),
  ).toBeVisible();
}

test("assignment access: instructor resolves safe previews and persists then removes M2 through M4", async ({
  page,
}) => {
  await installInstructorMock(page);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto(accessPath);

  await expect(page.getByRole("heading", { name: "Access and modifiers" })).toBeVisible();
  const preview = page.getByRole("region", { name: "Preview a learner" });
  await preview.getByRole("combobox").selectOption("M-1");
  await expect(page.getByText("Course time zone:")).toContainText("America/Chicago");
  await expect(page.getByText("2026-08-24T10:00:00.000", { exact: true })).toBeVisible();
  await revealProvenance(preview);
  await expect(
    preview.getByRole("listitem").filter({ hasText: "Course policy" }).first(),
  ).toBeVisible();

  await preview.getByRole("combobox").selectOption("M-2");
  await expect(preview.getByText("This learner is not entitled to this assignment.")).toBeVisible();
  await preview.getByRole("combobox").selectOption("M-1");
  await expect(page.getByText("Course time zone:")).toContainText("America/Chicago");

  const m2Button = page.getByRole("button", { name: "Add or change group schedule offsets" });
  await m2Button.click();
  const m2 = accessDialog(page, "Group schedule offset");
  await expect(m2.getByLabel("Course group")).toContainText("Section A");
  await m2.getByLabel("Offset in seconds").fill("900");
  await m2.getByRole("button", { name: "Save modifier" }).click();
  await expect(
    page.getByText("Modifier saved. The assignment revision was updated."),
  ).toBeVisible();
  await revealProvenance(preview);
  await expect(
    preview.getByRole("listitem").filter({ hasText: "Section A" }).first(),
  ).toBeVisible();
  await m2Button.click();
  await acceptNamedRemoval(page, accessDialog(page, "Group schedule offset"));
  await revealProvenance(preview);
  await expect(
    preview.getByRole("listitem").filter({ hasText: "Course policy" }).first(),
  ).toBeVisible();

  await page.getByRole("button", { name: "Group accommodations" }).click();
  await page.getByRole("button", { name: "Add or change group accommodations" }).click();
  const m3 = accessDialog(page, "Group accommodation");
  await expect(m3.getByRole("radio", { name: "Extend only" })).toBeChecked();
  const closes = m3.locator("fieldset").filter({ hasText: "Closes" });
  await closes.getByRole("radio", { name: "Set" }).check();
  await closes.locator('input[type="datetime-local"]').fill("2026-08-30T14:15");
  await m3.getByRole("button", { name: "Save modifier" }).click();
  await expect(page.getByText("2026-08-30T14:15:00.000", { exact: true })).toBeVisible();
  await revealProvenance(preview);
  await expect(
    preview.getByRole("listitem").filter({ hasText: "Accessibility extensions" }).first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Add or change group accommodations" }).click();
  await acceptNamedRemoval(page, accessDialog(page, "Group accommodation"));
  await expect(page.getByText("2026-08-24T11:00:00.000", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Individual exceptions" }).click();
  await page.getByRole("button", { name: "Add or change individual exceptions" }).click();
  const m4 = accessDialog(page, "Individual exception");
  await m4.getByRole("radio", { name: "Override" }).check();
  const individualCloses = m4.locator("fieldset").filter({ hasText: "Closes" });
  await individualCloses.getByRole("radio", { name: "Set" }).check();
  await individualCloses.locator('input[type="datetime-local"]').fill("2026-08-31T17:45");
  await m4.getByRole("button", { name: "Save modifier" }).click();
  await expect(page.getByText("2026-08-31T17:45:00.000", { exact: true })).toBeVisible();
  await revealProvenance(preview);
  await expect(
    preview.getByRole("listitem").filter({ hasText: "Individual exception" }).first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Add or change individual exceptions" }).click();
  await acceptNamedRemoval(page, accessDialog(page, "Individual exception"));

  const visibleText = await page.locator("main").innerText();
  expect(visibleText).not.toMatch(/\b(?:M|G|U|CI)-\d+\b/u);
  expect(visibleText).not.toMatch(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f-]{15,}/iu);
  expect(visibleText).not.toContain("@");

  await page.getByRole("button", { name: "Group schedule offsets" }).click();
  await m2Button.focus();
  await m2Button.click();
  await page.keyboard.press("Escape");
  await expect(accessDialog(page, "Group schedule offset")).toHaveCount(0);
  await expect(m2Button).toBeFocused();

  const axe = await new AxeBuilder({ page }).include("main").analyze();
  expect(
    axe.violations.filter((item) => item.impact === "critical" || item.impact === "serious"),
  ).toEqual([]);
});

test("assignment access: a one-time stale revision reload retains the open local-time draft", async ({
  page,
}) => {
  await installInstructorMock(page, true);
  await page.goto(accessPath);
  await expect(page.getByRole("heading", { name: "Access and modifiers" })).toBeVisible();
  await page
    .getByRole("region", { name: "Preview a learner" })
    .getByRole("combobox")
    .selectOption("M-1");
  await page.getByRole("button", { name: "Group accommodations" }).click();
  await page.getByRole("button", { name: "Add or change group accommodations" }).click();
  const dialog = accessDialog(page, "Group accommodation");
  const closes = dialog.locator("fieldset").filter({ hasText: "Closes" });
  await closes.getByRole("radio", { name: "Set" }).check();
  const localInput = closes.locator('input[type="datetime-local"]');
  await localInput.fill("2026-08-30T14:15");
  await dialog.getByRole("button", { name: "Save modifier" }).click();
  await expect(
    dialog.getByRole("button", { name: "Reload latest assignment revision" }),
  ).toBeVisible();
  await expect(dialog).toBeVisible();
  await expect(localInput).toHaveValue("2026-08-30T14:15");
  await dialog.getByRole("button", { name: "Reload latest assignment revision" }).click();
  await expect(
    page.getByText("Latest assignment revision loaded. Your modifier draft is unchanged."),
  ).toBeVisible();
  await expect(dialog).toBeVisible();
  await expect(localInput).toHaveValue("2026-08-30T14:15");
  await dialog.getByRole("button", { name: "Save modifier" }).click();
  await expect(page.getByText("2026-08-30T14:15:00.000", { exact: true })).toBeVisible();
});
