// Connected student delivery proof. Product state is created through the visible production UI.
//
// Selector contract:
// - src/pages/course_assignments_page.tsx:324 owns the assignments surface and assignment links.
// - src/features/ple_question_json_authoring/question_json_editor_page.tsx:535 owns question creation
//   fields and publication controls used to seed the journey.
// - src/pages/course_list_page.tsx, src/pages/assignment_workspace/, and
//   src/pages/course_roster_page.tsx own course, assignment, and invitation controls.
// - src/pages/course_invitation_page.tsx:62 and src/pages/assignment_overview_page.tsx:114 own
//   student claiming and assignment entry; data-route-surface is defined at
//   course_assignments_page.tsx:324.
// - src/pages/gradebook_page.tsx:151 owns the calculated assignment cell and inspect link;
//   src/pages/student_work_inspection_page.tsx:346 owns the audited detail and return focus route.
import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { waitForAutomatedStudentFeedback } from "./automated_grading_ui";
import { BIOCHEMISTRY_COURSE_TITLE } from "./helper_course_titles";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  relativeIsoDate,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
  startOrContinuePractice,
  writeOriginReceipt,
} from "./real_stack_ui";

const maryEmail = "mary.okafor@live-demo.ple.example";
const journeyTimeoutMs = 600_000;
const actionTimeoutMs = 30_000;
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };

function configureContext(context: BrowserContext): void {
  context.setDefaultTimeout(actionTimeoutMs);
  context.setDefaultNavigationTimeout(actionTimeoutMs);
}

function configurePage(page: Page): void {
  page.setDefaultTimeout(actionTimeoutMs);
  page.setDefaultNavigationTimeout(actionTimeoutMs);
}

async function openCourseAssignments(page: Page): Promise<void> {
  await page.getByRole("link", { name: "Assignments", exact: true }).click();
  await expect(page.locator("[data-route-surface=courseAssignments]")).toBeVisible();
}

function assignmentArticle(page: Page, assignmentTitle: string): Locator {
  return page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
}

async function restoreViewportOrigin(page: Page): Promise<void> {
  await page.evaluate(() => window.scrollTo(0, 0));
  await expect
    .poll(() => page.evaluate(() => ({ x: window.scrollX, y: window.scrollY })))
    .toEqual({ x: 0, y: 0 });
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          document.documentElement.scrollWidth <= window.innerWidth &&
          document.body.scrollWidth <= window.innerWidth,
      ),
    )
    .toBe(true);
}

async function createPublishedQuestion(
  page: Page,
  questionTitle: string,
  correctChoice: string,
): Promise<string> {
  await page.getByRole("link", { name: "Workspace" }).click();
  await page.getByRole("button", { name: "Create Question" }).click();
  await page.getByLabel("Question Title").fill(questionTitle);
  await page
    .getByLabel("Student-facing prompt")
    .fill(`Choose the supported statement: ${questionTitle}`);
  await page.getByLabel("Choice text").nth(0).fill(correctChoice);
  await page
    .getByLabel("Choice text")
    .nth(1)
    .fill("Peptide bonds rotate freely because they are ordinary single bonds");
  await page
    .getByRole("radio", { name: new RegExp(`Mark choice 1 as correct: ${correctChoice}`) })
    .check();
  await page.getByRole("button", { name: "Save private draft" }).click();
  await page.getByRole("button", { name: "Review publication changes" }).click();
  await page.getByLabel("Question Authors").fill("Dr. Elena Rivera");
  await page.getByRole("button", { name: "Confirm and publish" }).click();
  await expect(page.getByRole("heading", { name: "Published" })).toBeVisible();

  await page.getByRole("link", { name: "Question Library", exact: true }).click();
  await page.getByLabel("Search published questions").fill(questionTitle);
  const questionCard = page
    .getByRole("region", { name: "Published questions" })
    .getByText(questionTitle)
    .locator("..");
  await expect(questionCard).toBeVisible();
  const questionId = await questionCard.locator("code").innerText();
  expect(questionId).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);
  return questionId;
}

async function createReleasedCourseAssignment(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
  questionId: string,
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

  await openCourseAssignments(page);
  await page.getByRole("link", { name: "Create the first assignment" }).click();
  await expect(page.getByRole("heading", { name: "Create an Assignment" })).toBeVisible();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
  await page.getByLabel("Question IDs").fill(questionId);
  await page.getByRole("button", { name: "Check Question ID", exact: true }).click();
  await expect(page.getByText(new RegExp(`${questionId} is ready to add`))).toBeVisible();
  await page.getByRole("button", { name: "Add Question IDs", exact: true }).click();
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: "Added 1 Question ID. Save questions and order when ready." }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Save questions and order", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Questions and order saved." }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Policies", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Policies", exact: true })).toBeVisible();
  await page.getByLabel("Lifecycle").selectOption("released");
  await page.getByRole("button", { name: "Save assignment policies", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Assignment policies saved." }),
  ).toBeVisible();

  await page.getByRole("link", { name: "Students" }).click();
  await page.getByLabel("Course roster email").fill(maryEmail);
  await page.getByLabel("Course roster ID").fill("BIO-MARY-002");
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
  const assignmentCard = assignmentArticle(page, assignmentTitle);
  await expect(assignmentCard).toHaveCount(1);
  await expect(assignmentCard.locator(".course-card-description")).toHaveText(
    "Open this assignment to review its instructions and delivery details.",
  );
  await expect(assignmentCard.locator(".course-card-progress")).toHaveText(
    "No score yet. Submit a response to record scored progress.",
  );
  await expect
    .poll(() =>
      assignmentCard.evaluate((card) => {
        const description = card.querySelector(".course-card-description");
        const progress = card.querySelector(".course-card-progress");
        if (!(description instanceof HTMLElement) || !(progress instanceof HTMLElement)) {
          return null;
        }
        return {
          description: getComputedStyle(description).gridArea,
          progress: getComputedStyle(progress).gridArea,
        };
      }),
    )
    .toEqual({ description: "summary", progress: "progress" });
  await assignmentCard.getByRole("link", { name: "Start assignment", exact: true }).click();
  await expect(page.locator("[data-route-surface=assignmentOverview]")).toBeVisible();
  await startOrContinuePractice(page);
  const selectedResponse = page.getByRole("radio", { name: correctChoice, exact: true });
  await selectedResponse.focus();
  await page.keyboard.press("Space");
  await expect(selectedResponse).toBeChecked();
  const submitAnswer = page.getByRole("button", { name: "Submit answer" });
  await submitAnswer.focus();
  await page.keyboard.press("Enter");
  const feedback = await waitForAutomatedStudentFeedback(page);
  await expect(feedback).toBeVisible();
  await expect(feedback.getByRole("heading", { name: "Correct", exact: true })).toBeVisible();
  await expect(feedback.getByRole("heading", { name: "Your response", exact: true })).toBeVisible();
  await expect(
    feedback.getByRole("heading", { name: "Correct Feedback", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "View completed Assignment Attempt", exact: true })
    .click();
  const summary = page.locator(".attempt-summary");
  await expect(summary.getByText("Your completed Assignment Attempt is recorded.")).toBeVisible();
  const assignmentScore = summary.getByRole("region", { name: "Assignment score" });
  await expect(assignmentScore).toContainText(
    "Score available: Current 100%, Latest 100%, Best 100%.",
  );
  await expect(assignmentScore).toContainText("This Assignment Attempt: 100%");
  await summary.getByRole("button", { name: "Start fresh practice" }).click();
  await expect(page.locator("[data-route-surface=assignmentAttempt]")).toBeVisible();
  const assignmentAttemptHeader = page.locator(".assignment-attempt-header");
  await expect(assignmentAttemptHeader).toBeVisible();
  await expect(
    assignmentAttemptHeader.getByText("Practice Assignment Attempt 2", { exact: true }),
  ).toBeVisible();
  await expect(assignmentAttemptHeader.getByRole("heading")).toBeVisible();
  await expect(page.locator("header.site-header")).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Primary navigation" })).toBeVisible();
  await restoreViewportOrigin(page);
  await expect(page.getByRole("radio", { name: correctChoice, exact: true })).not.toBeChecked();
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
  const scoreStatus = facts
    .getByText("Score status", { exact: true })
    .locator("..")
    .getByRole("status");
  await expect(scoreStatus).toHaveText("Score available: Current 100%, Latest 100%, Best 100%.");
  await expect(facts.getByText("Completed Assignment Attempts", { exact: true })).toBeVisible();
  const completedAssignmentAttempts = facts
    .getByText("Completed Assignment Attempts", { exact: true })
    .locator("..")
    .locator("dd");
  await expect(completedAssignmentAttempts).toHaveText(/[1-9]\d*/u);
}

async function observeInstructorOutcomesAndAccess(
  page: Page,
  assignmentTitle: string,
  submittedResponse: string,
): Promise<void> {
  // Leave the invitation-era roster instance so the visible return performs a
  // fresh server read after Mary's claim and completed Assignment Attempt.
  await openCourseAssignments(page);
  await page.getByRole("link", { name: "Students" }).click();
  await expect(page.locator("[data-route-surface=courseRoster]")).toBeVisible();
  const activeLearner = page.getByRole("row", { name: /Mary Okafor/u });
  await expect(activeLearner).toBeVisible();

  await page.getByRole("link", { name: "Gradebook" }).click();
  await expect(page.locator("[data-route-surface=gradebook]")).toBeVisible();
  // ASVS 8.2.2 and 8.3.1: verify the server-authorized Gradebook and Student Work
  // Inspection for the exact student work created through Mary's separate authenticated session.
  const learnerScore = page
    .locator("tr.gradebook-row")
    .filter({ has: page.getByText("Mary Okafor", { exact: true }) });
  await expect(learnerScore).toHaveCount(1);
  await expect(learnerScore.locator(".gradebook-course-total")).toContainText("100%");
  const assignmentCell = learnerScore.locator(`[data-label="${assignmentTitle}"]`);
  await expect(assignmentCell).toContainText("100%");
  const inspectSubmittedWork = assignmentCell.getByRole("link", {
    name: "Inspect submitted work",
    exact: true,
  });
  await expect(inspectSubmittedWork).toHaveCount(1);

  await inspectSubmittedWork.click();
  const inspectedWork = page.locator("[data-route-surface=studentWorkInspection]");
  await expect(inspectedWork).toBeVisible();
  await expect(
    inspectedWork.getByRole("heading", { name: assignmentTitle, exact: true }),
  ).toBeVisible();
  await expect(inspectedWork.locator(".page-lede")).toContainText("Mary Okafor");
  await expect(inspectedWork.locator(".page-lede")).toContainText(
    /submitted Assignment Attempt R-[1-9]\d*/u,
  );
  await expect(inspectedWork.getByRole("region", { name: "Student response" })).toContainText(
    submittedResponse,
  );
  const automatedGrading = inspectedWork.getByRole("region", {
    name: "Automated grading result",
  });
  await expect(automatedGrading).toContainText("Correct");
  await expect(automatedGrading).toContainText("Scoring generation");
  const immutableEvidence = inspectedWork.getByText("Immutable evidence", { exact: true });
  await expect(immutableEvidence).toBeVisible();
  await immutableEvidence.click();
  await expect(inspectedWork.getByText("Presentation SHA-256", { exact: true })).toBeVisible();

  await page.getByRole("link", { name: "Back to Gradebook", exact: true }).click();
  await expect(page.locator("[data-route-surface=gradebook]")).toBeVisible();
  await expect(page).toHaveURL(/#gradebook-cell-M-[1-9]\d*-A-[1-9]\d*$/u);
  await expect(assignmentCell).toBeFocused();

  await openCourseAssignments(page);
  const workspaceCard = assignmentArticle(page, assignmentTitle);
  await expect(workspaceCard).toHaveCount(1);
  await workspaceCard.getByRole("link", { name: assignmentTitle, exact: true }).click();
  await expect(page.locator("[data-route-surface=assignmentWorkspace]")).toBeVisible();
  await page.getByRole("link", { name: "Policies", exact: true }).click();
  await page.getByLabel("Lifecycle").selectOption("archived");
  await page.getByRole("button", { name: "Save assignment policies", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Assignment policies saved." }),
  ).toBeVisible();
}

test.describe.configure({ mode: "serial" });

test("student delivery: Mary completes and revisits an instructor-created assignment", async ({
  browser,
}) => {
  test.setTimeout(journeyTimeoutMs);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("learner_delivery");
  const questionTitle = "Peptide Bond Planarity";
  const correctChoice = "Resonance restricts rotation around the peptide bond";
  const courseTitle = "Biochemistry: Molecular Structure Practice";
  const assignmentTitle = "Peptide Bonds: Guided Practice";
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
    await selectVisibleCourse(elena, BIOCHEMISTRY_COURSE_TITLE);
    const questionId = await createPublishedQuestion(elena, questionTitle, correctChoice);
    const invitationUrl = await createReleasedCourseAssignment(
      elena,
      courseTitle,
      assignmentTitle,
      questionId,
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
    await signOutVisible(freshMary);
    await freshMaryContext.close();
    contexts.splice(contexts.indexOf(freshMaryContext), 1);

    await observeInstructorOutcomesAndAccess(elena, assignmentTitle, correctChoice);

    expect(pageOrigins.size).toBeGreaterThan(0);
    expect(requestOrigins.size).toBeGreaterThan(0);
    expect([...pageOrigins]).toEqual([expectedOrigin]);
    expect([...requestOrigins]).toEqual([expectedOrigin]);
    writeOriginReceipt(pageOrigins, requestOrigins);
  } finally {
    await Promise.all(contexts.map(async (context) => context.close()));
  }
});
