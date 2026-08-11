// ui_walkthrough_keyboard_j4.spec.ts - J4 completes the second visible Mastery run.

import { expect, test, type Page } from "@playwright/test";

import { configuredUiWalkthroughInputs } from "../../playwright.config";

import { tabTo, tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";
import {
  appendStudentRepeatState,
  passedStudentRepeatFragment,
} from "./simulator/student_repeat_state";
import { studentCredentialFromValidatedFile } from "./ui_walkthrough_live_config";
import { captureDocumentationScreenshot } from "./docs_screenshot_capture";

test.describe.configure({ mode: "serial" });

test.skip(
  configuredUiWalkthroughInputs === undefined,
  "requires the explicit UI walkthrough live-stack invocation",
);
test.setTimeout(90_000);

async function signInAndResumeSecondRun(page: Page): Promise<void> {
  const inputs = configuredUiWalkthroughInputs;
  if (inputs === undefined)
    throw new Error("the declaration-time live walkthrough skip did not apply");
  await page.goto("/");
  const credential = page.getByLabel("Local development credential");
  await tabTo(page, credential);
  await expect(credential).toBeFocused();
  await credential.fill(studentCredentialFromValidatedFile(inputs.credentialFile));
  const signIn = page.getByRole("button", { name: "Sign in locally" });
  await tabTo(page, signIn);
  await expect(signIn).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=courses]")).toBeVisible();
  const course = page.locator(`a[href="/courses/${inputs.courseId}"]`);
  await tabToTargetThroughVisiblePagination(page, {
    target: course,
    renderedItems: page.locator(".course-card"),
    firstAppendedControl: (index) =>
      page
        .locator(".course-card")
        .nth(index)
        .getByRole("link", { name: "Open course", exact: true }),
    itemName: "courses",
  });
  await expect(course).toHaveCount(1);
  await expect(course).toBeVisible();
  await expect(course).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
  const assignment = page.locator(
    `a[href="/courses/${inputs.courseId}/assignments/${inputs.masteryAssignmentId}"]`,
  );
  await tabToTargetThroughVisiblePagination(page, {
    target: assignment,
    renderedItems: page.locator(".course-card"),
    firstAppendedControl: (index) =>
      page
        .locator(".course-card")
        .nth(index)
        .getByRole("link", { name: "Review assignment", exact: true }),
    itemName: "assignments",
  });
  await expect(assignment).toHaveCount(1);
  await expect(assignment).toBeVisible();
  await tabTo(page, assignment, "backward");
  await expect(assignment).toBeFocused();
  await page.keyboard.press("Enter");
  const resume = page.getByRole("button", { name: "Start or resume practice" });
  await tabTo(page, resume);
  await expect(resume).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible({ timeout: 15_000 });
}

test("J4 completes the resumed second Mastery run through visible keyboard controls", async ({
  page,
}) => {
  const inputs = configuredUiWalkthroughInputs;
  if (inputs === undefined)
    throw new Error("the declaration-time live walkthrough skip did not apply");
  const startedAt = performance.now();
  await signInAndResumeSecondRun(page);
  const radios = page.locator('input[type="radio"]:visible');
  await expect(radios).toHaveCount(2);
  await expect(radios.nth(0)).not.toBeChecked();
  await expect(radios.nth(1)).not.toBeChecked();
  const response = radios.nth(1);
  await tabTo(page, response, "backward");
  await expect(response).toBeFocused();
  await page.keyboard.press("Space");
  await expect(response).toBeChecked();
  const submit = page.getByRole("button", { name: "Submit answer" });
  await tabTo(page, submit);
  await expect(submit).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible();
  const continueButton = page.getByRole("button", { name: "Continue" });
  await tabTo(page, continueButton);
  await expect(continueButton).toBeFocused();
  await page.keyboard.press("Space");
  await expect(
    page.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
  ).toBeVisible({ timeout: 30_000 });
  await expect(page.getByRole("button", { name: "Start another practice run" })).toBeVisible();
  const back = page.getByRole("button", { name: "Back to assignment" });
  await tabTo(page, back);
  await expect(back).toBeFocused();
  await captureDocumentationScreenshot(
    page,
    "student_fresh_practice.png",
    page.locator(".attempt-summary"),
    72,
  );
  appendStudentRepeatState(
    inputs.journeyStateFile,
    passedStudentRepeatFragment(
      "J4",
      inputs.courseId,
      inputs.masteryAssignmentId,
      Math.round(performance.now() - startedAt),
    ),
  );
});
