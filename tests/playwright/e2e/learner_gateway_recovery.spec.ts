// A real learner retry remains visible and durable while the owner recovers the gateway.
import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { liveDemoOriginReceiptPathFromEnvironment } from "../browser_suite_live_config";
import { faultHandshakeFromEnvironment } from "./fault_handshake";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
} from "./real_stack_ui";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";

const maryEmail = "mary.okafor@live-demo.ple.example";
const timeoutMs = 600_000;
const actionTimeoutMs = 30_000;
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };

interface ObservedOrigins {
  readonly pageOrigins: Set<string>;
  readonly requestOrigins: Set<string>;
}

function isoDate(offset: number): string {
  const date = new Date();
  date.setDate(date.getDate() + offset);
  return date.toISOString().slice(0, 10);
}

function configure(context: BrowserContext, page: Page): void {
  context.setDefaultTimeout(actionTimeoutMs);
  context.setDefaultNavigationTimeout(actionTimeoutMs);
  page.setDefaultTimeout(actionTimeoutMs);
  page.setDefaultNavigationTimeout(actionTimeoutMs);
}

function writeOriginReceipt(contexts: Readonly<Record<string, ObservedOrigins>>): void {
  const pages = new Set<string>();
  const requests = new Set<string>();
  for (const value of Object.values(contexts)) {
    for (const origin of value.pageOrigins) pages.add(origin);
    for (const origin of value.requestOrigins) requests.add(origin);
  }
  writeFileSync(
    liveDemoOriginReceiptPathFromEnvironment(process.env),
    JSON.stringify({
      pageOrigins: [...pages].sort(),
      requestOrigins: [...requests].sort(),
      contexts: Object.fromEntries(
        Object.entries(contexts).map(([name, value]) => [
          name,
          {
            pageOrigins: [...value.pageOrigins].sort(),
            requestOrigins: [...value.requestOrigins].sort(),
          },
        ]),
      ),
    }),
    { encoding: "ascii", flag: "wx", mode: 0o600 },
  );
}

function expectOrigin(value: ObservedOrigins, expected: string): void {
  expect([...value.pageOrigins].sort()).toEqual([expected]);
  expect([...value.requestOrigins].sort()).toEqual([expected]);
}

async function createQuestion(page: Page, title: string, answer: string): Promise<string> {
  await page.getByRole("link", { name: "Workspace" }).click();
  await page.getByRole("button", { name: "Create flat question" }).click();
  await page.getByLabel("Question title").fill(title);
  await page
    .getByLabel("Learner-facing prompt")
    .fill(`Choose the supported recovery response for ${title}`);
  await page.getByLabel("Choice text").nth(0).fill(answer);
  await page.getByLabel("Choice text").nth(1).fill(`Alternative recovery response for ${title}`);
  await page
    .getByRole("radio", { name: new RegExp(`Mark choice 1 as correct: ${answer}`) })
    .check();
  await page.getByRole("button", { name: "Save private draft" }).click();
  await page.getByRole("button", { name: "Review publication changes" }).click();
  await page.getByLabel("Reviewed public byline").fill("Dr. Elena Rivera");
  await page.getByRole("button", { name: "Confirm and publish" }).click();
  await expect(page.getByRole("heading", { name: "Published" })).toBeVisible();
  await page.getByRole("link", { name: "Library", exact: true }).click();
  await page.getByLabel("Search published questions").fill(title);
  const card = page
    .getByRole("region", { name: "Published questions" })
    .getByText(title)
    .locator("..");
  await expect(card).toBeVisible();
  const identifier = await card.locator("code").innerText();
  expect(identifier).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);
  return identifier;
}

async function createCourseAssignment(
  page: Page,
  course: string,
  assignment: string,
  question: string,
  namespace: string,
): Promise<string> {
  await page.getByRole("link", { name: "Courses" }).click();
  await page.getByLabel("Course title").fill(course);
  await page.getByLabel("Start date").fill(isoDate(-30));
  await page.getByLabel("End date").fill(isoDate(365));
  await page.getByLabel("Time zone (IANA)").fill("America/Chicago");
  await page.getByRole("button", { name: "Create course" }).click();
  const courseCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: course, exact: true }) });
  await expect(courseCard).toHaveCount(1);
  await courseCard.getByRole("link", { name: "Open course", exact: true }).click();
  await page.getByRole("link", { name: "Assignments" }).click();
  await page.getByRole("link", { name: "Create the first assignment" }).click();
  await page.getByLabel("Assignment title").fill(assignment);
  await page.getByText("Add several Question IDs", { exact: true }).click();
  await page.getByLabel("Question IDs").fill(question);
  await page.getByRole("button", { name: "Add questions by ID" }).click();
  await page.getByRole("button", { name: "Create assignment" }).click();
  await page.getByRole("link", { name: `Open ${assignment}` }).click();
  await page.getByLabel("Lifecycle").selectOption("published");
  await page.getByRole("button", { name: "Save teaching operations" }).click();
  await expect(page.getByTestId("assignment-current-state")).toHaveText("Published, open now.");
  await page.getByRole("link", { name: "Students" }).click();
  await page.getByLabel("Institutional email").fill(maryEmail);
  await page.getByLabel("Institutional student ID").fill(`mary-${namespace}`);
  await page.getByRole("button", { name: "Create invitation" }).click();
  const link = page.getByLabel("Invitation link");
  await expect(link).toBeVisible();
  return link.inputValue();
}

async function startRun(
  page: Page,
  invitation: string,
  course: string,
  assignment: string,
  answer: string,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await page.goto(invitation);
  await page.getByRole("button", { name: "Claim this course" }).click();
  await expect(page.getByRole("heading", { name: course, exact: true })).toBeVisible();
  const card = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignment, exact: true }) });
  await card.getByRole("link", { name: "Start assignment", exact: true }).click();
  await page.getByRole("button", { name: "Start or continue practice" }).click();
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible();
  await page.getByRole("radio", { name: answer, exact: true }).check();
}

async function observeFreshScore(page: Page, course: string, assignment: string): Promise<Locator> {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await selectVisibleCourse(page, course);
  const card = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignment, exact: true }) });
  await card.getByRole("link", { name: "Start assignment", exact: true }).click();
  const facts = page.locator(".assignment-facts");
  const scoreStatus = facts.getByRole("status");
  await expect(scoreStatus).toHaveText("Score available: Current 100%, Latest 100%, Best 100%.");
  return scoreStatus;
}

test.describe.configure({ mode: "serial" });

test("learner gateway recovery: a saved response retries after the owner restores the real gateway", async ({
  browser,
}) => {
  test.setTimeout(timeoutMs);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("learner_gateway_recovery");
  expect(scenarioInput.faultTransition).toBe("gateway_submit_outage");
  const handshake = await faultHandshakeFromEnvironment(
    process.env,
    scenarioInput.scenarioId,
    scenarioInput.namespace,
  );
  const course = `Gateway recovery course ${scenarioInput.namespace}`;
  const assignment = `Gateway recovery assignment ${scenarioInput.namespace}`;
  const answer = `Supported gateway recovery choice ${scenarioInput.namespace}`;
  const origins = {
    instructor: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    learner: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    fresh_learner: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
  };
  const expected = new URL(scenarioInput.baseUrl).origin;
  const contexts: BrowserContext[] = [];
  let originEvidence = false;

  try {
    const instructorContext = await browser.newContext(contextOptions);
    const learnerContext = await browser.newContext(contextOptions);
    contexts.push(instructorContext, learnerContext);
    observeContextOrigins(
      instructorContext,
      origins.instructor.pageOrigins,
      origins.instructor.requestOrigins,
    );
    observeContextOrigins(
      learnerContext,
      origins.learner.pageOrigins,
      origins.learner.requestOrigins,
    );
    const instructor = await instructorContext.newPage();
    const learner = await learnerContext.newPage();
    configure(instructorContext, instructor);
    configure(learnerContext, learner);
    await chooseSeededIdentity(instructor, /Elena Rivera/u);
    await selectVisibleCourse(instructor, "Biochemistry Base Course");
    const question = await createQuestion(
      instructor,
      `Gateway recovery question ${scenarioInput.namespace}`,
      answer,
    );
    const invitation = await createCourseAssignment(
      instructor,
      course,
      assignment,
      question,
      scenarioInput.namespace,
    );
    await startRun(learner, invitation, course, assignment, answer);
    handshake.notify("response_selected");
    await handshake.waitFor("gateway_stopped");
    await learner.getByRole("button", { name: "Submit answer" }).click();
    const selectedResponse = learner.getByRole("radio", { name: answer, exact: true });
    await expect(selectedResponse).toBeChecked();
    const retry = learner.getByRole("button", { name: "Retry saved response" });
    await expect(retry).toBeVisible();
    await retry.scrollIntoViewIfNeeded();
    await captureRealStackScreenshot(learner, scenarioInput, "learner_gateway_retry");
    handshake.notify("network_recovery_visible");
    await handshake.waitFor("gateway_recovered");
    await retry.focus();
    await expect(retry).toBeFocused();
    await learner.keyboard.press("Enter");
    await expect(learner.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible();
    await expect(learner.getByRole("heading", { name: "Correct", exact: true })).toBeVisible();
    await learner.getByRole("heading", { name: "Feedback", exact: true }).scrollIntoViewIfNeeded();
    await captureRealStackScreenshot(learner, scenarioInput, "learner_gateway_recovered_feedback");
    await learner.getByRole("button", { name: "View completed run", exact: true }).click();
    const completion = learner.locator(".attempt-summary");
    await expect(completion.getByText("Your completed run is recorded.")).toBeVisible();
    await completion.scrollIntoViewIfNeeded();
    await captureRealStackScreenshot(
      learner,
      scenarioInput,
      "learner_gateway_recovered_completion",
    );
    await signOutVisible(learner);
    await learnerContext.close();
    contexts.splice(contexts.indexOf(learnerContext), 1);
    const freshContext = await browser.newContext(contextOptions);
    contexts.push(freshContext);
    observeContextOrigins(
      freshContext,
      origins.fresh_learner.pageOrigins,
      origins.fresh_learner.requestOrigins,
    );
    expect(await freshContext.storageState()).toEqual({ cookies: [], origins: [] });
    const fresh = await freshContext.newPage();
    configure(freshContext, fresh);
    const freshScore = await observeFreshScore(fresh, course, assignment);
    await freshScore.scrollIntoViewIfNeeded();
    await captureRealStackScreenshot(fresh, scenarioInput, "learner_gateway_fresh_session_score");
    expectOrigin(origins.instructor, expected);
    expectOrigin(origins.learner, expected);
    expectOrigin(origins.fresh_learner, expected);
    originEvidence = true;
    handshake.notify("completed");
  } finally {
    handshake.close();
    await Promise.all(contexts.map(async (context) => context.close()));
    if (originEvidence) writeOriginReceipt(origins);
  }
});
