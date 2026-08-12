// ui_walkthrough_keyboard_j4.spec.ts - J4 completes the second visible Mastery run.

import type { Page } from "@playwright/test";

import { expect, test } from "./ui_walkthrough_fixture";

import { tabTo, tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";
import { completeVisibleQuestionThroughFeedback } from "./simulator/chapter_question_responses";
import { classifyPostStartSurface, type PostStartSurface } from "./simulator/post_start_surface";
import {
  appendStudentRepeatState,
  passedStudentRepeatFragment,
} from "./simulator/student_repeat_state";
import {
  credentialFromValidatedFile,
  type UiWalkthroughInputs,
} from "./ui_walkthrough_config_factory";
import { captureDocumentationScreenshot } from "./docs_screenshot_capture";

test.describe.configure({ mode: "serial" });

test.setTimeout(90_000);

async function signInAndResumeSecondRun(page: Page, inputs: UiWalkthroughInputs): Promise<void> {
  await page.goto("/");
  const credential = page.getByLabel("Local development credential");
  await tabTo(page, credential);
  await expect(credential).toBeFocused();
  await credential.fill(credentialFromValidatedFile(inputs.credentialFile, "student"));
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

async function waitForNextMasterySurface(page: Page): Promise<"run" | "complete"> {
  const radios = page.getByRole("radio");
  const freshPractice = page.getByRole("button", { name: "Start another practice run" });
  const inlineErrors = page.locator(".inline-error:visible");
  let surface: PostStartSurface = "pending";
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
  if (surface === "error") {
    throw new Error("rendered Mastery continuation surface reported an inline error");
  }
  if (surface === "fresh-practice") return "complete";
  if (surface === "run") return "run";
  throw new Error("rendered Mastery continuation surface remained unavailable");
}

test("J4 completes the resumed second Mastery run through visible keyboard controls", async ({
  page,
  uiWalkthroughInputs,
}) => {
  test.skip(uiWalkthroughInputs === undefined, "requires the explicit UI walkthrough config");
  if (uiWalkthroughInputs === undefined) return;
  const inputs = uiWalkthroughInputs;
  const startedAt = performance.now();
  await signInAndResumeSecondRun(page, inputs);
  // J3 intentionally left this new run without submitting. Complete its rendered
  // questions through visible response controls and visible correctness feedback.
  for (let questionCount = 0; questionCount < 4; questionCount += 1) {
    await completeVisibleQuestionThroughFeedback(page);
    const surface = await waitForNextMasterySurface(page);
    if (surface === "complete") break;
  }
  const completed = await waitForNextMasterySurface(page);
  if (completed !== "complete") {
    throw new Error(
      "four rendered question completions did not reach the Mastery completion surface",
    );
  }
  await expect(
    page.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
  ).toBeVisible({ timeout: 30_000 });
  const freshPractice = page.getByRole("button", { name: "Start another practice run" });
  await expect(freshPractice).toBeVisible();
  await tabTo(page, freshPractice);
  await expect(freshPractice).toBeFocused();
  await captureDocumentationScreenshot(
    page,
    "student_fresh_practice.png",
    page.locator(".attempt-summary"),
    72,
    inputs.screenshotDirectory,
  );
  const back = page.getByRole("button", { name: "Back to assignment" });
  await tabTo(page, back);
  await expect(back).toBeFocused();
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
