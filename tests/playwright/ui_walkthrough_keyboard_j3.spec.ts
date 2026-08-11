// ui_walkthrough_keyboard_j3.spec.ts - J3 visible recovery through the platform keyboard path.
// Selector contract: src/pages/assignment_overview_page.tsx and src/pages/run_page.tsx.

import { expect, test, type Page } from "@playwright/test";

import { configuredUiWalkthroughInputs } from "../../playwright.config";

import { tabTo, tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";
import {
  appendStudentRepeatState,
  passedStudentRepeatFragment,
} from "./simulator/student_repeat_state";
import { studentCredentialFromValidatedFile } from "./ui_walkthrough_live_config";

test.describe.configure({ mode: "serial" });

test.skip(
  configuredUiWalkthroughInputs === undefined,
  "requires the explicit UI walkthrough live-stack invocation",
);
test.setTimeout(90_000);

async function signInAndOpenMastery(page: Page): Promise<void> {
  const inputs = configuredUiWalkthroughInputs;
  if (inputs === undefined)
    throw new Error("the declaration-time live walkthrough skip did not apply");

  await page.goto("/");
  const credentialInput = page.getByLabel("Local development credential");
  await tabTo(page, credentialInput);
  await expect(credentialInput).toBeFocused();
  await credentialInput.fill(studentCredentialFromValidatedFile(inputs.credentialFile));

  const signIn = page.getByRole("button", { name: "Sign in locally" });
  await tabTo(page, signIn);
  await expect(signIn).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=courses]")).toBeVisible();

  const courseLink = page.locator(`a[href="/courses/${inputs.courseId}"]`);
  await expect(courseLink).toHaveCount(1);
  await tabTo(page, courseLink);
  await expect(courseLink).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();

  const assignmentLink = page.locator(
    `a[href="/courses/${inputs.courseId}/assignments/${inputs.masteryAssignmentId}"]`,
  );
  await tabToTargetThroughVisiblePagination(page, {
    target: assignmentLink,
    renderedItems: page.locator(".course-card"),
    firstAppendedControl: (index) =>
      page
        .locator(".course-card")
        .nth(index)
        .getByRole("link", { name: "Review assignment", exact: true }),
    itemName: "assignments",
  });
  await expect(assignmentLink).toHaveCount(1);
  await expect(assignmentLink).toBeVisible();
  await tabTo(page, assignmentLink, "backward");
  await expect(assignmentLink).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
}

async function startOrResumeMastery(page: Page): Promise<void> {
  const start = page.getByRole("button", { name: "Start or resume practice" });
  await tabTo(page, start);
  await expect(start).toBeFocused();
  await page.keyboard.press("Space");
  const freshPractice = page.getByRole("button", { name: "Start another practice run" });
  await expect(freshPractice).toBeVisible({ timeout: 15_000 });
  await tabTo(page, freshPractice);
  await expect(freshPractice).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible({ timeout: 15_000 });
}

test("J3 starts the second Mastery run, proves cleared controls, and resumes it by keyboard", async ({
  page,
}) => {
  const inputs = configuredUiWalkthroughInputs;
  if (inputs === undefined)
    throw new Error("the declaration-time live walkthrough skip did not apply");
  const startedAt = performance.now();
  await signInAndOpenMastery(page);
  await startOrResumeMastery(page);
  const radios = page.locator('input[type="radio"]:visible');
  await expect(radios).toHaveCount(2);
  await expect(radios.nth(0)).not.toBeChecked();
  await expect(radios.nth(1)).not.toBeChecked();

  const returnToAssignment = page.getByRole("button", { name: "Return to assignment" });
  await tabTo(page, returnToAssignment);
  await expect(returnToAssignment).toBeFocused();
  await page.keyboard.press("Space");

  await expect(page.locator("[data-route-surface=assignmentOverview]")).toBeVisible({
    timeout: 15_000,
  });
  await expect(page.locator("#main-content")).toBeFocused({ timeout: 15_000 });

  const resume = page.getByRole("button", { name: "Start or resume practice" });
  await tabTo(page, resume);
  await expect(resume).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible({ timeout: 15_000 });
  await expect(radios).toHaveCount(2);
  await expect(radios.nth(0)).not.toBeChecked();
  await expect(radios.nth(1)).not.toBeChecked();
  appendStudentRepeatState(
    inputs.journeyStateFile,
    passedStudentRepeatFragment(
      "J3",
      inputs.courseId,
      inputs.masteryAssignmentId,
      Math.round(performance.now() - startedAt),
    ),
  );
});
