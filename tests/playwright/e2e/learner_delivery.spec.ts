// Connected learner delivery proof. Product state is created through the visible production UI.
import { expect, test, type BrowserContext, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { liveDemoOriginReceiptPathFromEnvironment } from "../browser_suite_live_config";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
} from "./real_stack_ui";

const maryEmail = "mary.okafor@live-demo.ple.example";
const journeyTimeoutMs = 600_000;
const actionTimeoutMs = 30_000;
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };

function relativeIsoDate(offsetDays: number): string {
  const date = new Date();
  date.setDate(date.getDate() + offsetDays);
  const result = date.toISOString().slice(0, 10);
  return result;
}

function configureContext(context: BrowserContext): void {
  context.setDefaultTimeout(actionTimeoutMs);
  context.setDefaultNavigationTimeout(actionTimeoutMs);
}

function configurePage(page: Page): void {
  page.setDefaultTimeout(actionTimeoutMs);
  page.setDefaultNavigationTimeout(actionTimeoutMs);
}

function writeOriginReceipt(pageOrigins: Set<string>, requestOrigins: Set<string>): void {
  const receiptPath = liveDemoOriginReceiptPathFromEnvironment(process.env);
  const value = {
    pageOrigins: [...pageOrigins].sort(),
    requestOrigins: [...requestOrigins].sort(),
  };
  writeFileSync(receiptPath, JSON.stringify(value), { encoding: "ascii", flag: "wx", mode: 0o600 });
}

async function createPublishedQuestion(
  page: Page,
  title: string,
  correctChoice: string,
): Promise<string> {
  await page.getByRole("link", { name: "Workspace" }).click();
  await page.getByRole("button", { name: "Create flat question" }).click();
  await page.getByLabel("Question title").fill(title);
  await page.getByLabel("Learner-facing prompt").fill(`Which statement is correct? ${title}`);
  await page.getByLabel("Choice text").nth(0).fill(correctChoice);
  await page.getByLabel("Choice text").nth(1).fill(`Incorrect choice for ${title}`);
  await page
    .getByRole("radio", { name: new RegExp(`Mark choice 1 as correct: ${correctChoice}`) })
    .check();
  await page.getByRole("button", { name: "Save private draft" }).click();
  await page.getByRole("button", { name: "Review publication changes" }).click();
  await page.getByLabel("Reviewed public byline").fill("Dr. Elena Rivera");
  await page.getByRole("button", { name: "Confirm and publish" }).click();
  await expect(page.getByRole("heading", { name: "Published" })).toBeVisible();

  await page.getByRole("link", { name: "Library", exact: true }).click();
  await page.getByLabel("Search published questions").fill(title);
  const questionCard = page
    .getByRole("region", { name: "Published questions" })
    .getByText(title)
    .locator("..");
  await expect(questionCard).toBeVisible();
  const questionId = await questionCard.locator("code").innerText();
  expect(questionId).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);
  return questionId;
}

async function createPublishedCourseAssignment(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
  questionId: string,
  namespace: string,
): Promise<string> {
  await page.getByRole("link", { name: "Courses" }).click();
  await page.getByLabel("Course title").fill(courseTitle);
  await page.getByLabel("Start date").fill(relativeIsoDate(-30));
  await page.getByLabel("End date").fill(relativeIsoDate(365));
  await page.getByLabel("Time zone (IANA)").fill("America/Chicago");
  await page.getByRole("button", { name: "Create course" }).click();
  const courseCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: courseTitle, exact: true }) });
  await expect(courseCard).toHaveCount(1);
  await courseCard.getByRole("link", { name: "Open course", exact: true }).click();

  await page.getByRole("link", { name: "Assignments" }).click();
  await page.getByRole("link", { name: "Create the first assignment" }).click();
  await expect(page.getByRole("heading", { name: "Create assignment" })).toBeVisible();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByText("Add several Question IDs", { exact: true }).click();
  await page.getByLabel("Question IDs").fill(questionId);
  await page.getByRole("button", { name: "Add questions by ID" }).click();
  await page.getByRole("button", { name: "Create assignment" }).click();
  await expect(page.getByText(`${assignmentTitle} now appears in this course.`)).toBeVisible();
  await page.getByRole("link", { name: `Open ${assignmentTitle}` }).click();
  await page.getByLabel("Lifecycle").selectOption("published");
  await page.getByRole("button", { name: "Save teaching operations" }).click();
  await expect(page.getByText("Published, open now.")).toBeVisible();

  await page.getByRole("link", { name: "Students" }).click();
  await page.getByLabel("Institutional email").fill(maryEmail);
  await page.getByLabel("Institutional student ID").fill(`mary-${namespace}`);
  await page.getByRole("button", { name: "Create invitation" }).click();
  const invitation = page.getByLabel("Invitation link");
  await expect(invitation).toBeVisible();
  const invitationUrl = await invitation.inputValue();
  expect(new URL(invitationUrl).origin).toBe(new URL(page.url()).origin);
  return invitationUrl;
}

async function claimCourseAndCompleteAssignment(
  page: Page,
  invitationUrl: string,
  courseTitle: string,
  assignmentTitle: string,
  correctChoice: string,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await page.goto(invitationUrl);
  await expect(page.getByRole("heading", { name: "Join your PLE course" })).toBeVisible();
  await page.getByRole("button", { name: "Claim this course" }).click();
  await expect(page.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
  const assignmentCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await expect(assignmentCard).toHaveCount(1);
  await assignmentCard.getByRole("link", { name: "Start assignment", exact: true }).click();
  await expect(page.locator("[data-route-surface=assignmentOverview]")).toBeVisible();
  await page.getByRole("button", { name: "Start or continue practice" }).click();
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible();
  await page.getByRole("radio", { name: correctChoice, exact: true }).check();
  await page.getByRole("button", { name: "Submit answer" }).click();
  await expect(page.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Continue" }).click();
  const summary = page.locator(".attempt-summary");
  await expect(summary.getByText("Your completed run is recorded.")).toBeVisible();
  await expect(summary.getByRole("region", { name: "Assignment score" })).toBeVisible();
  await summary.getByRole("button", { name: "Start another practice" }).click();
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible();
  await expect(page.locator("[data-route-surface=runAttempt] .eyebrow")).toHaveText(
    "Practice run 2",
  );
  await expect(page.getByRole("radio", { name: correctChoice, exact: true })).not.toBeChecked();
  await page.getByRole("button", { name: "Back to assignment" }).click();
  await expect(page.locator("[data-route-surface=assignmentOverview]")).toBeVisible();
}

async function observeCompletedRunInFreshSession(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await selectVisibleCourse(page, courseTitle);
  const assignmentCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignmentCard.getByRole("link", { name: "Start assignment", exact: true }).click();
  const overview = page.locator("[data-route-surface=assignmentOverview]");
  await expect(overview).toBeVisible();
  const facts = overview.locator(".assignment-facts");
  await expect(facts.getByText("Score status", { exact: true })).toBeVisible();
  await expect(facts.getByText("Completed runs", { exact: true })).toBeVisible();
  const completedRuns = facts
    .getByText("Completed runs", { exact: true })
    .locator("..")
    .locator("dd");
  await expect(completedRuns).toHaveText(/[1-9]\d*/u);
}

test.describe.configure({ mode: "serial" });

test("learner delivery: Mary completes and revisits an instructor-created assignment", async ({
  browser,
}) => {
  test.setTimeout(journeyTimeoutMs);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("learner_delivery");
  expect(scenarioInput.sysadminRequirement).toBe("not_required");
  const tag = scenarioInput.namespace;
  const questionTitle = `Learner delivery question ${tag}`;
  const correctChoice = `Correct learner choice ${tag}`;
  const courseTitle = `Learner delivery course ${tag}`;
  const assignmentTitle = `Learner delivery assignment ${tag}`;
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
  const contexts: BrowserContext[] = [];

  try {
    const elenaContext = await browser.newContext(contextOptions);
    const maryContext = await browser.newContext(contextOptions);
    contexts.push(elenaContext, maryContext);
    for (const context of contexts) {
      configureContext(context);
      observeContextOrigins(context, pageOrigins, requestOrigins);
    }
    const elena = await elenaContext.newPage();
    const mary = await maryContext.newPage();
    configurePage(elena);
    configurePage(mary);

    await chooseSeededIdentity(elena, /Elena Rivera/u);
    await selectVisibleCourse(elena, "Biochemistry Base Course");
    const questionId = await createPublishedQuestion(elena, questionTitle, correctChoice);
    const invitationUrl = await createPublishedCourseAssignment(
      elena,
      courseTitle,
      assignmentTitle,
      questionId,
      tag,
    );
    await claimCourseAndCompleteAssignment(
      mary,
      invitationUrl,
      courseTitle,
      assignmentTitle,
      correctChoice,
    );
    await signOutVisible(mary);
    await maryContext.close();
    contexts.splice(contexts.indexOf(maryContext), 1);

    const freshMaryContext = await browser.newContext(contextOptions);
    contexts.push(freshMaryContext);
    configureContext(freshMaryContext);
    observeContextOrigins(freshMaryContext, pageOrigins, requestOrigins);
    expect(await freshMaryContext.storageState()).toEqual({ cookies: [], origins: [] });
    const freshMary = await freshMaryContext.newPage();
    configurePage(freshMary);
    await observeCompletedRunInFreshSession(freshMary, courseTitle, assignmentTitle);

    expect(pageOrigins.size).toBeGreaterThan(0);
    expect(requestOrigins.size).toBeGreaterThan(0);
    expect([...pageOrigins]).toEqual([expectedOrigin]);
    expect([...requestOrigins]).toEqual([expectedOrigin]);
    writeOriginReceipt(pageOrigins, requestOrigins);
  } finally {
    await Promise.all(contexts.map(async (context) => context.close()));
  }
});
