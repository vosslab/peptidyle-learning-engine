// ui_walkthrough_keyboard_j1.spec.ts - J1 through the rendered platform keyboard path.

import { expect, test } from "./ui_walkthrough_fixture";

import { tabTo, tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";
import {
  expectVisibleResponseControlsCleared,
  submitVisibleResponseCandidate,
} from "./simulator/chapter_question_responses";
import {
  appendStudentRepeatState,
  passedStudentRepeatFragment,
} from "./simulator/student_repeat_state";
import { writeJ1Checkpoint, type J1Checkpoint } from "./simulator/j1_checkpoint";
import {
  credentialFromValidatedFile,
  type UiWalkthroughInputs,
} from "./ui_walkthrough_config_factory";
import {
  captureDocumentationScreenshot,
  documentationScreenshotsEnabled,
} from "./docs_screenshot_capture";

test.describe.configure({ mode: "serial" });

test.setTimeout(90_000);

function appendPassedJourneyState(inputs: UiWalkthroughInputs, elapsedMs: number): void {
  appendStudentRepeatState(
    inputs.journeyStateFile,
    passedStudentRepeatFragment("J1", inputs.courseId, inputs.masteryAssignmentId, elapsedMs),
  );
}

function checkpoint(inputs: UiWalkthroughInputs, stage: J1Checkpoint): void {
  writeJ1Checkpoint(inputs.j1CheckpointFile, stage);
}

test("J1 student reaches visible feedback and the next question in the instructor-created Mastery assignment", async ({
  page,
  uiWalkthroughInputs,
}) => {
  test.skip(uiWalkthroughInputs === undefined, "requires the explicit UI walkthrough config");
  if (uiWalkthroughInputs === undefined) return;
  const inputs = uiWalkthroughInputs;
  const startedAt = performance.now();

  await page.goto("/");
  const credential = credentialFromValidatedFile(inputs.credentialFile, "student");
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
  checkpoint(inputs, "signed_in");

  const courseLink = page.locator(`a[href="/courses/${inputs.courseId}"]`);
  await tabToTargetThroughVisiblePagination(page, {
    target: courseLink,
    renderedItems: page.locator(".course-card"),
    firstAppendedControl: (index) =>
      page
        .locator(".course-card")
        .nth(index)
        .getByRole("link", { name: "Open course", exact: true }),
    itemName: "courses",
  });
  await expect(courseLink).toHaveCount(1);
  await expect(courseLink).toBeVisible();
  checkpoint(inputs, "course_visible");
  await expect(courseLink).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=courseAssignments]")).toBeVisible({
    timeout: 30_000,
  });
  await expect(page.locator("#main-content")).toBeFocused({ timeout: 30_000 });
  checkpoint(inputs, "course_opened");

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
  checkpoint(inputs, "assignment_visible");
  await captureDocumentationScreenshot(
    page,
    "student_assignment_list.png",
    assignmentLink.locator("xpath=ancestor::article[contains(@class, 'course-card')]"),
    72,
    inputs.screenshotDirectory,
  );
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
  const questionsPerRun = page.locator(".assignment-facts > div", {
    has: page.locator("dt", { hasText: "Questions per run" }),
  });
  await expect(questionsPerRun.locator("dd")).toHaveText("4");

  const start = page.getByRole("button", { name: "Start or resume practice" });
  await tabTo(page, start);
  await expect(start).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible({ timeout: 30_000 });

  await expectVisibleResponseControlsCleared(page);
  checkpoint(inputs, "run_controls_visible");
  if (documentationScreenshotsEnabled(inputs.screenshotDirectory)) {
    await expect(page.getByRole("timer")).not.toHaveText("Untimed");
  }
  const firstResponse = page.locator('input[type="radio"]:visible').first();
  await tabTo(page, firstResponse);
  await expect(firstResponse).toBeFocused();
  await captureDocumentationScreenshot(
    page,
    "student_timed_problem.png",
    undefined,
    undefined,
    inputs.screenshotDirectory,
  );
  const submitted = await submitVisibleResponseCandidate(page);
  expect(submitted.outcome).toBe("not-quite");
  checkpoint(inputs, "feedback_visible");

  const continueButton = page.getByRole("button", { name: "Continue" });
  await tabTo(page, continueButton);
  await expect(continueButton).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible({ timeout: 30_000 });
  await expect(page.getByRole("button", { name: "Start another practice run" })).toHaveCount(0);
  await expectVisibleResponseControlsCleared(page);
  checkpoint(inputs, "next_question_visible");
  appendPassedJourneyState(inputs, Math.round(performance.now() - startedAt));
});
