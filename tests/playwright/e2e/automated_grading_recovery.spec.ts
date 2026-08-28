// Production browser journey for one deterministic automated-grading exception.
//
// Selector contract:
// - src/pages/run_page.tsx owns the learner pending state and status refresh.
// - src/pages/assignment_workspace/assignment_workspace_operations_page.tsx owns the
//   Instructor operation list and row-local retry.
// - src/pages/gradebook_page.tsx owns the course-grade projection.
import { expect, test, type BrowserContext, type Page, type Request } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
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
  writeContextOriginReceipt,
} from "./real_stack_ui";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";
import {
  AUTOMATED_GRADING_RECOVERY_LABELS,
  answerFreeViolation,
  automatedGradingRetryName,
  isInstructorRetryPost,
  isLearnerStatusGet,
  isLearnerSubmissionPost,
} from "./automated_grading_recovery_ui";

const maryEmail = "mary.okafor@live-demo.ple.example";
const actionTimeoutMs = 30_000;
const journeyTimeoutMs = 600_000;
const pollingIntervalMs = 2_000;
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };
const journeyName =
  "automated grading recovery: Elena visibly resolves one deterministic grader exception";
const setupStep = "Elena creates the live assessment and Mary selects an ordinary answer";
const statusStep =
  "Mary refreshes only grading status until the deterministic exception is visible";
const retryStep = "Elena opens the answer-free operation and requests exactly one retry";
const gradebookStep =
  "Elena sees the real current gradebook total after the ordinary worker completes";

async function createQuestion(page: Page, title: string, correctChoice: string): Promise<string> {
  await page.getByRole("link", { name: "Workspace" }).click();
  await page.getByRole("button", { name: "Create flat question" }).click();
  await page.getByLabel("Question title").fill(title);
  await page.getByLabel("Learner-facing prompt").fill("Choose the supported response for " + title);
  await page.getByLabel("Choice text").nth(0).fill(correctChoice);
  await page
    .getByLabel("Choice text")
    .nth(1)
    .fill("Alternative response for " + title);
  await page
    .getByRole("radio", { name: new RegExp("Mark choice 1 as correct: " + correctChoice) })
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

async function createAssignmentWithInvitation(
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
  await page.getByRole("link", { name: "Assignments", exact: true }).click();
  await page.getByRole("link", { name: "Create the first assignment", exact: true }).click();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByRole("button", { name: "Create assignment draft", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
  await page.getByLabel("Question IDs").fill(questionId);
  await page.getByRole("button", { name: "Add Question IDs", exact: true }).click();
  await page.getByRole("button", { name: "Save questions and order", exact: true }).click();
  await page.getByRole("link", { name: "Policies", exact: true }).click();
  await page.getByLabel("Lifecycle").selectOption("published");
  await page.getByRole("button", { name: "Save assignment policies", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Assignment policies saved." }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Students", exact: true }).click();
  await page.getByLabel("Institutional email").fill(maryEmail);
  await page.getByLabel("Institutional student ID").fill("BIO-MARY-007");
  await page.getByRole("button", { name: "Create invitation" }).click();
  const invitation = page.getByLabel("Invitation link");
  await expect(invitation).toBeVisible();
  return invitation.inputValue();
}

async function startVisibleAttempt(
  page: Page,
  invitation: string,
  courseTitle: string,
  assignmentTitle: string,
  correctChoice: string,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await page.goto(invitation);
  await page.getByRole("button", { name: "Claim this course" }).click();
  await expect(page.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
  const assignment = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignment.getByRole("link", { name: "Start assignment", exact: true }).click();
  await page.getByRole("button", { name: "Start or continue practice" }).click();
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible();
  await page.getByRole("radio", { name: correctChoice, exact: true }).check();
}

interface RecoveryNetworkEvidence {
  learnerSubmissionBodies: string[];
  learnerSubmissionResponses: unknown[];
  learnerSubmissionStatuses: number[];
  learnerStatusResponses: unknown[];
  learnerStatusStatuses: number[];
  retryBodies: string[];
  retryResponses: unknown[];
  retryStatuses: number[];
  pendingResponses: Promise<void>[];
}

function trackRecoveryNetworkTraffic(learner: Page, instructor: Page): RecoveryNetworkEvidence {
  const evidence: RecoveryNetworkEvidence = {
    learnerSubmissionBodies: [],
    learnerSubmissionResponses: [],
    learnerSubmissionStatuses: [],
    learnerStatusResponses: [],
    learnerStatusStatuses: [],
    retryBodies: [],
    retryResponses: [],
    retryStatuses: [],
    pendingResponses: [],
  };

  learner.on("request", (request: Request) => {
    const pathname = new URL(request.url()).pathname;
    if (isLearnerSubmissionPost(request.method(), pathname)) {
      evidence.learnerSubmissionBodies.push(request.postData() ?? "");
    }
  });
  learner.on("response", (response) => {
    const pathname = new URL(response.url()).pathname;
    if (isLearnerSubmissionPost("POST", pathname)) {
      evidence.learnerSubmissionStatuses.push(response.status());
      evidence.pendingResponses.push(
        response.text().then((body) => evidence.learnerSubmissionResponses.push(JSON.parse(body))),
      );
    }
    if (isLearnerStatusGet("GET", pathname)) {
      evidence.learnerStatusStatuses.push(response.status());
      evidence.pendingResponses.push(
        response.text().then((body) => evidence.learnerStatusResponses.push(JSON.parse(body))),
      );
    }
  });
  instructor.on("request", (request: Request) => {
    const pathname = new URL(request.url()).pathname;
    if (isInstructorRetryPost(request.method(), pathname)) {
      evidence.retryBodies.push(request.postData() ?? "");
    }
  });
  instructor.on("response", (response) => {
    const pathname = new URL(response.url()).pathname;
    if (!isInstructorRetryPost("POST", pathname)) return;
    evidence.retryStatuses.push(response.status());
    evidence.pendingResponses.push(
      response.text().then((body) => evidence.retryResponses.push(JSON.parse(body))),
    );
  });
  return evidence;
}

async function assertRecoveryNetworkEvidence(
  evidence: RecoveryNetworkEvidence,
  privateValues: ReadonlyArray<string>,
): Promise<void> {
  await Promise.all(evidence.pendingResponses);
  expect(evidence.learnerSubmissionBodies).toHaveLength(1);
  expect(evidence.learnerSubmissionStatuses).toEqual([202]);
  expect(evidence.learnerSubmissionResponses).toHaveLength(1);
  expect(evidence.learnerStatusStatuses.length).toBeGreaterThan(0);
  expect(evidence.learnerStatusStatuses.every((status) => status === 202)).toBe(true);
  expect(evidence.learnerStatusResponses).toHaveLength(evidence.learnerStatusStatuses.length);
  expect(evidence.retryBodies).toEqual([""]);
  expect(evidence.retryStatuses).toEqual([200]);
  expect(evidence.retryResponses).toHaveLength(1);

  const responses = [
    ...evidence.learnerSubmissionResponses,
    ...evidence.learnerStatusResponses,
    ...evidence.retryResponses,
  ];
  for (const response of responses) {
    expect(answerFreeViolation(response, privateValues)).toBeNull();
  }
}

async function openInstructorOperations(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
): Promise<void> {
  await selectVisibleCourse(page, courseTitle);
  await page.getByRole("link", { name: "Assignments", exact: true }).click();
  const assignment = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignment.getByRole("link", { name: assignmentTitle, exact: true }).click();
  await page
    .getByRole("link", { name: AUTOMATED_GRADING_RECOVERY_LABELS.gradingOperations })
    .click();
  await expect(
    page.getByRole("heading", { name: AUTOMATED_GRADING_RECOVERY_LABELS.gradingOperations }),
  ).toBeVisible();
}

test.describe.configure({ mode: "serial" });

test(journeyName, async ({ browser }) => {
  test.setTimeout(journeyTimeoutMs);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("automated_grading_recovery");
  expect(scenarioInput.faultTransition).toBe("deterministic_grader_exception");
  const handshake = await faultHandshakeFromEnvironment(
    process.env,
    scenarioInput.scenarioId,
    scenarioInput.namespace,
    "deterministic_grader_exception",
  );
  const courseTitle = "Biochemistry: Automated Recovery";
  const assignmentTitle = "Peptide Bonds: Deterministic Recovery";
  const correctChoice = "Resonance restricts peptide-bond rotation";
  const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
  const origins = {
    instructor: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    learner: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
  };
  const contexts: BrowserContext[] = [];
  let originEvidenceVerified = false;

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

    await test.step(setupStep, async () => {
      await chooseSeededIdentity(instructor, /Elena Rivera/u);
      await selectVisibleCourse(instructor, BIOCHEMISTRY_COURSE_TITLE);
      const questionId = await createQuestion(
        instructor,
        "Peptide Bond Resonance During Automated Recovery",
        correctChoice,
      );
      const invitation = await createAssignmentWithInvitation(
        instructor,
        courseTitle,
        assignmentTitle,
        questionId,
      );
      await startVisibleAttempt(learner, invitation, courseTitle, assignmentTitle, correctChoice);
    });

    const traffic = trackRecoveryNetworkTraffic(learner, instructor);
    handshake.notify("submission_ready");
    await handshake.waitFor("ordinary_worker_stopped");

    await test.step("Mary sees accepted-pending with the local answer buffer cleared", async () => {
      await learner.getByRole("button", { name: "Submit answer", exact: true }).click();
      const pending = learner
        .getByRole("heading", { name: AUTOMATED_GRADING_RECOVERY_LABELS.responseReceived })
        .locator("..");
      await expect(pending).toBeVisible();
      await expect(
        pending.getByRole("button", { name: AUTOMATED_GRADING_RECOVERY_LABELS.checkGradingStatus }),
      ).toBeVisible();
      await expect(learner.getByRole("radio", { name: correctChoice, exact: true })).toHaveCount(0);
      await expect(learner.getByRole("button", { name: "Submit answer", exact: true })).toHaveCount(
        0,
      );
      expect(traffic.learnerSubmissionBodies).toHaveLength(1);
      handshake.notify("accepted_pending_visible");
      await handshake.waitFor("fault_worker_started");
    });

    await test.step(statusStep, async () => {
      const statusButton = learner.getByRole("button", {
        name: AUTOMATED_GRADING_RECOVERY_LABELS.checkGradingStatus,
      });
      await expect
        .poll(
          async () => {
            if (await statusButton.isVisible()) await statusButton.click();
            return learner.getByText("Your response needs instructor attention.").isVisible();
          },
          { timeout: journeyTimeoutMs / 2, intervals: [pollingIntervalMs] },
        )
        .toBe(true);
      expect(traffic.learnerSubmissionBodies).toHaveLength(1);
      expect(traffic.learnerStatusStatuses.length).toBeGreaterThan(0);
      handshake.notify("fault_worker_exception_visible");
    });

    await test.step(retryStep, async () => {
      await openInstructorOperations(instructor, courseTitle, assignmentTitle);
      const retry = instructor.getByRole("button", { name: automatedGradingRetryName });
      await expect(retry).toHaveCount(1);
      await expect(
        instructor.getByText("Automatic grading stopped before it could finish."),
      ).toBeVisible();
      await captureRealStackScreenshot(
        instructor,
        scenarioInput,
        "automated_grading_recovery_operation_laptop",
      );
      await retry.click();
      await expect(
        instructor.getByRole("status").filter({ hasText: "The grading retry was accepted." }),
      ).toBeVisible();
      handshake.notify("instructor_retry_visible");
      await handshake.waitFor("ordinary_worker_recovered");
    });

    await test.step(gradebookStep, async () => {
      await instructor
        .getByRole("link", { name: AUTOMATED_GRADING_RECOVERY_LABELS.gradebook })
        .click();
      const records = instructor.getByRole("region", { name: "Gradebook records" });
      await expect
        .poll(
          async () => {
            const row = instructor
              .getByRole("row")
              .filter({ hasText: assignmentTitle })
              .filter({ hasText: "Mary Okafor" });
            if (
              (await row.isVisible()) &&
              /100%/u.test((await row.innerText()).replace(/\s+/gu, " "))
            ) {
              return true;
            }
            await instructor.getByRole("link", { name: "Assignments", exact: true }).click();
            await instructor
              .getByRole("link", { name: AUTOMATED_GRADING_RECOVERY_LABELS.gradebook })
              .click();
            return false;
          },
          { timeout: journeyTimeoutMs / 2, intervals: [pollingIntervalMs] },
        )
        .toBe(true);
      await records.scrollIntoViewIfNeeded();
      await captureRealStackScreenshot(
        instructor,
        scenarioInput,
        "automated_grading_recovery_gradebook_laptop",
      );
    });

    await test.step("the live network proves one learner answer, one empty retry, and answer-free JSON", async () => {
      await assertRecoveryNetworkEvidence(traffic, [
        correctChoice,
        `Alternative response for Peptide Bond Resonance During Automated Recovery`,
      ]);
    });

    expectObservedOrigin(origins.instructor, expectedOrigin);
    expectObservedOrigin(origins.learner, expectedOrigin);
    originEvidenceVerified = true;
    handshake.notify("completed");
  } finally {
    handshake.close();
    await Promise.all(contexts.map(async (context) => context.close()));
    if (originEvidenceVerified) writeContextOriginReceipt(origins);
  }
});
