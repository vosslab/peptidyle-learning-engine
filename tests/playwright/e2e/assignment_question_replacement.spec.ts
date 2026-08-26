// Real-stack issued-work contract: visible replacement changes future runs, not issued work.
//
// Selector contract:
// - src/features/flat_question_authoring/flat_question_editor_page.tsx:535 owns question editing,
//   publication, and the question field labels.
// - src/pages/course_list_page.tsx:330 and src/pages/course_assignments_page.tsx:324 own course
//   creation and assignment navigation.
// - src/pages/assignment_editor_page.tsx:481 and src/pages/course_roster_page.tsx:423 own
//   assignment teaching settings and invitation controls.
// - src/pages/assignment_overview_page.tsx:114 and src/pages/run_page.tsx:387 own learner
//   assignment and attempt surfaces.
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { BIOCHEMISTRY_COURSE_TITLE } from "./helper_course_titles";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  observeContextOrigins,
  relativeIsoDate,
  requireScenarioInput,
  selectVisibleCourse,
  writeOriginReceipt,
} from "./real_stack_ui";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 300_000;
const maryEmail = "mary.okafor@live-demo.ple.example";

async function createPublishedQuestion(
  page: Page,
  title: string,
  correctChoice: string,
): Promise<string> {
  await page.getByRole("link", { name: "Workspace", exact: true }).click();
  await page.getByRole("button", { name: "Create flat question", exact: true }).click();
  await page.getByLabel("Question title").fill(title);
  await page.getByLabel("Learner-facing prompt").fill(`Choose the supported statement: ${title}`);
  await page.getByLabel("Choice text").nth(0).fill(correctChoice);
  await page.getByLabel("Choice text").nth(1).fill(`Alternative choice for ${title}`);
  await page
    .getByRole("radio", { name: new RegExp(`Mark choice 1 as correct: ${correctChoice}`) })
    .check();
  await page.getByRole("button", { name: "Save private draft", exact: true }).click();
  await page.getByRole("button", { name: "Review publication changes", exact: true }).click();
  await page.getByLabel("Reviewed public byline").fill("Dr. Elena Rivera");
  await page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Published", exact: true })).toBeVisible();
  await page.getByRole("link", { name: "Library", exact: true }).click();
  await page.getByLabel("Search published questions").fill(title);
  const card = page
    .getByRole("region", { name: "Published questions" })
    .getByText(title, { exact: true })
    .locator("..");
  await expect(card).toBeVisible();
  const questionId = await card.locator("code").innerText();
  expect(questionId).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);
  return questionId;
}

async function createPublishedAssignmentAndInvitation(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
  originalQuestionTitle: string,
  namespace: string,
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
  await page.getByRole("button", { name: "Choose questions", exact: true }).click();
  const picker = page.getByRole("dialog", { name: "Choose assignment questions", exact: true });
  await expect(picker).toBeVisible();
  await picker.getByLabel("Search questions", { exact: true }).fill(originalQuestionTitle);
  await picker.getByRole("button", { name: "Search questions", exact: true }).click();
  await picker.getByRole("checkbox", { name: new RegExp(originalQuestionTitle) }).check();
  await picker.getByRole("button", { name: "Add selected questions", exact: true }).click();
  await expect(picker).toHaveCount(0);
  await expect(page.locator(".assignment-editor-list")).toContainText(originalQuestionTitle);
  await page.getByRole("button", { name: "Create assignment", exact: true }).click();
  await expect(page.getByText(`${assignmentTitle} now appears in this course.`)).toBeVisible();
  await page.getByRole("link", { name: `Open ${assignmentTitle}`, exact: true }).click();
  await page.getByLabel("Lifecycle").selectOption("published");
  await page.getByRole("button", { name: "Save teaching operations", exact: true }).click();
  await expect(page.getByTestId("assignment-current-state")).toHaveText("Published, open now.");
  await page.getByRole("link", { name: "Students", exact: true }).click();
  await page.getByLabel("Institutional email").fill(maryEmail);
  await page.getByLabel("Institutional student ID").fill(`mary-${namespace}`);
  await page.getByRole("button", { name: "Create invitation", exact: true }).click();
  const invitation = page.getByLabel("Invitation link");
  await expect(invitation).toBeVisible();
  const invitationUrl = await invitation.inputValue();
  expect(new URL(invitationUrl).origin).toBe(new URL(page.url()).origin);
  return invitationUrl;
}

async function startIssuedRun(
  page: Page,
  invitationUrl: string,
  courseTitle: string,
  assignmentTitle: string,
  originalQuestionTitle: string,
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
  await page.getByRole("button", { name: "Start or continue practice", exact: true }).click();
  await expect(page.locator('[data-route-surface="runAttempt"]')).toBeVisible();
  await expect(
    page.getByRole("heading", { name: originalQuestionTitle, exact: true }),
  ).toBeVisible();
}

async function openAssignmentEditor(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
): Promise<void> {
  await chooseSeededIdentity(page, /Elena Rivera/u);
  await selectVisibleCourse(page, courseTitle);
  await page.getByRole("link", { name: "Assignments", exact: true }).click();
  const assignmentCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignmentCard.getByRole("link", { name: "Edit assignment", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Assignment editor", exact: true })).toBeVisible();
}

async function replaceAssignedQuestion(
  page: Page,
  originalQuestionTitle: string,
  replacementQuestionTitle: string,
): Promise<void> {
  const originalRow = page
    .locator(".assignment-editor-list")
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: originalQuestionTitle, exact: true }) });
  await expect(originalRow).toHaveCount(1);
  await originalRow.getByRole("button", { name: "Replace", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Replace assigned question", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Choose replacement", exact: true }).click();
  const picker = page.getByRole("dialog", {
    name: "Choose a replacement question",
    exact: true,
  });
  await expect(picker).toBeVisible();
  await picker.getByLabel("Search questions", { exact: true }).fill(replacementQuestionTitle);
  await picker.getByRole("button", { name: "Search questions", exact: true }).click();
  await picker.getByRole("radio", { name: new RegExp(replacementQuestionTitle) }).check();
  await picker.getByRole("button", { name: "Use selected replacement", exact: true }).click();
  await expect(picker).toHaveCount(0);
  await page.getByRole("button", { name: "Replace with selected question", exact: true }).click();
  await expect(
    page.getByText(
      "Replacement saved. Future runs use the replacement; issued work stays with its original question.",
      { exact: true },
    ),
  ).toBeVisible();
  await expect(page.locator(".assignment-editor-list")).toContainText(replacementQuestionTitle);
}

test.describe("assignment question replacement on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("issued work survives a visible replacement while a future run receives the new question", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("assignment_question_replacement");
    const tag = scenarioInput.namespace;
    const originalQuestionTitle = `Issued question ${tag}`;
    const originalChoice = `Original supported choice ${tag}`;
    const replacementQuestionTitle = `Replacement question ${tag}`;
    const replacementChoice = `Replacement supported choice ${tag}`;
    const courseTitle = `Replacement course ${tag}`;
    const assignmentTitle = `Replacement assignment ${tag}`;
    const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
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
      const replacingElenaContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      contexts.push(elenaContext, maryContext, replacingElenaContext);
      for (const context of contexts) observeContextOrigins(context, pageOrigins, requestOrigins);
      const elena = await elenaContext.newPage();
      const mary = await maryContext.newPage();
      const replacingElena = await replacingElenaContext.newPage();
      configureContextAndPage(elenaContext, elena, actionTimeoutMs);
      configureContextAndPage(maryContext, mary, actionTimeoutMs);
      configureContextAndPage(replacingElenaContext, replacingElena, actionTimeoutMs);

      await chooseSeededIdentity(elena, /Elena Rivera/u);
      await selectVisibleCourse(elena, BIOCHEMISTRY_COURSE_TITLE);
      await createPublishedQuestion(elena, originalQuestionTitle, originalChoice);
      await createPublishedQuestion(elena, replacementQuestionTitle, replacementChoice);
      const invitationUrl = await createPublishedAssignmentAndInvitation(
        elena,
        courseTitle,
        assignmentTitle,
        originalQuestionTitle,
        tag,
      );
      await openAssignmentEditor(elena, courseTitle, assignmentTitle);
      await startIssuedRun(
        mary,
        invitationUrl,
        courseTitle,
        assignmentTitle,
        originalQuestionTitle,
      );

      await openAssignmentEditor(replacingElena, courseTitle, assignmentTitle);
      await replaceAssignedQuestion(
        replacingElena,
        originalQuestionTitle,
        replacementQuestionTitle,
      );

      await elena.reload();
      await expect(
        elena.getByRole("heading", { name: "Assignment editor", exact: true }),
      ).toBeVisible();
      await expect(elena.locator(".assignment-editor-list")).toContainText(
        replacementQuestionTitle,
      );
      await expect(elena.locator(".assignment-editor-list")).not.toContainText(
        originalQuestionTitle,
      );

      await mary.reload();
      await expect(
        mary.getByRole("heading", { name: originalQuestionTitle, exact: true }),
      ).toBeVisible();
      await mary.getByRole("radio", { name: originalChoice, exact: true }).check();
      await mary.getByRole("button", { name: "Submit answer", exact: true }).click();
      await expect(mary.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible();
      await mary.getByRole("button", { name: "View completed run", exact: true }).click();
      await expect(
        mary.getByRole("button", { name: "Start fresh practice", exact: true }),
      ).toBeVisible();
      await mary.getByRole("button", { name: "Start fresh practice", exact: true }).click();
      await expect(
        mary.getByRole("heading", { name: replacementQuestionTitle, exact: true }),
      ).toBeVisible();
      await expect(mary.getByRole("radio", { name: replacementChoice, exact: true })).toBeVisible();

      expect([...pageOrigins].sort()).toEqual([expectedOrigin]);
      expect([...requestOrigins].sort()).toEqual([expectedOrigin]);
      originEvidenceVerified = true;
    } finally {
      try {
        await Promise.all(contexts.map((context) => context.close()));
      } finally {
        if (originEvidenceVerified) writeOriginReceipt(pageOrigins, requestOrigins);
      }
    }
  });
});
