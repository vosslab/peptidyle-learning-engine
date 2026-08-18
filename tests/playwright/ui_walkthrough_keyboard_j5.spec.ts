// ui_walkthrough_keyboard_j5.spec.ts - J5 instructor gradebook through rendered keyboard controls.

import type { Page } from "@playwright/test";

import { expect, test } from "./ui_walkthrough_fixture";

import {
  instructorGradebookLinkSelector,
  passedJ5SummaryEvidence,
} from "./simulator/instructor_gradebook_j5";
import { j5V2Input } from "./simulator/j5_v2_handoff";
import { tabTo, tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";
import { closeThenAppendV2J5State } from "./simulator/v2_j5_j8_state";
import {
  credentialFromValidatedFile,
  type UiWalkthroughInputs,
} from "./ui_walkthrough_config_factory";
import { captureDocumentationScreenshot } from "./docs_screenshot_capture";

test.setTimeout(120_000);

interface J5VisibleAssignment {
  readonly title: string;
}

async function signInAndOpenGradebook(
  page: Page,
  inputs: UiWalkthroughInputs,
): Promise<J5VisibleAssignment> {
  await page.goto("/");
  const credentialInput = page.getByLabel("Local development credential");
  await tabTo(page, credentialInput);
  await expect(credentialInput).toBeFocused();
  await credentialInput.fill(credentialFromValidatedFile(inputs.credentialFile, "instructor"));
  const signIn = page.getByRole("button", { name: "Sign in locally" });
  await tabTo(page, signIn);
  await expect(signIn).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=courses]")).toBeVisible({ timeout: 30_000 });

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
  await expect(page.locator("[data-route-surface=courseAssignments]")).toBeVisible({
    timeout: 30_000,
  });

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
        .getByRole("link", { name: "Start assignment", exact: true }),
    itemName: "assignments",
  });
  await expect(assignmentLink).toHaveCount(1);
  await expect(assignmentLink).toBeVisible();
  const assignmentCard = assignmentLink.locator(
    "xpath=ancestor::article[contains(@class, 'course-card')]",
  );
  const assignmentHeading = assignmentCard.getByRole("heading");
  await expect(assignmentHeading).toHaveCount(1);
  const title = (await assignmentHeading.innerText()).trim();
  if (title === "") throw new Error("visible assignment card has no heading");

  const gradebookLink = page.locator(instructorGradebookLinkSelector(inputs.courseReference));
  await expect(gradebookLink).toHaveCount(1);
  await tabTo(page, gradebookLink);
  await expect(gradebookLink).toBeFocused();
  await page.keyboard.press("Enter");
  return { title };
}

test("J5 instructor opens gradebook run history with the keyboard after learner activity", async ({
  browser,
  uiWalkthroughInputs,
}) => {
  test.skip(uiWalkthroughInputs === undefined, "requires the explicit UI walkthrough config");
  if (uiWalkthroughInputs === undefined) return;
  const inputs = uiWalkthroughInputs;
  const startedAt = performance.now();
  const context = await browser.newContext({ baseURL: inputs.baseUrl });
  let evidence: ReturnType<typeof passedJ5SummaryEvidence> | undefined;
  try {
    const page = await context.newPage();
    const assignment = await signInAndOpenGradebook(page, inputs);

    await expect(page.locator("[data-route-surface=gradebook]")).toBeVisible({ timeout: 30_000 });
    const row = page
      .locator("tr.gradebook-row")
      .filter({ has: page.getByRole("rowheader", { name: assignment.title, exact: true }) })
      .filter({ has: page.locator('td[data-label="Best"]', { hasText: /^100%$/u }) })
      .filter({ has: page.locator('td[data-label="Latest"]', { hasText: /^100%$/u }) })
      .filter({ has: page.locator('td[data-label="Completed"]', { hasText: /^2$/u }) });
    const historyButton = row.getByRole("button", { name: "View run history", exact: true });
    await tabToTargetThroughVisiblePagination(page, {
      target: row,
      keyboardTarget: historyButton,
      renderedItems: page.locator(".gradebook-row"),
      firstAppendedControl: (index) =>
        page
          .locator(".gradebook-row")
          .nth(index)
          .getByRole("button", { name: "View run history", exact: true }),
      itemName: "gradebook records",
    });
    await expect(row).toHaveCount(1);
    await expect(row).toBeVisible();
    await expect(row.locator('td[data-label="Best"]')).toHaveText("100%");
    await expect(row.locator('td[data-label="Latest"]')).toHaveText("100%");
    await expect(row.locator('td[data-label="Completed"]')).toHaveText("2");
    await expect(historyButton).toHaveCount(1);
    await tabTo(page, historyButton);
    await expect(historyButton).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(historyButton).toHaveAttribute("aria-expanded", "true");
    const controls = await historyButton.getAttribute("aria-controls");
    if (controls === null) throw new Error("visible gradebook history control has no target");
    const runHistory = page.getByRole("region", { name: /^Run history for learner /u });
    await expect(runHistory).toHaveCount(1);
    await expect(runHistory).toHaveAttribute("id", controls);
    await expect(runHistory).toBeVisible({ timeout: 30_000 });
    const completedRuns = runHistory.locator(".run-history-list > li");
    await expect(completedRuns).toHaveCount(2, { timeout: 30_000 });
    for (const [index, runNumber] of [1, 2].entries()) {
      const completedRun = completedRuns.nth(index);
      await expect(completedRun.getByText(`Run ${runNumber}`, { exact: true })).toBeVisible();
      await expect(completedRun).toContainText("Completed, 100%");
    }
    await captureDocumentationScreenshot(
      page,
      "instructor_gradebook_mastery_loop.png",
      undefined,
      undefined,
      inputs.screenshotDirectory,
    );

    const publicIds = j5V2Input(inputs.courseReference, inputs.masteryAssignmentReference);
    evidence = passedJ5SummaryEvidence(
      publicIds.courseReference,
      publicIds.assignmentReference,
      Math.round(performance.now() - startedAt),
    );
  } finally {
    if (evidence === undefined) {
      await context.close();
    } else {
      await closeThenAppendV2J5State(inputs.journeyStateFile, evidence, () => context.close());
    }
  }
  if (evidence === undefined) throw new Error("visible J5 evidence was not produced");
});
