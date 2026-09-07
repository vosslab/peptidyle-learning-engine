// One accepted response remains durable while the owner recovers the native PLE worker.
//
// Selector contract:
// - src/features/ple_question_json_authoring/question_json_editor_page.tsx:535 owns question creation
//   fields and publication controls used to seed the recovery journey.
// - src/pages/course_list_page.tsx, course_assignments_page.tsx, assignment_workspace/, and
//   course_roster_page.tsx own course, title-first assignment setup, and invitation controls.
// - src/pages/course_invitation_page.tsx:62 and src/pages/assignment_overview_page.tsx:114 own
//   student claiming and the Start assignment control.
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

const maryEmail = "mary.student@live-demo.invalid";
const timeoutMs = 600_000;
const actionTimeoutMs = 30_000;
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };

async function createCourseAssignment(
  page: Page,
  course: string,
  assignment: string,
): Promise<{ readonly course: string; readonly assignment: string }> {
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
  const courseMatch = /\/courses\/(C-[1-9][0-9]*)$/u.exec(new URL(page.url()).pathname);
  expect(courseMatch).not.toBeNull();
  const students = page.getByRole("link", { name: "Open Students", exact: true });
  await expect(students).toBeVisible();
  await students.click();
  await page.waitForURL(/\/instructor\/courses\/C-[1-9][0-9]*\/students$/u);
  await expect(page.getByRole("heading", { name: "Students", exact: true })).toBeVisible();
  await page.getByLabel("Email, roster ID").fill(`${maryEmail},recovery-student`);
  await page.getByRole("button", { name: "Import roster", exact: true }).click();
  await expect(page.getByText("Roster import recorded.")).toBeVisible();
  await page.goto(`/courses/${courseMatch![1]!}`);
  await expect(page.getByRole("heading", { name: course, exact: true })).toBeVisible();
  await page.getByRole("link", { name: "Open Assignments", exact: true }).click();
  await page.getByLabel("Assignment title").fill(assignment);
  await page.getByLabel("Instructions").fill("Complete the selected published Question.");
  await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
  const available = page.getByRole("group", { name: "Available Published Questions" });
  await available.getByRole("checkbox").first().check();
  await page.getByLabel("Due date").fill("2026-12-01T12:00");
  await page.getByLabel("Late-work rule").selectOption("mark_late");
  await page.getByRole("button", { name: "Save Assignment", exact: true }).click();
  await page.getByRole("button", { name: "Validate Assignment", exact: true }).click();
  await page.getByRole("button", { name: "Release Assignment", exact: true }).click();
  const assignmentMatch = /\/courses\/C-[1-9][0-9]*\/assignments\/(A-[1-9][0-9]*)\/release$/u.exec(
    new URL(page.url()).pathname,
  );
  expect(assignmentMatch).not.toBeNull();
  return { course: courseMatch![1]!, assignment: assignmentMatch![1]! };
}

async function startAssignmentAttempt(
  page: Page,
  course: string,
  assignment: string,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Student/u);
  await page.goto(`/courses/${course}/invitation`);
  await page.getByRole("button", { name: "Accept Course Invitation", exact: true }).click();
  await expect(page.getByText("Course Invitation accepted.")).toBeVisible();
  await page.goto(`/courses/${course}/assignments/${assignment}`);
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
    await chooseSeededIdentity(instructor, /Elena Instructor/u);
    await instructor.goto("/library");
    const references = await createCourseAssignment(instructor, course, assignment);
    await startAssignmentAttempt(student, references.course, references.assignment);
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
    const terminal = student.getByRole("heading", { name: /^(Correct|Not quite)$/u });
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
