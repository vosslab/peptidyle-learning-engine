// Connected WebWork delivery proof. All teaching and student state uses the visible PLE UI.
//
// Selector contract:
// - src/pages/library_page.tsx:117 owns published-question search, cards, and question IDs.
// - src/pages/course_list_page.tsx, course_assignments_page.tsx, and assignment_workspace/ own
//   course creation and the title-first assignment workflow.
// - src/pages/course_roster_page.tsx:423 owns student invitation fields and the invitation link.
// - src/pages/course_invitation_page.tsx:62 and src/pages/assignment_overview_page.tsx:114 own
//   student claiming and practice entry.
// - src/pages/assignment_attempt_page.tsx and src/components/responses/common.tsx own Assignment Attempt visibility,
//   answer controls, feedback, and completion navigation.
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { waitForAutomatedFeedback } from "./automated_grading_ui";
import { BIOCHEMISTRY_COURSE_TITLE } from "./helper_course_titles";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  observeContextOrigins,
  relativeIsoDate,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
  startOrContinuePractice,
  writeOriginReceipt,
} from "./real_stack_ui";
import {
  requireWebworkCatalogBaselineInput,
  writeVisibleIssuanceAcknowledgement,
} from "./webwork_delivery_input";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 360_000;
const maryEmail = "mary.okafor@live-demo.ple.example";
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };

async function findCatalogQuestion(page: Page, title: string, questionId: string): Promise<void> {
  await page.getByRole("link", { name: "Library", exact: true }).click();
  await page.getByLabel("Search published questions").fill(title);
  const catalog = page.getByRole("region", { name: "Published questions" });
  const card = catalog.getByText(title, { exact: true }).locator("..");
  await expect(card).toBeVisible();
  await expect(card.locator("code")).toHaveText(questionId);
}

async function createCourseAssignmentAndInvitation(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
  questionId: string,
  scenarioNamespace: string,
): Promise<string> {
  await page.getByRole("link", { name: "Courses", exact: true }).click();
  await page.getByLabel("Course title").fill(courseTitle);
  await page.getByLabel("Start date").fill(relativeIsoDate(-30));
  await page.getByLabel("End date").fill(relativeIsoDate(365));
  await page.getByLabel("Time zone (IANA)").fill("America/Chicago");
  await page.getByRole("button", { name: "Create course", exact: true }).click();
  const course = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: courseTitle, exact: true }) });
  await expect(course).toHaveCount(1);
  await course.getByRole("link", { name: "Open course", exact: true }).click();
  await page.getByRole("link", { name: "Assignments", exact: true }).click();
  await page.getByRole("link", { name: "Create the first assignment", exact: true }).click();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByRole("button", { name: "Create assignment draft", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
  await page.getByLabel("Question IDs").fill(questionId);
  await page.getByRole("button", { name: "Add Question IDs", exact: true }).click();
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: "Added 1 Question ID. Save questions and order when ready." }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Save questions and order", exact: true }).click();
  await page.getByRole("link", { name: "Policies", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Policies", exact: true })).toBeVisible();
  await page.getByLabel("Completion requirement").selectOption("answerAll");
  await page.getByLabel("Lifecycle").selectOption("published");
  await page.getByRole("button", { name: "Save assignment policies", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Assignment policies saved." }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Students", exact: true }).click();
  await page.getByLabel("Course roster email").fill(maryEmail);
  await page.getByLabel("Course roster ID").fill(`mary-${scenarioNamespace}`);
  await page.getByRole("button", { name: "Create invitation", exact: true }).click();
  const invitation = page.getByLabel("Invitation link");
  await expect(invitation).toBeVisible();
  const url = await invitation.inputValue();
  expect(new URL(url).origin).toBe(new URL(page.url()).origin);
  return url;
}

async function completeVisibleWebworkRun(
  page: Page,
  invitationUrl: string,
  courseTitle: string,
  assignmentTitle: string,
  questionTitle: string,
  scenarioNamespace: string,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await page.goto(invitationUrl);
  await expect(
    page.getByRole("heading", { name: "Join your PLE course", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Claim this course", exact: true }).click();
  await expect(page.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
  const assignment = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignment.getByRole("link", { name: "Start assignment", exact: true }).click();
  await startOrContinuePractice(page);
  await expect(page.getByRole("heading", { name: questionTitle, exact: true })).toBeVisible();
  writeVisibleIssuanceAcknowledgement(
    process.env,
    requireWebworkCatalogBaselineInput(process.env),
    scenarioNamespace,
  );
  const choices = page.getByRole("radio");
  await expect(choices).toHaveCount(5);
  await choices.first().check();
  await expect(choices.first()).toBeChecked();
  await page.getByRole("button", { name: "Submit answer", exact: true }).click();
  const feedback = await waitForAutomatedFeedback(page);
  await expect(feedback.getByRole("heading", { name: /^(Correct|Not quite)$/u })).toBeVisible();
  await page.getByRole("button", { name: "View completed run", exact: true }).click();
  await expect(page.getByText("Your completed run is recorded.")).toBeVisible();
}

async function observeCompletionInFreshSession(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await selectVisibleCourse(page, courseTitle);
  const assignment = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignment.getByRole("link", { name: "Start assignment", exact: true }).click();
  const overview = page.locator('[data-route-surface="assignmentOverview"]');
  await expect(overview).toBeVisible();
  const completedRuns = overview
    .getByText("Completed runs", { exact: true })
    .locator("..")
    .locator("dd");
  const scoreStatus = overview
    .getByText("Score status", { exact: true })
    .locator("..")
    .getByRole("status");
  await expect(completedRuns).toHaveText("1");
  await expect(scoreStatus).toHaveText(
    /^Score available: Current \d+(?:\.\d+)?%, Latest \d+(?:\.\d+)?%, Best \d+(?:\.\d+)?%\.$/u,
  );
}

test.describe.configure({ mode: "serial" });

test("WebWork delivery: Elena assigns reviewed catalog material and Mary completes it", async ({
  browser,
}) => {
  test.setTimeout(scenarioTimeoutMs);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  const baseline = requireWebworkCatalogBaselineInput(process.env);
  expect(scenarioInput.scenarioId).toBe("webwork_delivery");
  expect(baseline.scenarioId).toBe(scenarioInput.scenarioId);
  const courseTitle = "Biochemistry: WebWork Practice";
  const assignmentTitle = "Peptide Bond WebWork Practice";
  const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  const contexts: BrowserContext[] = [];
  let originEvidenceVerified = false;

  try {
    const elenaContext = await browser.newContext(contextOptions);
    const maryContext = await browser.newContext(contextOptions);
    contexts.push(elenaContext, maryContext);
    for (const context of contexts) observeContextOrigins(context, pageOrigins, requestOrigins);
    const elena = await elenaContext.newPage();
    const mary = await maryContext.newPage();
    configureContextAndPage(elenaContext, elena, actionTimeoutMs);
    configureContextAndPage(maryContext, mary, actionTimeoutMs);

    await chooseSeededIdentity(elena, /Elena Rivera/u);
    await selectVisibleCourse(elena, BIOCHEMISTRY_COURSE_TITLE);
    await findCatalogQuestion(elena, baseline.title, baseline.questionId);
    const invitationUrl = await createCourseAssignmentAndInvitation(
      elena,
      courseTitle,
      assignmentTitle,
      baseline.questionId,
      scenarioInput.namespace,
    );
    await completeVisibleWebworkRun(
      mary,
      invitationUrl,
      courseTitle,
      assignmentTitle,
      baseline.title,
      scenarioInput.namespace,
    );
    await signOutVisible(mary);
    await maryContext.close();
    contexts.splice(contexts.indexOf(maryContext), 1);

    const freshMaryContext = await browser.newContext(contextOptions);
    contexts.push(freshMaryContext);
    observeContextOrigins(freshMaryContext, pageOrigins, requestOrigins);
    expect(await freshMaryContext.storageState()).toEqual({ cookies: [], origins: [] });
    const freshMary = await freshMaryContext.newPage();
    configureContextAndPage(freshMaryContext, freshMary, actionTimeoutMs);
    await observeCompletionInFreshSession(freshMary, courseTitle, assignmentTitle);
    await signOutVisible(freshMary);
    await freshMaryContext.close();
    contexts.splice(contexts.indexOf(freshMaryContext), 1);

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
