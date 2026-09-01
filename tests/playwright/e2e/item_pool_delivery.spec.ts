// Production-stack item-pool journey: all teaching state and student work use visible PLE UI.
//
// Selector contract:
// - src/pages/assignment_workspace/ owns mixed fixed/pool creation and post-issue Questions saves.
// - src/pages/assignment_pool_editor.tsx:109 owns Question Pool Item, selection, ordering, and preview controls.
// - src/pages/assignment_workspace/assignment_workspace_policies_page.tsx owns publishing controls.
// - src/pages/assignment_attempt_page.tsx owns issued student questions, feedback, and completion surfaces.
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { advanceToNextIssuedQuestion, waitForAutomatedFeedback } from "./automated_grading_ui";
import { BIOCHEMISTRY_COURSE_TITLE } from "./helper_course_titles";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  observeContextOrigins,
  relativeIsoDate,
  requireScenarioInput,
  selectVisibleCourse,
  startOrContinuePractice,
  writeOriginReceipt,
} from "./real_stack_ui";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 300_000;
const maryEmail = "mary.okafor@live-demo.ple.example";

interface PublishedQuestion {
  readonly id: string;
  readonly title: string;
  readonly correctChoice: string;
}

async function selectQuestionsInPicker(
  page: Page,
  triggerName: string,
  dialogName: string,
  titles: ReadonlyArray<string>,
  confirmName: string,
): Promise<void> {
  await page.getByRole("button", { name: triggerName, exact: true }).click();
  const picker = page.getByRole("dialog", { name: dialogName, exact: true });
  await expect(picker).toBeVisible();
  for (const title of titles) {
    await picker.getByLabel("Search questions", { exact: true }).fill(title);
    await picker.getByRole("button", { name: "Search questions", exact: true }).click();
    await picker.getByRole("checkbox", { name: new RegExp(title) }).check();
  }
  await picker.getByRole("button", { name: confirmName, exact: true }).click();
  await expect(picker).toHaveCount(0);
}

async function createPublishedQuestion(
  page: Page,
  title: string,
  correctChoice: string,
): Promise<PublishedQuestion> {
  await page.getByRole("link", { name: "Workspace", exact: true }).click();
  await page.getByRole("button", { name: "Create Question", exact: true }).click();
  await page.getByLabel("Question title").fill(title);
  await page.getByLabel("Student-facing prompt").fill(`Choose the supported statement: ${title}`);
  await page.getByLabel("Choice text").nth(0).fill(correctChoice);
  await page.getByLabel("Choice text").nth(1).fill(`Alternative statement for ${title}`);
  await page
    .getByRole("radio", { name: new RegExp(`Mark choice 1 as correct: ${correctChoice}`) })
    .check();
  await page.getByRole("button", { name: "Save private draft", exact: true }).click();
  await page.getByRole("button", { name: "Review publication changes", exact: true }).click();
  await page.getByLabel("Question Authors").fill("Dr. Elena Rivera");
  await page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Published", exact: true })).toBeVisible();

  await page.getByRole("link", { name: "Library", exact: true }).click();
  await page.getByLabel("Search published questions").fill(title);
  const card = page
    .getByRole("region", { name: "Published questions" })
    .getByText(title, { exact: true })
    .locator("..");
  await expect(card).toBeVisible();
  const id = await card.locator("code").innerText();
  expect(id).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);
  return { id, title, correctChoice };
}

async function createCourseWithMixedPool(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
  fixed: PublishedQuestion,
  questionPoolItems: ReadonlyArray<PublishedQuestion>,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
): Promise<string> {
  await page.getByRole("link", { name: "Courses", exact: true }).click();
  await page.getByLabel("Course title").fill(courseTitle);
  await page.getByLabel("Start date").fill(relativeIsoDate(-30));
  await page.getByLabel("End date").fill(relativeIsoDate(365));
  await page.getByLabel("Time zone (IANA)").fill("America/Chicago");
  await page.getByRole("button", { name: "Create course", exact: true }).click();
  const courseCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: courseTitle, exact: true }) });
  await expect(courseCard).toHaveCount(1);
  await courseCard.getByRole("link", { name: "Open course", exact: true }).click();
  await page.getByRole("link", { name: "Assignments", exact: true }).click();
  await page.getByRole("link", { name: "Create the first assignment", exact: true }).click();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
  await selectQuestionsInPicker(
    page,
    "Search question library",
    "Choose assignment questions",
    [fixed.title],
    "Add selected questions",
  );
  await expect(page.locator(".assignment-editor-list")).toContainText(fixed.title);
  await page.getByRole("button", { name: "Add question pool", exact: true }).click();

  const pool = page.getByRole("listitem", { name: "Question pool at position 2" });
  await expect(pool).toBeVisible();
  await pool.getByRole("button", { name: "Choose Questions for pool", exact: true }).click();
  const picker = page.getByRole("dialog", { name: "Choose Questions for pool", exact: true });
  await expect(picker).toBeVisible();
  for (const questionPoolItem of questionPoolItems) {
    await picker.getByLabel("Search questions", { exact: true }).fill(questionPoolItem.title);
    await picker.getByRole("button", { name: "Search questions", exact: true }).click();
    await picker.getByRole("checkbox", { name: new RegExp(questionPoolItem.title) }).check();
  }
  await picker.getByRole("button", { name: "Add selected Questions to pool", exact: true }).click();
  await expect(picker).toHaveCount(0);
  for (const questionPoolItem of questionPoolItems)
    await expect(pool).toContainText(questionPoolItem.title);
  await pool.getByLabel("Selection count").fill("2");
  await pool.getByLabel("Points per selected Question").fill("2");
  await pool.getByLabel("Selected Question order").selectOption("questionPoolOrder");
  await expect(pool).toContainText("Question Pool Selection");
  await page.getByRole("button", { name: "Save questions and order", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Questions and order saved." }),
  ).toBeVisible();

  const savedPool = page.getByRole("listitem", { name: "Question pool at position 2" });
  await savedPool.getByRole("button", { name: "Preview draw", exact: true }).click();
  const preview = savedPool
    .getByRole("heading", { name: "Server-sampled draw", exact: true })
    .locator("..");
  await expect(preview).toBeVisible();
  const previewQuestionPoolItems = await preview
    .getByRole("list")
    .nth(0)
    .getByRole("listitem")
    .allTextContents();
  const previewSample = await preview
    .getByRole("list")
    .nth(1)
    .getByRole("listitem")
    .allTextContents();
  expect(previewQuestionPoolItems).toHaveLength(questionPoolItems.length);
  expect(previewSample).toHaveLength(2);
  expect(new Set(previewSample).size).toBe(2);
  for (const sample of previewSample) {
    expect(previewQuestionPoolItems).toContain(sample);
  }
  await preview.scrollIntoViewIfNeeded();
  await captureRealStackScreenshot(page, scenarioInput, "item_pool_delivery_pool_preview");
  await savedPool.getByRole("button", { name: "Preview another draw", exact: true }).click();
  await expect(
    savedPool.getByRole("heading", { name: "Server-sampled draw", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("status").filter({ hasText: "server sample is ready" }),
  ).toBeVisible();

  await page.getByRole("link", { name: "Policies", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Policies", exact: true })).toBeVisible();
  await page.getByLabel("Lifecycle").selectOption("released");
  await page.getByRole("button", { name: "Save assignment policies", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Assignment policies saved." }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Students", exact: true }).click();
  await page.getByLabel("Course roster email").fill(maryEmail);
  await page.getByLabel("Course roster ID").fill("BIO-MARY-003");
  await page.getByRole("button", { name: "Create invitation", exact: true }).click();
  const invitation = page.getByLabel("Invitation link");
  await expect(invitation).toBeVisible();
  const invitationUrl = await invitation.inputValue();
  expect(new URL(invitationUrl).origin).toBe(new URL(page.url()).origin);
  return invitationUrl;
}

async function completeDeliveredPoolRun(
  page: Page,
  invitationUrl: string,
  courseTitle: string,
  assignmentTitle: string,
  fixed: PublishedQuestion,
  questionPoolItems: ReadonlyArray<PublishedQuestion>,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await page.goto(invitationUrl);
  await expect(page.getByRole("heading", { name: "Join your PLE course" })).toBeVisible();
  await page.getByRole("button", { name: "Claim this course", exact: true }).click();
  await expect(page.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
  const assignmentCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignmentCard.getByRole("link", { name: "Start assignment", exact: true }).click();
  await startOrContinuePractice(page);

  const itemsByTitle = new Map(questionPoolItems.map((item) => [item.title, item]));
  await expect(page.getByRole("heading", { name: fixed.title, exact: true })).toBeVisible();
  await expect(page.locator(".assignment-attempt-question-pool-selection")).toHaveCount(0);
  await page.getByRole("radio", { name: fixed.correctChoice, exact: true }).check();
  await page.getByRole("button", { name: "Submit answer", exact: true }).click();
  const fixedFeedback = await waitForAutomatedFeedback(page);
  await expect(fixedFeedback.getByRole("heading", { name: "Correct", exact: true })).toBeVisible();
  await advanceToNextIssuedQuestion(page);

  const deliveredQuestionPoolItemIndexes: number[] = [];
  const questionHeading = page.locator(".run-header h1");
  for (let position = 0; position < 2; position += 1) {
    await expect(page.locator(".assignment-attempt-question-pool-selection")).toHaveText(
      `Server-selected Question Pool item ${position + 1} of 2 for this Assignment Attempt.`,
    );
    const title = await questionHeading.innerText();
    const questionPoolItem = itemsByTitle.get(title);
    expect(questionPoolItem).toBeDefined();
    const questionPoolItemIndex = questionPoolItems.findIndex((item) => item.title === title);
    expect(questionPoolItemIndex).toBeGreaterThanOrEqual(0);
    deliveredQuestionPoolItemIndexes.push(questionPoolItemIndex);
    if (position === 0) {
      await captureRealStackScreenshot(
        page,
        scenarioInput,
        "item_pool_delivery_learner_delivered_pool",
      );
    }
    await page.getByRole("radio", { name: questionPoolItem!.correctChoice, exact: true }).check();
    await page.getByRole("button", { name: "Submit answer", exact: true }).click();
    const questionPoolItemFeedback = await waitForAutomatedFeedback(page);
    await expect(
      questionPoolItemFeedback.getByRole("heading", { name: "Correct", exact: true }),
    ).toBeVisible();
    if (position === 0) {
      await advanceToNextIssuedQuestion(page);
    } else {
      await page.getByRole("button", { name: "View completed run", exact: true }).click();
    }
  }
  expect(deliveredQuestionPoolItemIndexes).toEqual(
    [...deliveredQuestionPoolItemIndexes].sort((a, b) => a - b),
  );
  const summary = page.locator(".attempt-summary");
  await expect(summary.getByText("Your completed run is recorded.")).toBeVisible();
  await expect(summary.getByRole("region", { name: "Assignment score" })).toContainText("100%");
  await page.reload();
  await expect(
    page.locator(".attempt-summary").getByText("Your completed run is recorded."),
  ).toBeVisible();
}

async function inspectPostIssueEdits(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
): Promise<void> {
  await chooseSeededIdentity(page, /Elena Rivera/u);
  await selectVisibleCourse(page, courseTitle);
  await page.getByRole("link", { name: "Gradebook", exact: true }).click();
  const gradebook = page.locator("[data-route-surface=gradebook]");
  await expect(gradebook).toBeVisible();
  const gradebookRow = gradebook
    .getByRole("row")
    .filter({ has: page.getByText("Mary Okafor", { exact: true }) });
  await expect(gradebookRow).toBeVisible();
  await expect(gradebookRow.locator('[data-label="Course total"]')).toContainText("100%");
  await expect(gradebookRow.locator(`[data-label="${assignmentTitle}"]`)).toContainText("100%");
  await page.getByRole("link", { name: "Assignments", exact: true }).click();
  const assignmentCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignmentCard.getByRole("link", { name: assignmentTitle, exact: true }).click();
  await page.getByRole("link", { name: "Questions", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
  const pool = page.getByRole("listitem", { name: "Question pool at position 2" });
  const revisedTitle = `${assignmentTitle} renamed`;
  await page.getByLabel("Assignment title").fill(revisedTitle);
  await page.getByRole("button", { name: "Save questions and order", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Questions and order saved." }),
  ).toBeVisible();
  await expect(page.getByLabel("Assignment title")).toHaveValue(revisedTitle);
  await expect(pool.getByLabel("Points per drawn question")).toHaveValue("2");

  await pool.getByLabel("Points per selected Question").fill("3");
  await pool.getByLabel("Selection count").fill("1");
  await page.getByRole("button", { name: "Save questions and order", exact: true }).click();
  const recovery = page.getByRole("alert");
  await expect(recovery).toContainText("Student work has already been issued");
  await expect(recovery).toContainText("Your local question changes remain here");
  await expect(recovery).toContainText("issued student work remains unchanged");
  await expect(recovery.getByRole("link", { name: "Create a new assignment" })).toBeVisible();
  await expect(pool.getByLabel("Points per selected Question")).toHaveValue("3");
  await expect(pool.getByLabel("Selection count")).toHaveValue("1");
}

test.describe("item-pool delivery on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("server previews and delivers a fixed item plus an ordered immutable Question Pool Selection", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("item_pool_delivery");
    const courseTitle = "Biochemistry: Protein Structure Variations";
    const assignmentTitle = "Peptide Bonds: Mixed Practice";
    const pageOrigins = new Set<string>();
    const requestOrigins = new Set<string>();
    const contexts: BrowserContext[] = [];
    let originEvidenceVerified = false;

    try {
      const elenaContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      const maryContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      const inspectionContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      contexts.push(elenaContext, maryContext, inspectionContext);
      for (const context of contexts) observeContextOrigins(context, pageOrigins, requestOrigins);
      const elena = await elenaContext.newPage();
      const mary = await maryContext.newPage();
      const inspectingElena = await inspectionContext.newPage();
      for (const [context, page] of [
        [elenaContext, elena],
        [maryContext, mary],
        [inspectionContext, inspectingElena],
      ] as const) {
        configureContextAndPage(context, page, actionTimeoutMs);
      }

      await chooseSeededIdentity(elena, /Elena Rivera/u);
      await selectVisibleCourse(elena, BIOCHEMISTRY_COURSE_TITLE);
      const fixed = await createPublishedQuestion(
        elena,
        "Peptide Bond Geometry",
        "Peptide bonds are usually planar",
      );
      const questionPoolItems: PublishedQuestion[] = [];
      for (const label of ["one", "two", "three"]) {
        questionPoolItems.push(
          await createPublishedQuestion(
            elena,
            `Peptide Bond Variation ${label}`,
            `Supported peptide-bond statement ${label}`,
          ),
        );
      }
      const invitationUrl = await createCourseWithMixedPool(
        elena,
        courseTitle,
        assignmentTitle,
        fixed,
        questionPoolItems,
        scenarioInput,
      );
      await completeDeliveredPoolRun(
        mary,
        invitationUrl,
        courseTitle,
        assignmentTitle,
        fixed,
        questionPoolItems,
        scenarioInput,
      );
      await inspectPostIssueEdits(inspectingElena, courseTitle, assignmentTitle);

      const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
      expect([...pageOrigins].sort()).toEqual([expectedOrigin]);
      expect([...requestOrigins].sort()).toEqual([expectedOrigin]);
      originEvidenceVerified = true;
    } finally {
      try {
        await Promise.all(contexts.map(async (context) => await context.close()));
      } finally {
        if (originEvidenceVerified) writeOriginReceipt(pageOrigins, requestOrigins);
      }
    }
  });
});
