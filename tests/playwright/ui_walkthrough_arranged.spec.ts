// ui_walkthrough_arranged.spec.ts - visible student confirmation of arranged course work.
//
// Selector contract:
// - src/app.tsx: Local development credential and Sign in locally controls.
// - src/pages/course_list_page.tsx: visible course card and Open course link.
// - src/pages/course_assignments_page.tsx: assignment card headings and Review assignment links.

import { expect, test } from "@playwright/test";

import { configuredUiWalkthroughInputs } from "../../playwright.config";
import { examContrastTitle, masteryRetryTitle } from "./simulator/assignment_titles";
import { tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";
import { studentCredentialFromValidatedFile } from "./ui_walkthrough_live_config";

test.describe.configure({ mode: "serial" });

test.skip(
  configuredUiWalkthroughInputs === undefined,
  "requires the explicit UI walkthrough live-stack invocation",
);

test("student visibly signs in and opens the arranged Mastery and Exam assignments", async ({
  page,
}) => {
  const inputs = configuredUiWalkthroughInputs;
  if (inputs === undefined)
    throw new Error("the declaration-time live walkthrough skip did not apply");
  if (inputs.examAssignmentId === undefined) {
    throw new Error("historical arranged journey requires its separately arranged Exam assignment");
  }

  await page.goto("/");
  const credential = studentCredentialFromValidatedFile(inputs.credentialFile);
  await page.getByLabel("Local development credential").fill(credential);
  await page.getByRole("button", { name: "Sign in locally" }).click();
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();

  const courseCard = page
    .locator(".course-card")
    .filter({ has: page.locator(`a[href="/courses/${inputs.courseId}"]`) });
  await courseCard.getByRole("link", { name: "Open course" }).click();
  await expect(page).toHaveURL(new RegExp(inputs.courseId, "u"));
  const masteryCard = page.locator(".course-card").filter({
    has: page.locator(
      `a[href="/courses/${inputs.courseId}/assignments/${inputs.masteryAssignmentId}"]`,
    ),
  });
  await tabToTargetThroughVisiblePagination(page, {
    target: masteryCard,
    keyboardTarget: masteryCard.getByRole("link", { name: "Review assignment", exact: true }),
    renderedItems: page.locator(".course-card"),
    firstAppendedControl: (index) =>
      page
        .locator(".course-card")
        .nth(index)
        .getByRole("link", { name: "Review assignment", exact: true }),
    itemName: "assignments",
  });
  await expect(
    masteryCard.getByRole("heading", { name: masteryRetryTitle(inputs.masteryProblemId) }),
  ).toBeVisible();
  await masteryCard.getByRole("link", { name: "Review assignment" }).click();
  await expect(page).toHaveURL(new RegExp(inputs.masteryAssignmentId, "u"));

  await page.goBack();
  const examCard = page.locator(".course-card").filter({
    has: page.locator(
      `a[href="/courses/${inputs.courseId}/assignments/${inputs.examAssignmentId}"]`,
    ),
  });
  await tabToTargetThroughVisiblePagination(page, {
    target: examCard,
    keyboardTarget: examCard.getByRole("link", { name: "Review assignment", exact: true }),
    renderedItems: page.locator(".course-card"),
    firstAppendedControl: (index) =>
      page
        .locator(".course-card")
        .nth(index)
        .getByRole("link", { name: "Review assignment", exact: true }),
    itemName: "assignments",
  });
  await expect(
    examCard.getByRole("heading", { name: examContrastTitle(inputs.masteryProblemId) }),
  ).toBeVisible();
  await examCard.getByRole("link", { name: "Review assignment" }).click();
  await expect(page).toHaveURL(new RegExp(inputs.examAssignmentId, "u"));
});
