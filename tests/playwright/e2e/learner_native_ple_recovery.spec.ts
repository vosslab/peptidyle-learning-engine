// One submitted Assignment Attempt remains durable while the owner recovers the native PLE worker.
//
// Selector contract:
// - src/features/ple_question_json_authoring/question_json_editor_page.tsx:535 owns question creation
//   fields and publication controls used to seed the recovery journey.
// - src/pages/course_list_page.tsx, course_instance_page.tsx, course_roster_page.tsx, and
//   assignment_workspace_create_page.tsx and assignment_workspace pages own visible Course, roster,
//   focused Assignment creation, Questions, and delivery-policy setup.
// - src/pages/student_courses_page.tsx:27-33, student_course_invitations_page.tsx:31-37,
//   student_course_invitation_page.tsx:40-58, and student_course_landing_page.tsx:61-101 own
//   the visible Student course, invitation, acceptance, and released Assignment journey.
// - src/pages/assignment_attempt_page.tsx and src/components/question_response_controls/common.tsx own the attempt surface
//   and visible Question Response Controls.
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { faultHandshakeFromEnvironment } from "./fault_handshake";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  enterStudentCourse,
  expectObservedOrigin,
  observeContextOrigins,
  requireScenarioInput,
  writeContextOriginReceipt,
} from "./real_stack_ui";

const maryEmail = "mary.okafor@live-demo.invalid";
const marySeededCourseLongName = "Biochemistry 301: Proteins and Peptides";
const timeoutMs = 600_000;
const actionTimeoutMs = 30_000;
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };

function nativePleStatusPath(started: unknown, responseUrl: string): string {
  if (typeof started !== "object" || started === null) {
    throw new Error("Assignment start response was not an object");
  }
  const questions = (started as { readonly questions?: unknown }).questions;
  if (!Array.isArray(questions) || questions.length !== 1) {
    throw new Error("Assignment start response did not issue one native PLE Question");
  }
  const question = questions[0] as unknown;
  if (typeof question !== "object" || question === null) {
    throw new Error("Assignment start Question was not an object");
  }
  const nonce = (question as { readonly presentationNonce?: unknown }).presentationNonce;
  if (typeof nonce !== "string" || !/^[0-9a-f]{32}$/u.test(nonce)) {
    throw new Error("Assignment start Question did not include a presentation nonce");
  }
  const endpoint = new URL(responseUrl);
  const match = endpoint.pathname.match(
    /^\/api\/course-instances\/(C-[1-9][0-9]*)\/assignments\/(A-[1-9][0-9]*)\/start$/u,
  );
  if (match?.[1] === undefined || match[2] === undefined) {
    throw new Error(
      "Assignment start response did not use public Course and Assignment references",
    );
  }
  return `/api/course-instances/${match[1]}/assignments/${match[2]}/presentations/${nonce}/submissions`;
}

async function gradingState(page: Page, path: string): Promise<string> {
  return page.evaluate(async (statusPath) => {
    const response = await fetch(statusPath, { cache: "no-store" });
    if (!response.ok) throw new Error(`Native PLE grading status failed: ${response.status}`);
    const value: unknown = await response.json();
    if (typeof value !== "object" || value === null) {
      throw new Error("Native PLE grading status was not an object");
    }
    const state = (value as { readonly gradingState?: unknown }).gradingState;
    if (state !== "pending" && state !== "graded" && state !== "instructorAttention") {
      throw new Error("Native PLE grading status was not closed");
    }
    return state;
  }, path);
}

async function createCourseAssignment(
  page: Page,
  courseShortName: string,
  courseLongName: string,
  assignment: string,
): Promise<void> {
  const blueprint = `${courseLongName} blueprint`;
  await page
    .getByRole("navigation", { name: "Ribbon tasks", exact: true })
    .getByRole("link", { name: "My Blueprint Courses", exact: true })
    .click();
  await page.getByRole("button", { name: "Create Blueprint Course", exact: true }).click();
  await page.getByLabel("Blueprint Course title").fill(blueprint);
  await page.getByRole("button", { name: "Choose published Questions", exact: true }).click();
  await page.getByRole("button", { name: "Search questions", exact: true }).click();
  await page.locator(".question-picker-result input").first().check();
  await page.getByRole("button", { name: "Use selected Questions", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Create Blueprint Course", exact: true })
    .click();
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page
    .getByLabel("Blueprint Course Revision")
    .selectOption({ label: `${blueprint} · Revision 1` });
  await page.getByLabel("Course short name").fill(courseShortName);
  await page.getByLabel("Course long name").fill(courseLongName);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByRole("button", { name: "Create Course Instance", exact: true }).click();
  await expect(page.getByRole("heading", { name: courseLongName, exact: true })).toBeVisible();
  await page.getByRole("link", { name: "Open Course Instance", exact: true }).first().click();
  await page.waitForURL(/\/courses\/C-[1-9][0-9]*$/u);
  const students = page.getByRole("link", { name: "Open Students", exact: true });
  await expect(students).toBeVisible();
  await students.click();
  await page.waitForURL(/\/instructor\/courses\/C-[1-9][0-9]*\/students$/u);
  await expect(page.getByRole("heading", { name: "Students", exact: true })).toBeVisible();
  await page.getByLabel("Email, roster ID").fill(`${maryEmail},recovery-student`);
  await page.getByRole("button", { name: "Import roster", exact: true }).click();
  await expect(page.getByText("Roster import recorded.")).toBeVisible();
  await page.getByRole("link", { name: "Return to Course Instances", exact: true }).click();
  const courseCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: courseLongName, exact: true }) });
  await courseCard.getByRole("link", { name: "Open Course Instance", exact: true }).click();
  await expect(page.getByRole("heading", { name: courseLongName, exact: true })).toBeVisible();
  await page.getByRole("link", { name: "Create Assignment", exact: true }).click();
  await page.getByLabel("Assignment title").fill(assignment);
  await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
  await page.waitForURL(
    /\/instructor\/courses\/C-[1-9][0-9]*\/assignments\/A-[1-9][0-9]*\/questions$/u,
  );
  await expect(page.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Search question library", exact: true }).click();
  const picker = page.getByRole("dialog", { name: "Choose assignment questions", exact: true });
  await picker.getByRole("button", { name: "Search questions", exact: true }).click();
  await picker.locator(".question-picker-result input").first().check();
  await picker.getByRole("button", { name: "Add selected questions", exact: true }).click();
  await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
  await expect(
    page.getByText("Questions and order saved. Review assignment policies when you are ready."),
  ).toBeVisible();
  await page.getByRole("link", { name: "Review assignment policies", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Policies", exact: true })).toBeVisible();
  await page.getByLabel("Due date (America/Chicago)", { exact: true }).fill("2026-12-01");
  await page.getByLabel("Due time", { exact: true }).fill("12:00");
  await page.getByLabel("Time limit in seconds", { exact: true }).fill("3600");
  await page.getByLabel("Late-work rule").selectOption("mark_late");
  await page.getByRole("button", { name: "Save assignment policies", exact: true }).click();
  await expect(
    page.getByText("Assignment policies saved. Future Attempts use the current policy values."),
  ).toBeVisible();
  const deliveryCheckPage = page.context().waitForEvent("page");
  await page.getByRole("link", { name: "Check assignment delivery", exact: true }).click();
  const deliveryCheck = await deliveryCheckPage;
  await expect(
    deliveryCheck.getByRole("heading", { name: "Assignment delivery check", exact: true }),
  ).toBeVisible();
  await expect(
    deliveryCheck.getByText("Preview only - no Student work or grades are created."),
  ).toBeVisible();
  await expect(
    deliveryCheck.getByRole("button", {
      name: /response|submit|start assignment|student attempt/iu,
    }),
  ).toHaveCount(0);
  await expect(
    deliveryCheck.getByRole("link", { name: "Return to assignment policies", exact: true }),
  ).toBeVisible();
  await deliveryCheck.close();
  await page.getByRole("button", { name: "Release assignment", exact: true }).click();
  await expect(
    page.getByText(/^Assignment released\. Current edit number: [1-9][0-9]*\.$/u),
  ).toBeVisible();
}

async function signInStudentAndStartAssignment(
  page: Page,
  courseLongName: string,
  assignmentTitle: string,
): Promise<string> {
  await page.goto("/sign-in");
  await expect(
    page.getByRole("heading", { level: 1, name: "Explore Peptidyle Learning Engine", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: /Continue as .*Mary Okafor/iu }).click();
  await expect(page.getByRole("button", { name: "Sign out", exact: true })).toBeVisible();
  await enterStudentCourse(page, marySeededCourseLongName);
  await page.getByRole("link", { name: "Your courses", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Your courses", exact: true })).toBeVisible();
  await page.getByRole("link", { name: "Course invitations", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Course invitations", exact: true }),
  ).toBeVisible();
  const invitation = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: courseLongName, exact: true }) });
  await invitation.getByRole("link", { name: "Review invitation", exact: true }).click();
  await page.getByRole("button", { name: "Accept invitation", exact: true }).click();
  await expect(page.getByText("Invitation accepted.", { exact: true })).toBeVisible();
  await page.getByRole("link", { name: "Open assigned work", exact: true }).click();
  await expect(page.getByRole("heading", { name: courseLongName, exact: true })).toBeVisible();
  const assignment = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignment.getByRole("link", { name: "Open Assignment", exact: true }).click();
  const started = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      /^\/api\/course-instances\/C-[1-9][0-9]*\/assignments\/A-[1-9][0-9]*\/start$/u.test(
        new URL(response.url()).pathname,
      ),
  );
  await page.getByRole("button", { name: "Start Assignment", exact: true }).click();
  const startResponse = await started;
  expect(startResponse.status()).toBe(201);
  const startPayload = (await startResponse.json()) as unknown;
  const statusPath = nativePleStatusPath(startPayload, startResponse.url());
  await expect(page.locator('[data-route-surface="assignmentAttempt"]')).toBeVisible();
  await page.getByRole("radio").first().check();
  await page.getByRole("button", { name: "Save response", exact: true }).click();
  await expect(page.getByText("Response saved.", { exact: true })).toBeVisible();
  return statusPath;
}

test.describe.configure({ mode: "serial" });

test("student native PLE recovery: one accepted response survives the owner worker interruption", async ({
  browser,
}) => {
  test.setTimeout(timeoutMs);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("learner_native_ple_recovery");
  expect(scenarioInput.faultTransition).toBe("native_ple_submission_recovery");
  const handshake = await faultHandshakeFromEnvironment(
    process.env,
    scenarioInput.scenarioId,
    scenarioInput.namespace,
  );
  const courseShortName = "BIO Recovery";
  const courseLongName = `Biochemistry: Resilient Practice ${scenarioInput.namespace}`;
  const assignment = `Peptide Bonds: Connection Recovery ${scenarioInput.namespace}`;
  const origins = {
    instructor: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    student: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
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
      origins.student.pageOrigins,
      origins.student.requestOrigins,
    );
    const instructor = await instructorContext.newPage();
    const student = await learnerContext.newPage();
    configureContextAndPage(instructorContext, instructor, actionTimeoutMs);
    configureContextAndPage(learnerContext, student, actionTimeoutMs);
    await chooseSeededIdentity(instructor, /Elena Rivera/u);
    await createCourseAssignment(instructor, courseShortName, courseLongName, assignment);
    const statusPath = await signInStudentAndStartAssignment(student, courseLongName, assignment);
    await student.getByRole("button", { name: "Submit Assignment", exact: true }).click();
    await expect(
      student.getByRole("heading", { name: "Assignment submitted", exact: true }),
    ).toBeVisible();
    await expect.poll(async () => gradingState(student, statusPath)).toBe("pending");
    handshake.notify("submission_accepted");
    await handshake.waitFor("native_ple_worker_replaced");
    await expect
      .poll(async () => gradingState(student, statusPath), { timeout: 150_000, intervals: [2_000] })
      .toBe("graded");
    await expect(student.getByText(/correct|score|answer/iu)).toHaveCount(0);
    await student.reload();
    await expect(
      student.getByRole("heading", { name: "Assignment submitted", exact: true }),
    ).toBeVisible();
    await expect(
      student.getByRole("button", { name: "Start Assignment", exact: true }),
    ).toHaveCount(0);
    expectObservedOrigin(origins.instructor, expected);
    expectObservedOrigin(origins.student, expected);
    originEvidence = true;
    handshake.notify("completed");
  } finally {
    handshake.close();
    await Promise.all(contexts.map(async (context) => context.close()));
    if (originEvidence) writeContextOriginReceipt(origins);
  }
});
