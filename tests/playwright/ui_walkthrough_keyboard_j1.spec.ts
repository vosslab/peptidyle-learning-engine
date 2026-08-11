// ui_walkthrough_keyboard_j1.spec.ts - J1 through the rendered platform keyboard path.

import { expect, test } from "@playwright/test";

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

function appendPassedJourneyState(elapsedMs: number): void {
  const inputs = configuredUiWalkthroughInputs;
  if (inputs === undefined)
    throw new Error("the declaration-time live walkthrough skip did not apply");
  appendStudentRepeatState(
    inputs.journeyStateFile,
    passedStudentRepeatFragment("J1", inputs.courseId, inputs.masteryAssignmentId, elapsedMs),
  );
}

test("J1 student reaches visible retry controls for the instructor-created Mastery assignment", async ({
  page,
}) => {
  const inputs = configuredUiWalkthroughInputs;
  if (inputs === undefined)
    throw new Error("the declaration-time live walkthrough skip did not apply");
  const startedAt = performance.now();

  await page.goto("/");
  const credential = studentCredentialFromValidatedFile(inputs.credentialFile);
  const credentialInput = page.getByLabel("Local development credential");
  await tabTo(page, credentialInput);
  await expect(credentialInput).toBeFocused();
  await credentialInput.fill(credential);
  const signIn = page.getByRole("button", { name: "Sign in locally" });
  await tabTo(page, signIn);
  await expect(signIn).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
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

  const start = page.getByRole("button", { name: "Start or resume practice" });
  await tabTo(page, start);
  await expect(start).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible();

  const radios = page.locator('input[type="radio"]:visible');
  await expect(radios).toHaveCount(2);
  await expect(radios.nth(0)).not.toBeChecked();
  await expect(radios.nth(1)).not.toBeChecked();
  const response = radios.nth(0);
  await tabTo(page, response);
  await expect(response).toBeFocused();
  await page.keyboard.press("Space");
  await expect(response).toBeChecked();
  const submit = page.getByRole("button", { name: "Submit answer" });
  await expect(submit).toBeEnabled();
  await page.keyboard.press("Tab");
  await expect(submit).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(response).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(submit).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible();

  const continueButton = page.getByRole("button", { name: "Continue" });
  await tabTo(page, continueButton);
  await expect(continueButton).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible({ timeout: 30_000 });
  await expect(page.getByRole("button", { name: "Start another practice run" })).toHaveCount(0);
  await expect(radios).toHaveCount(2);
  await expect(radios.nth(0)).not.toBeChecked();
  await expect(radios.nth(1)).not.toBeChecked();
  appendPassedJourneyState(Math.round(performance.now() - startedAt));
});
