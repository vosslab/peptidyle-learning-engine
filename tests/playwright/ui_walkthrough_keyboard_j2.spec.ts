// ui_walkthrough_keyboard_j2.spec.ts - J2 retry path through rendered keyboard controls.

import { expect, test, type Page } from "@playwright/test";

import { configuredUiWalkthroughInputs } from "../../playwright.config";

import {
  classifyFinalSurface,
  classifyPostStartSurface,
  isFinalSurfaceTerminal,
  type FinalSurface,
  type PostStartSurface,
} from "./simulator/post_start_surface";
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

function appendPassedJourneyState(elapsedMs: number): void {
  const inputs = configuredUiWalkthroughInputs;
  if (inputs === undefined)
    throw new Error("the declaration-time live walkthrough skip did not apply");
  appendStudentRepeatState(
    inputs.journeyStateFile,
    passedStudentRepeatFragment("J2", inputs.courseId, inputs.masteryAssignmentId, elapsedMs),
  );
}

async function signInAndStartMastery(page: Page): Promise<void> {
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
  await captureDocumentationScreenshot(page, "peptide_bond_mastery_overview.png");
  await page.keyboard.press("Space");
  const freshPractice = page.getByRole("button", { name: "Start another practice run" });
  const surface = await waitForPostStartSurface(page, freshPractice);
  if (surface === "fresh-practice") {
    throw new Error("J2 requires J1's active visible retry rather than a fresh practice run");
  }
  if (surface === "error")
    throw new Error("rendered Mastery start surface reported an inline error");
  if (surface !== "run") throw new Error("J2 did not receive rendered retry controls");
  await expect(page.getByRole("radio")).not.toHaveCount(0);
}

async function waitForPostStartSurface(
  page: Page,
  freshPractice: ReturnType<Page["getByRole"]>,
): Promise<PostStartSurface> {
  const radios = page.getByRole("radio");
  const inlineErrors = page.locator(".inline-error:visible");
  let surface: PostStartSurface = "pending";
  try {
    await expect
      .poll(
        async () => {
          surface = classifyPostStartSurface({
            radios: await radios.count(),
            freshPractice: await freshPractice.isVisible(),
            inlineErrors: await inlineErrors.count(),
          });
          return surface;
        },
        { timeout: 30_000 },
      )
      .not.toBe("pending");
  } catch (error: unknown) {
    if (!isPollTimeout(error)) throw error;
    return "pending";
  }
  return surface;
}

function isPollTimeout(error: unknown): boolean {
  return error instanceof Error && error.message.includes("Timeout");
}

async function chooseAndSubmit(page: Page, choiceIndex: number): Promise<void> {
  if (choiceIndex !== 0 && choiceIndex !== 1) {
    throw new Error("J2 only supports the arranged two-choice visible retry path");
  }
  const radios = page.locator('input[type="radio"]:visible');
  await expect(radios).toHaveCount(2);
  await expect(radios.nth(0)).not.toBeChecked();
  await expect(radios.nth(1)).not.toBeChecked();
  const response = radios.nth(choiceIndex);
  await tabTo(page, response, choiceIndex === 0 ? "forward" : "backward");
  await expect(response).toBeFocused();
  await page.keyboard.press("Space");
  await expect(response).toBeChecked();
  const submit = page.getByRole("button", { name: "Submit answer" });
  await expect(submit).toBeEnabled();
  await tabTo(page, submit);
  await expect(submit).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible({
    timeout: 15_000,
  });
}

async function continueFromFeedback(page: Page): Promise<void> {
  const continueButton = page.getByRole("button", { name: "Continue" });
  await tabTo(page, continueButton);
  await expect(continueButton).toBeFocused();
  await page.keyboard.press("Space");
}

async function waitForFinalSurface(page: Page): Promise<FinalSurface> {
  const radios = page.locator('input[type="radio"]:visible');
  const freshPractice = page.getByRole("button", { name: "Start another practice run" });
  const inlineErrors = page.locator(".inline-error:visible");
  const continueButton = page.getByRole("button", { name: "Continue" });
  const feedback = page
    .locator(".question-card")
    .getByRole("heading", { name: "Feedback", exact: true });
  const neutralComplete = page.getByRole("heading", { name: "Run complete", exact: true });
  const closedComplete = page.getByRole("heading", { name: "This run is complete", exact: true });
  let surface: FinalSurface = "pending";
  try {
    await expect
      .poll(
        async () => {
          surface = classifyFinalSurface({
            radios: await radios.count(),
            freshPractice: await freshPractice.isVisible(),
            inlineErrors: await inlineErrors.count(),
            continueVisible: await continueButton.isVisible(),
            feedbackVisible: await feedback.isVisible(),
            neutralComplete: await neutralComplete.isVisible(),
            closedComplete: await closedComplete.isVisible(),
          });
          return isFinalSurfaceTerminal(surface);
        },
        { timeout: 30_000 },
      )
      .toBe(true);
  } catch (error: unknown) {
    if (!isPollTimeout(error)) throw error;
    return surface;
  }
  return surface;
}

test("J2 resumes the visible retry and completes the first instructor-created Mastery run", async ({
  page,
}) => {
  const startedAt = performance.now();
  await signInAndStartMastery(page);

  await chooseAndSubmit(page, 1);
  await continueFromFeedback(page);
  const finalSurface = await waitForFinalSurface(page);
  if (finalSurface === "run")
    throw new Error("final feedback cycle returned visible retry controls");
  if (finalSurface === "error")
    throw new Error("final feedback cycle returned a visible inline error");
  if (finalSurface === "feedback")
    throw new Error("final Feedback or Continue did not leave after visible Continue activation");
  if (finalSurface === "neutral")
    throw new Error("final feedback cycle reached neutral visible completion");
  if (finalSurface === "closed")
    throw new Error("final feedback cycle reached closed visible completion");
  if (finalSurface === "pending")
    throw new Error("final fresh-practice control did not appear in time");
  const freshPractice = page.getByRole("button", { name: "Start another practice run" });
  await expect(freshPractice).toBeVisible();
  await tabTo(page, freshPractice);
  await expect(freshPractice).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible({ timeout: 30_000 });
  const radios = page.locator('input[type="radio"]:visible');
  await expect(radios).toHaveCount(2, { timeout: 30_000 });
  await expect(radios.nth(0)).not.toBeChecked();
  await expect(radios.nth(1)).not.toBeChecked();
  await expect(page.locator("[data-route-surface=runAttempt] .eyebrow")).toHaveText(
    "Practice run 2",
  );
  await tabTo(page, radios.nth(0));
  await expect(radios.nth(0)).toBeFocused();
  await captureDocumentationScreenshot(page, "student_retake_fresh_problem.png");
  appendPassedJourneyState(Math.round(performance.now() - startedAt));
});
