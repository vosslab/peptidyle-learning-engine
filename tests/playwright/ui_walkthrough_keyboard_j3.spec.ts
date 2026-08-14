// ui_walkthrough_keyboard_j3.spec.ts - J3 visible recovery through the platform keyboard path.
// Selector contract: src/pages/assignment_overview_page.tsx and src/pages/run_page.tsx.

import type { Page } from "@playwright/test";

import { expect, test } from "./ui_walkthrough_fixture";

import { tabTo, tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";
import {
  choosePlausibleVisibleResponse,
  expectVisibleResponseControlsCleared,
} from "./simulator/chapter_question_responses";
import {
  appendStudentRepeatState,
  passedStudentRepeatFragment,
} from "./simulator/student_repeat_state";
import {
  credentialFromValidatedFile,
  type UiWalkthroughInputs,
} from "./ui_walkthrough_config_factory";

test.describe.configure({ mode: "serial" });

test.setTimeout(90_000);

async function signInAndOpenMastery(page: Page, inputs: UiWalkthroughInputs): Promise<void> {
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

  const courseLink = page.locator(`a[href="/courses/${inputs.courseReference}"]`);
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
    `a[href="/courses/${inputs.courseReference}/assignments/${inputs.masteryAssignmentReference}"]`,
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

async function resumeActiveSecondMastery(page: Page): Promise<void> {
  const start = page.getByRole("button", { name: "Start or resume practice" });
  await tabTo(page, start);
  await expect(start).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible({ timeout: 30_000 });
  await expectVisibleResponseControlsCleared(page);
}

test("J3 resumes the active second Mastery run, proves cleared controls, and resumes it by keyboard", async ({
  page,
  uiWalkthroughInputs,
}) => {
  test.skip(uiWalkthroughInputs === undefined, "requires the explicit UI walkthrough config");
  if (uiWalkthroughInputs === undefined) return;
  const inputs = uiWalkthroughInputs;
  const startedAt = performance.now();
  await signInAndOpenMastery(page, inputs);
  await resumeActiveSecondMastery(page);
  await expectVisibleResponseControlsCleared(page);
  await choosePlausibleVisibleResponse(page);

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
  await expectVisibleResponseControlsCleared(page);
  appendStudentRepeatState(
    inputs.journeyStateFile,
    passedStudentRepeatFragment(
      "J3",
      inputs.courseReference,
      inputs.masteryAssignmentReference,
      Math.round(performance.now() - startedAt),
    ),
  );
});
