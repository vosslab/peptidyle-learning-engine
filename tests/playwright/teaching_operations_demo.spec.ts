// Built mock journey for WP-PROF-T2. Selector ownership: teaching_operations_page.tsx,
// teaching_team_panel.tsx, retention_panel.tsx, and assignment_access_page.tsx.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

const COURSE = "C-1";
const ASSIGNMENT = "A-1";

async function installInstructorFixture(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", { value: true });
    Object.defineProperty(window, "__PLE_MOCK_TEACHING_OPERATIONS_INSTRUCTOR__", { value: true });
  });
}

async function installRetentionConflictFixture(page: Page): Promise<void> {
  await installInstructorFixture(page);
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_MOCK_TEACHING_RETENTION_CONFLICT_ONCE__", {
      value: true,
    });
  });
}

async function installModalFailureFixture(page: Page): Promise<void> {
  await installInstructorFixture(page);
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_MOCK_TEACHING_GROUP_DELETE_CONFLICT_ONCE__", {
      value: true,
    });
    Object.defineProperty(window, "__PLE_MOCK_TEACHING_RETENTION_ARCHIVE_FORBIDDEN_ONCE__", {
      value: true,
    });
    Object.defineProperty(window, "__PLE_MOCK_TEACHING_RETENTION_DELETE_UNAVAILABLE_ONCE__", {
      value: true,
    });
  });
}

async function installPendingAccountFixture(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", { value: true });
    Object.defineProperty(window, "__PLE_MOCK_ACCOUNT_PENDING_INVITATION__", { value: true });
  });
}

async function expectNoBlockingAxeViolations(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).include("main").analyze();
  expect(
    results.violations.filter(
      (violation) => violation.impact === "critical" || violation.impact === "serious",
    ),
  ).toEqual([]);
}

test("instructor teaching operations manages safe groups, teaching team, and retention", async ({
  page,
}) => {
  await installInstructorFixture(page);
  await page.goto(`/instructor/courses/${COURSE}/teaching-operations`);
  await expect(page.getByRole("heading", { name: "Teaching operations" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Groups and sections" })).toBeVisible();
  await expect(page.getByText(/M-1|U-1/u)).toHaveCount(0);

  const groups = page.getByRole("region", { name: "Groups and sections" });
  await expect(
    groups.getByText(
      "Course-group membership check: allowed. No overlapping memberships need attention.",
    ),
  ).toBeVisible();
  const editor = groups.locator("form");
  await editor.getByLabel("Group name").fill("Section B");
  await editor.getByLabel("Members").selectOption("M-1");
  await editor.getByRole("button", { name: "Create group" }).click();
  await expect(page.getByRole("button", { name: "Section B", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Section B", exact: true }).click();
  await editor.getByLabel("Group name").fill("Section B revised");
  await editor.getByRole("button", { name: "Save group" }).click();
  await expect(page.getByRole("button", { name: "Section B revised", exact: true })).toBeVisible();
  const deleteTrigger = page.getByRole("button", { name: "Delete Section B revised" });
  await deleteTrigger.click();
  const deleteGroup = page.getByRole("dialog", { name: "Delete Section B revised?" });
  await expect(deleteGroup.getByRole("button", { name: "Cancel" })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(deleteGroup).toHaveCount(0);
  await expect(deleteTrigger).toBeFocused();
  await deleteTrigger.click();
  await deleteGroup.getByLabel("Group name").fill("Section B revised");
  await deleteGroup.getByRole("button", { name: "Confirm delete" }).click();
  await expect(page.getByRole("button", { name: "Section B revised" })).toHaveCount(0);

  const policy = page.getByRole("group", { name: "Multiple membership policy" });
  await policy.getByLabel("Policy").selectOption("allow");
  await policy.getByRole("button", { name: "Save policy" }).click();
  await expect(policy.getByText(/A warning never blocks a valid write/u)).toBeVisible();

  const team = page.getByRole("heading", { name: "Teaching team" }).locator("..");
  await team.getByLabel("Find an approved colleague").fill("Taylor");
  await team.getByRole("button", { name: "Search eligible people" }).click();
  await expect(team.getByText("Taylor Mentor", { exact: true })).toBeVisible();
  await team
    .getByRole("listitem")
    .filter({ hasText: "Taylor Mentor" })
    .getByRole("button", { name: "Select" })
    .click();
  await team.getByRole("button", { name: "Invite selected colleague" }).click();
  const pendingInvitations = team.getByRole("region", { name: "Pending invitations" });
  await expect(pendingInvitations.getByText("Taylor Mentor", { exact: true })).toBeVisible();
  await pendingInvitations.getByRole("button", { name: "Cancel invitation" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Cancel invitation" }).click();
  await expect(pendingInvitations.getByText(/^Canceled\./u)).toBeVisible();
  await team.getByRole("button", { name: "Remove" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Remove instructor" }).click();
  await expect(team.getByRole("alert")).toContainText("must keep one active instructor");

  const retention = page.getByRole("heading", { name: "Record retention" }).locator("..");
  await expect(
    retention.getByText("Server-owned retention actions protect learner records."),
  ).toBeVisible();
  const archiveTrigger = retention.getByRole("button", {
    name: "Archive student records",
    exact: true,
  });
  await archiveTrigger.click();
  const archive = page.getByRole("dialog", { name: "Archive student records?" });
  await expect(archive.getByRole("button", { name: "Cancel" })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(archive).toHaveCount(0);
  await expect(archiveTrigger).toBeFocused();
  await archiveTrigger.click();
  await archive.getByLabel("Confirmation").fill("ARCHIVE");
  await archive.getByRole("button", { name: "Confirm archive student records" }).click();
  await expect(retention.getByRole("status")).toContainText("The retention action is complete.");
  await expect(retention.getByRole("status")).toBeFocused();
  await expectNoBlockingAxeViolations(page);
});

test("teaching-operation modal errors retain modal focus and drafts", async ({ page }) => {
  await installModalFailureFixture(page);
  await page.goto(`/instructor/courses/${COURSE}/teaching-operations`);
  const groups = page.getByRole("region", { name: "Groups and sections" });
  const groupDelete = groups.getByRole("button", { name: "Delete Section A" });
  await groupDelete.click();
  const deleteDialog = page.getByRole("dialog", { name: "Delete Section A?" });
  await deleteDialog.getByLabel("Group name").fill("Section A");
  await deleteDialog.getByRole("button", { name: "Confirm delete" }).click();
  const groupError = deleteDialog.getByRole("alert");
  await expect(groupError).toContainText("still referenced");
  await expect(groupError).toBeFocused();
  await expect(deleteDialog.getByLabel("Group name")).toHaveValue("Section A");
  await page.keyboard.press("Escape");
  await expect(groupDelete).toBeFocused();

  const retention = page.getByRole("heading", { name: "Record retention" }).locator("..");
  const archiveTrigger = retention.getByRole("button", {
    name: "Archive student records",
    exact: true,
  });
  await archiveTrigger.click();
  const archive = page.getByRole("dialog", { name: "Archive student records?" });
  await archive.getByLabel("Confirmation").fill("ARCHIVE");
  await archive.getByRole("button", { name: "Confirm archive student records" }).click();
  const archiveError = archive.getByRole("alert");
  await expect(archiveError).toContainText("permission");
  await expect(archiveError).toBeFocused();
  await expect(archive.getByLabel("Confirmation")).toHaveValue("ARCHIVE");
  await page.keyboard.press("Escape");
  await expect(archiveTrigger).toBeFocused();

  const deleteTrigger = retention.getByRole("button", {
    name: "Delete student records",
    exact: true,
  });
  await deleteTrigger.click();
  const deleteRetention = page.getByRole("dialog", { name: "Delete student records?" });
  await deleteRetention.getByLabel("Confirmation").fill("DELETE");
  await deleteRetention.getByRole("button", { name: "Confirm delete student records" }).click();
  const deleteError = deleteRetention.getByRole("alert");
  await expect(deleteError).toContainText("offline");
  await expect(deleteError).toBeFocused();
  await expect(deleteRetention.getByLabel("Confirmation")).toHaveValue("DELETE");
});

test("instructor assignment access preserves exact course-local preview values", async ({
  page,
}) => {
  await installInstructorFixture(page);
  await page.goto(`/instructor/courses/${COURSE}/assignments/${ASSIGNMENT}/access`);
  await expect(page.getByRole("heading", { name: "Access and modifiers" })).toBeVisible();
  await page
    .getByRole("region", { name: "Preview a learner" })
    .getByRole("combobox")
    .selectOption("M-1");
  await expect(page.getByText("Course time zone:")).toContainText("America/Chicago");
  await expect(page.getByText("2026-08-24T10:00:00.000", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Add or change group schedule offsets" }).click();
  const dialog = page.getByRole("dialog", { name: "Group schedule offset" });
  await dialog.getByLabel("Offset in seconds").fill("900");
  await dialog.getByRole("button", { name: "Save modifier" }).click();
  await expect(
    page.getByText(/Modifier saved\. The assignment revision was updated\./u),
  ).toBeVisible();
  await page.getByRole("button", { name: "Group accommodations" }).click();
  await page.getByRole("button", { name: "Add or change group accommodations" }).click();
  await expect(page.getByRole("dialog", { name: "Group accommodation" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "Group accommodation" })).toHaveCount(0);
});

test("retention conflict reloads the current server state by keyboard", async ({ page }) => {
  await installRetentionConflictFixture(page);
  await page.goto(`/instructor/courses/${COURSE}/teaching-operations`);
  const retention = page.getByRole("heading", { name: "Record retention" }).locator("..");
  await retention.getByRole("button", { name: "Archive student records" }).click();
  const archive = page.getByRole("dialog", { name: "Archive student records?" });
  await archive.getByLabel("Confirmation").fill("ARCHIVE");
  await archive.getByRole("button", { name: "Confirm archive student records" }).click();
  await expect(archive).toHaveCount(0);
  await expect(retention.getByRole("status")).toContainText("changed elsewhere");
  await expect(retention.getByRole("status")).toBeFocused();
  await page.keyboard.press("Tab");
  const reload = retention.getByRole("button", { name: "Reload latest retention state" });
  await expect(reload).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(retention.getByRole("status")).toHaveText("Retention state loaded.");
  await expect(reload).toHaveCount(0);
});

test("authenticated account accepts its own pending invitation", async ({ page }) => {
  await installPendingAccountFixture(page);
  await page.goto("/account/co-instructor-invitations");
  await expect(page.getByRole("heading", { name: "Pending teaching invitations" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Demo course" })).toBeVisible();
  await expect(page.getByText(/U-5|email|uuid/u)).toHaveCount(0);
  await page.getByRole("button", { name: "Accept" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Accept invitation" }).click();
  await expect(page.getByText("Invitation accepted.", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "No invitations waiting" })).toBeVisible();
  await expectNoBlockingAxeViolations(page);
});

test("authenticated account declines its own pending invitation", async ({ page }) => {
  await installPendingAccountFixture(page);
  await page.goto("/account/co-instructor-invitations");
  await expect(page.getByRole("heading", { name: "Pending teaching invitations" })).toBeVisible();
  await page.getByRole("button", { name: "Decline" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Decline invitation" }).click();
  await expect(page.getByText("Invitation declined.", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "No invitations waiting" })).toBeVisible();
  await expectNoBlockingAxeViolations(page);
});
