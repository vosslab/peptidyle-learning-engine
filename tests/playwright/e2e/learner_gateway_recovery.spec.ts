// A real learner retry remains visible and durable while the owner recovers the gateway.
//
// Selector contract:
// - src/features/flat_question_authoring/flat_question_editor_page.tsx:535 owns question creation
//   fields and publication controls used to seed the recovery journey.
// - src/pages/course_list_page.tsx, course_assignments_page.tsx, assignment_workspace/, and
//   course_roster_page.tsx own course, title-first assignment setup, and invitation controls.
// - src/pages/course_invitation_page.tsx:62 and src/pages/assignment_overview_page.tsx:114 own
//   learner claiming and the Start assignment control.
// - src/pages/run_page.tsx:442 and src/components/responses/common.tsx:294 own the attempt surface
//   and visible response controls.
import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { waitForAutomatedFeedback } from "./automated_grading_ui";
import { faultHandshakeFromEnvironment } from "./fault_handshake";
import { BIOCHEMISTRY_COURSE_TITLE } from "./helper_course_titles";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  expectObservedOrigin,
  observeContextOrigins,
  relativeIsoDate,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
  writeContextOriginReceipt,
} from "./real_stack_ui";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";

const maryEmail = "mary.okafor@live-demo.ple.example";
const timeoutMs = 600_000;
const actionTimeoutMs = 30_000;
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };

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
): Promise<string> {
  await page.getByRole("link", { name: "Courses" }).click();
  await page.getByLabel("Course title").fill(course);
  await page.getByLabel("Start date").fill(relativeIsoDate(-30));
  await page.getByLabel("End date").fill(relativeIsoDate(365));
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
  await page.getByRole("button", { name: "Create assignment draft", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
  await page.getByLabel("Question IDs").fill(question);
  await page.getByRole("button", { name: "Add Question IDs", exact: true }).click();
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: "Added 1 Question ID. Save questions and order when ready." }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Save questions and order", exact: true }).click();
  await page.getByRole("link", { name: "Policies", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Policies", exact: true })).toBeVisible();
  await page.getByLabel("Lifecycle").selectOption("published");
  await page.getByRole("button", { name: "Save assignment policies", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Assignment policies saved." }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Students" }).click();
  await page.getByLabel("Institutional email").fill(maryEmail);
  await page.getByLabel("Institutional student ID").fill("BIO-MARY-005");
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
  const course = "Biochemistry: Resilient Practice";
  const assignment = "Peptide Bonds: Connection Recovery";
  const answer = "Resonance restricts peptide-bond rotation";
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
    configureContextAndPage(instructorContext, instructor, actionTimeoutMs);
    configureContextAndPage(learnerContext, learner, actionTimeoutMs);
    await chooseSeededIdentity(instructor, /Elena Rivera/u);
    await selectVisibleCourse(instructor, BIOCHEMISTRY_COURSE_TITLE);
    const question = await createQuestion(
      instructor,
      "Peptide Bond Resonance During Recovery",
      answer,
    );
    const invitation = await createCourseAssignment(instructor, course, assignment, question);
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
    const feedback = await waitForAutomatedFeedback(learner);
    await expect(feedback.getByRole("heading", { name: "Correct", exact: true })).toBeVisible();
    await feedback.scrollIntoViewIfNeeded();
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
    configureContextAndPage(freshContext, fresh, actionTimeoutMs);
    const freshScore = await observeFreshScore(fresh, course, assignment);
    await freshScore.scrollIntoViewIfNeeded();
    await captureRealStackScreenshot(fresh, scenarioInput, "learner_gateway_fresh_session_score");
    expectObservedOrigin(origins.instructor, expected);
    expectObservedOrigin(origins.learner, expected);
    expectObservedOrigin(origins.fresh_learner, expected);
    originEvidence = true;
    handshake.notify("completed");
  } finally {
    handshake.close();
    await Promise.all(contexts.map(async (context) => context.close()));
    if (originEvidence) writeContextOriginReceipt(origins);
  }
});
