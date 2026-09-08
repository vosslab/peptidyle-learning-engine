// One accepted response remains durable while the owner recovers the native PLE worker.
//
// Selector contract:
// - src/features/ple_question_json_authoring/question_json_editor_page.tsx:535 owns question creation
//   fields and publication controls used to seed the recovery journey.
// - src/pages/course_list_page.tsx, course_instance_page.tsx, course_roster_page.tsx, and
//   assignment_release_page.tsx own visible Course, roster, and direct Assignment release setup.
// - src/pages/student_courses_page.tsx:27-33, student_course_invitations_page.tsx:31-37,
//   student_course_invitation_page.tsx:40-58, and student_course_landing_page.tsx:61-101 own
//   the visible Student course, invitation, acceptance, and released Assignment journey.
// - src/pages/assignment_overview_page.tsx:114 owns the Start assignment control and its
//   answer-free submission-status display.
// - src/pages/assignment_attempt_page.tsx and src/components/question_response_controls/common.tsx own the attempt surface
//   and visible Question Response Controls.
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { faultHandshakeFromEnvironment } from "./fault_handshake";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  expectObservedOrigin,
  observeContextOrigins,
  requireScenarioInput,
  writeContextOriginReceipt,
} from "./real_stack_ui";

const maryEmail = "mary.okafor@live-demo.invalid";
const timeoutMs = 600_000;
const actionTimeoutMs = 30_000;
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };

async function createCourseAssignment(
  page: Page,
  course: string,
  assignment: string,
): Promise<void> {
  const blueprint = `${course} blueprint`;
  await page.getByRole("link", { name: "Blueprint Courses", exact: true }).click();
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
  await page.getByRole("link", { name: "Courses", exact: true }).click();
  await page
    .getByLabel("Blueprint Course Revision")
    .selectOption({ label: `${blueprint} · Revision 1` });
  await page.getByLabel("Course Instance title").fill(course);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByLabel("Course Time Zone (IANA)").fill("America/Chicago");
  await page.getByRole("button", { name: "Create Course Instance", exact: true }).click();
  await expect(page.getByRole("heading", { name: course, exact: true })).toBeVisible();
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
    .filter({ has: page.getByRole("heading", { name: course, exact: true }) });
  await courseCard.getByRole("link", { name: "Open Course Instance", exact: true }).click();
  await expect(page.getByRole("heading", { name: course, exact: true })).toBeVisible();
  await page.getByRole("link", { name: "Open Assignments", exact: true }).click();
  await page.getByLabel("Assignment title").fill(assignment);
  await page.getByLabel("Instructions").fill("Complete the selected published Question.");
  await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
  await page.waitForURL(
    /\/instructor\/courses\/C-[1-9][0-9]*\/assignments\/A-[1-9][0-9]*\/release$/u,
  );
  await expect(
    page.getByRole("heading", { name: "Assignment Workspace", exact: true }),
  ).toBeVisible();
  const available = page.getByRole("group", { name: "Available Published Questions" });
  await available.getByRole("checkbox").first().check();
  await page.getByLabel("Due date").fill("2026-12-01T12:00");
  await page.getByLabel("Late-work rule").selectOption("mark_late");
  await page.getByRole("button", { name: "Save Assignment", exact: true }).click();
  await expect(page.getByRole("status")).toBeVisible();
  await page.getByRole("button", { name: "Open Assignment Preview", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Assignment Preview", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("This answer-free preview does not create a Student attempt or access."),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: /response|submit|start assignment|student attempt/iu }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Return to Assignment Workspace", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Assignment Preview", exact: true })).toBeHidden();
  await page.getByRole("button", { name: "Release Assignment", exact: true }).click();
}

async function signInStudentAndStartAssignment(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
): Promise<void> {
  await page.goto("/sign-in");
  await expect(
    page.getByRole("heading", { level: 1, name: "Explore Peptidyle Learning Engine", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: /Continue as .*Mary Okafor/iu }).click();
  await expect(page.getByRole("button", { name: "Sign out", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Your courses", exact: true })).toBeVisible();
  await page.getByRole("link", { name: "Course invitations", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Course invitations", exact: true }),
  ).toBeVisible();
  const invitation = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: courseTitle, exact: true }) });
  await invitation.getByRole("link", { name: "Review Course Invitation", exact: true }).click();
  await page.getByRole("button", { name: "Accept Course Invitation", exact: true }).click();
  await expect(page.getByText("Course Invitation accepted.", { exact: true })).toBeVisible();
  await page.getByRole("link", { name: "Open assigned work", exact: true }).click();
  await expect(page.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
  const assignment = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignment.getByRole("link", { name: "Open Assignment", exact: true }).click();
  await page.getByRole("button", { name: "Start Assignment", exact: true }).click();
  const overview = page.locator('[data-route-surface="assignmentOverview"]');
  await expect(overview.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
  await expect(overview.getByRole("heading", { name: /^Question 1:/u })).toBeVisible();
  await overview.getByRole("link", { name: "Answer this question", exact: true }).click();
  await page.getByRole("button", { name: "Start Assignment", exact: true }).click();
  await expect(page.locator('[data-route-surface="assignmentOverview"]')).toBeVisible();
  await page.getByRole("radio").first().check();
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
  const course = `Biochemistry: Resilient Practice ${scenarioInput.namespace}`;
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
    await createCourseAssignment(instructor, course, assignment);
    await signInStudentAndStartAssignment(student, course, assignment);
    await student.getByRole("button", { name: "Submit answer" }).click();
    const pending = student
      .getByRole("heading", { name: "Response received", exact: true })
      .locator("..");
    await expect(pending).toBeVisible();
    await expect(
      pending.getByRole("button", { name: "Check grading status", exact: true }),
    ).toBeVisible();
    handshake.notify("submission_accepted");
    await handshake.waitFor("native_ple_worker_replaced");
    const terminal = student.getByRole("heading", { name: "Graded", exact: true }).locator("..");
    await expect
      .poll(
        async () => {
          const check = student.getByRole("button", {
            name: "Check grading status",
            exact: true,
          });
          if (await check.isVisible()) await check.click();
          return await terminal.isVisible();
        },
        { timeout: 150_000, intervals: [2_000] },
      )
      .toBe(true);
    await expect(terminal.getByText(/correct|score|answer/iu)).toHaveCount(0);
    await student.reload();
    await student.getByRole("button", { name: "Start Assignment", exact: true }).click();
    await expect(terminal).toBeVisible();
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
