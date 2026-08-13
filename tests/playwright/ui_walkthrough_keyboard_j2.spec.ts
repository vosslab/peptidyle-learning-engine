// ui_walkthrough_keyboard_j2.spec.ts - J2 active-run continuation through rendered keyboard controls.

import { expect, test } from "./ui_walkthrough_fixture";
import type { Page } from "@playwright/test";

import { classifyPostStartSurface, type PostStartSurface } from "./simulator/post_start_surface";
import { tabTo, tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";
import { expectVisibleResponseControlsCleared } from "./simulator/chapter_question_responses";
import { completeReviewedGeneticsQuestion } from "./simulator/genetics_chapter_one_responses";
import {
  appendStudentRepeatState,
  passedStudentRepeatFragment,
} from "./simulator/student_repeat_state";
import { writeJ2Checkpoint, type J2Checkpoint } from "./simulator/j2_checkpoint";
import {
  credentialFromValidatedFile,
  type UiWalkthroughInputs,
} from "./ui_walkthrough_config_factory";
import { captureDocumentationScreenshot } from "./docs_screenshot_capture";

test.describe.configure({ mode: "serial" });

test.setTimeout(90_000);

function appendPassedJourneyState(inputs: UiWalkthroughInputs, elapsedMs: number): void {
  appendStudentRepeatState(
    inputs.journeyStateFile,
    passedStudentRepeatFragment("J2", inputs.courseId, inputs.masteryAssignmentId, elapsedMs),
  );
}

function checkpoint(inputs: UiWalkthroughInputs, stage: J2Checkpoint): void {
  writeJ2Checkpoint(inputs.j2CheckpointFile, stage);
}

function responseProgress(inputs: UiWalkthroughInputs): {
  readonly responseSelected: () => void;
  readonly feedbackVisible: () => void;
} {
  return {
    responseSelected: () => checkpoint(inputs, "response_selected"),
    feedbackVisible: () => checkpoint(inputs, "feedback_visible"),
  };
}

async function signInAndStartMastery(page: Page, inputs: UiWalkthroughInputs): Promise<void> {
  await page.goto("/");
  const credentialInput = page.getByLabel("Local development credential");
  await tabTo(page, credentialInput);
  await expect(credentialInput).toBeFocused();
  await credentialInput.fill(credentialFromValidatedFile(inputs.credentialFile, "student"));
  const signIn = page.getByRole("button", { name: "Sign in locally" });
  await tabTo(page, signIn);
  await expect(signIn).toBeFocused();
  await page.keyboard.press("Enter");
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
  await captureDocumentationScreenshot(
    page,
    "genetics_chapter_one_overview.png",
    undefined,
    undefined,
    inputs.screenshotDirectory,
  );
  await page.keyboard.press("Space");
  const freshPractice = page.getByRole("button", { name: "Start another practice run" });
  const surface = await waitForPostStartSurface(page, freshPractice);
  if (surface === "fresh-practice") {
    throw new Error("J2 requires J1's active first run rather than a fresh practice run");
  }
  if (surface === "error")
    throw new Error("rendered Mastery start surface reported an inline error");
  if (surface !== "run") throw new Error("J2 did not receive rendered active-run controls");
  await expect(page.getByRole("radio")).not.toHaveCount(0);
  checkpoint(inputs, "active_run_visible");
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

async function waitForNextMasterySurface(page: Page): Promise<"run" | "complete"> {
  const radios = page.getByRole("radio");
  const freshPractice = page.getByRole("button", { name: "Start another practice run" });
  const inlineErrors = page.locator(".inline-error:visible");
  // Keep the observed state in an object: `expect.poll` invokes its callback later,
  // so TypeScript correctly cannot prove an outer local variable was reassigned.
  const observed: { surface: PostStartSurface } = { surface: "pending" };
  await expect
    .poll(
      async () => {
        observed.surface = classifyPostStartSurface({
          radios: await radios.count(),
          freshPractice: await freshPractice.isVisible(),
          inlineErrors: await inlineErrors.count(),
        });
        return observed.surface;
      },
      { timeout: 30_000 },
    )
    .not.toBe("pending");
  if (observed.surface === "error") {
    throw new Error("rendered Mastery continuation surface reported an inline error");
  }
  if (observed.surface === "fresh-practice") return "complete";
  if (observed.surface === "run") return "run";
  throw new Error("rendered Mastery continuation surface remained unavailable");
}

test("J2 resumes the active first run and completes the instructor-created Mastery assignment", async ({
  page,
  uiWalkthroughInputs,
}) => {
  test.skip(uiWalkthroughInputs === undefined, "requires the explicit UI walkthrough config");
  if (uiWalkthroughInputs === undefined) return;
  const inputs = uiWalkthroughInputs;
  const startedAt = performance.now();
  await signInAndStartMastery(page, inputs);

  // J1 already proved the visible incorrect-feedback/retry transition. Complete the reviewed
  // teaching content here from learner-visible biology prose and visible keyboard controls.
  for (let questionCount = 0; questionCount < 4; questionCount += 1) {
    await completeReviewedGeneticsQuestion(page, responseProgress(inputs));
    const surface = await waitForNextMasterySurface(page);
    if (surface === "complete") break;
  }
  const completed = await waitForNextMasterySurface(page);
  if (completed !== "complete") {
    throw new Error(
      "four rendered question completions did not reach the Mastery completion surface",
    );
  }
  const freshPractice = page.getByRole("button", { name: "Start another practice run" });
  await expect(freshPractice).toBeVisible();
  checkpoint(inputs, "first_run_completed");
  await tabTo(page, freshPractice);
  await expect(freshPractice).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible({ timeout: 30_000 });
  await expectVisibleResponseControlsCleared(page);
  await expect(page.locator("[data-route-surface=runAttempt] .eyebrow")).toHaveText(
    "Practice run 2",
  );
  checkpoint(inputs, "fresh_practice_visible");
  await captureDocumentationScreenshot(
    page,
    "student_retake_fresh_problem.png",
    undefined,
    undefined,
    inputs.screenshotDirectory,
  );
  appendPassedJourneyState(inputs, Math.round(performance.now() - startedAt));
});
