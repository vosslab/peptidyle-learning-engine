// Production-stack Question Pool delivery through the current Instructor and Student tasks.
//
// Selector contract:
// - src/features/ple_question_json_authoring/question_json_editor_page.tsx:535-665 owns Draft
//   Question publication and the answer-free published confirmation.
// - src/features/blueprint_course/blueprint_course_create_dialog.tsx:128-190 and
//   src/pages/course_list_page.tsx:12-25,155-219 own Blueprint Course and Course Instance creation.
//   tests/playwright/e2e/real_stack_ui.ts:102-113 owns exact visible Course Instance selection.
// - src/pages/course_roster_page.tsx:50-57,124-143 and
//   src/pages/student_course_invitation_page.tsx:35-48 own roster import's pending-invitation
//   confirmation and Student Course Invitation acceptance.
// - src/pages/assignment_workspace/assignment_workspace_questions_page.tsx:357-467 and
//   src/pages/assignment_pool_editor.tsx:111-281 own ordered Question Pool configuration,
//   server previews, and the post-issue structural-change boundary.
// - src/pages/assignment_release_page.tsx:301-332 and src/pages/assignment_overview_page.tsx:113-179
//   own release and answer-free issued Question delivery.
import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  expectObservedOrigin,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  writeContextOriginReceipt,
} from "./real_stack_ui";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 300_000;
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };

interface CourseAssignment {
  readonly course: string;
  readonly assignment: string;
}

async function publishQuestion(page: Page, title: string): Promise<void> {
  await page.getByRole("link", { name: "Question Library", exact: true }).click();
  await page.getByRole("link", { name: "My Question Drafts", exact: true }).click();
  await page.getByRole("button", { name: "New Draft Question", exact: true }).click();
  await page.getByLabel("Question Title").fill(title);
  await page.getByLabel("Question License").selectOption("CC-BY-4.0");
  await page.getByRole("button", { name: "Save private draft", exact: true }).click();
  await page.getByRole("button", { name: "Review publication changes", exact: true }).click();
  await page.getByLabel("Question Authors").fill("Live Demo Instructor");
  await page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Published", exact: true })).toBeVisible();
}

async function chooseQuestions(picker: Locator, titles: ReadonlyArray<string>): Promise<void> {
  for (const title of titles) {
    await picker.getByLabel("Search questions", { exact: true }).fill(title);
    await picker.getByRole("button", { name: "Search questions", exact: true }).click();
    await picker.getByRole("checkbox", { name: new RegExp(title, "u") }).check();
  }
}

async function createCourseAssignment(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
  fixedTitle: string,
  poolTitles: ReadonlyArray<string>,
): Promise<CourseAssignment> {
  const blueprintTitle = `${courseTitle} blueprint`;
  await page.getByRole("link", { name: "Blueprint Courses", exact: true }).click();
  await page.getByRole("button", { name: "Create Blueprint Course", exact: true }).click();
  await page.getByLabel("Blueprint Course title").fill(blueprintTitle);
  await page.getByRole("button", { name: "Choose published Questions", exact: true }).click();
  const blueprintPicker = page.getByRole("dialog", {
    name: "Choose the first reusable Questions",
    exact: true,
  });
  await chooseQuestions(blueprintPicker, [fixedTitle]);
  await blueprintPicker
    .getByRole("button", { name: "Use selected Questions", exact: true })
    .click();
  await page
    .getByRole("dialog", { name: "Create a Blueprint Course", exact: true })
    .getByRole("button", { name: "Create Blueprint Course", exact: true })
    .click();

  await page.getByRole("link", { name: "Courses", exact: true }).click();
  await page
    .getByLabel("Blueprint Course Revision")
    .selectOption({ label: `${blueprintTitle} · Revision 1` });
  await page.getByLabel("Course Instance title").fill(courseTitle);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByLabel("Course Time Zone (IANA)").fill("America/Chicago");
  await page.getByRole("button", { name: "Create Course Instance", exact: true }).click();
  await selectVisibleCourse(page, courseTitle);
  const courseMatch = /\/courses\/(C-[1-9][0-9]*)$/u.exec(new URL(page.url()).pathname);
  expect(courseMatch).not.toBeNull();
  const course = courseMatch![1]!;

  await page.getByRole("link", { name: "Open Students", exact: true }).click();
  await page
    .getByLabel("Email, roster ID")
    .fill("mary.student@live-demo.invalid,item-pool-student");
  await page.getByRole("button", { name: "Import roster", exact: true }).click();
  await expect(
    page.getByText(
      "Roster import recorded. Students remain pending until they claim their Course Invitation.",
      { exact: true },
    ),
  ).toBeVisible();
  await page.getByRole("link", { name: "Course", exact: true }).click();
  await page.getByRole("link", { name: "Open Assignments", exact: true }).click();
  await page.getByRole("link", { name: "Create the first assignment", exact: true }).click();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByLabel("Instructions").fill("Complete the fixed Question and Question Pool.");
  await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Search question library", exact: true }).click();
  const fixedPicker = page.getByRole("dialog", {
    name: "Choose assignment questions",
    exact: true,
  });
  await chooseQuestions(fixedPicker, [fixedTitle]);
  await fixedPicker.getByRole("button", { name: "Add selected questions", exact: true }).click();
  await page.getByRole("button", { name: "Add question pool", exact: true }).click();
  const pool = page.getByRole("listitem", { name: "Question pool 2", exact: true });
  await pool.getByRole("button", { name: "Choose Questions for pool", exact: true }).click();
  const poolPicker = page.getByRole("dialog", { name: "Choose Questions for pool", exact: true });
  await chooseQuestions(poolPicker, poolTitles);
  await poolPicker
    .getByRole("button", { name: "Add selected Questions to pool", exact: true })
    .click();
  await pool.getByLabel("Selection count", { exact: true }).fill("2");
  await pool
    .getByLabel("Selected Question order", { exact: true })
    .selectOption("questionPoolOrder");
  await page.getByRole("button", { name: "Save questions and order", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("Questions and order saved.");

  await pool.getByRole("button", { name: "Preview selection", exact: true }).click();
  const preview = pool
    .getByRole("heading", { name: "Question Pool Selection", exact: true })
    .locator("..");
  await expect(preview).toBeVisible();
  await expect(
    preview.getByRole("heading", { name: "Question Pool Items", exact: true }),
  ).toBeVisible();
  const previewSelection = await preview
    .getByRole("heading", { name: "Server-selected Questions", exact: true })
    .locator("..")
    .getByRole("listitem")
    .allTextContents();
  expect(previewSelection).toHaveLength(2);
  expect(new Set(previewSelection).size).toBe(2);

  await page.getByRole("link", { name: "Review assignment policies", exact: true }).click();
  await page.getByRole("button", { name: "Validate Assignment", exact: true }).click();
  await page.getByRole("button", { name: "Release Assignment", exact: true }).click();
  const assignmentMatch = /\/courses\/C-[1-9][0-9]*\/assignments\/(A-[1-9][0-9]*)\/release$/u.exec(
    new URL(page.url()).pathname,
  );
  expect(assignmentMatch).not.toBeNull();
  return { course, assignment: assignmentMatch![1]! };
}

async function issueStudentWork(
  page: Page,
  references: CourseAssignment,
  fixedTitle: string,
  poolTitles: ReadonlyArray<string>,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Student/u);
  await page.goto(`/courses/${references.course}/invitation`);
  await expect(
    page.getByRole("heading", { name: "Join this Course Instance", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Accept Course Invitation", exact: true }).click();
  await expect(page.getByText("Course Invitation accepted.", { exact: true })).toBeVisible();
  await page.goto(`/courses/${references.course}/assignments/${references.assignment}`);
  await page.getByRole("button", { name: "Start Assignment", exact: true }).click();
  const overview = page.locator('[data-route-surface="assignmentOverview"]');
  await expect(overview.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
  const issuedTitles = (await overview.getByRole("heading", { level: 3 }).allTextContents()).map(
    (value) => value.replace(/^Question [1-9][0-9]*: /u, ""),
  );
  expect(issuedTitles[0]).toBe(fixedTitle);
  const selectedPoolTitles = issuedTitles.slice(1);
  expect(selectedPoolTitles).toHaveLength(2);
  expect(new Set(selectedPoolTitles).size).toBe(2);
  const selectedIndexes = selectedPoolTitles.map((title) => poolTitles.indexOf(title));
  expect(selectedIndexes.every((index) => index >= 0)).toBe(true);
  expect(selectedIndexes).toEqual([...selectedIndexes].sort((left, right) => left - right));

  await page.reload();
  await page.getByRole("button", { name: "Start Assignment", exact: true }).click();
  const reissuedTitles = (await overview.getByRole("heading", { level: 3 }).allTextContents()).map(
    (value) => value.replace(/^Question [1-9][0-9]*: /u, ""),
  );
  expect(reissuedTitles).toEqual(issuedTitles);
}

async function provePostIssuePoolImmutability(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
): Promise<void> {
  await chooseSeededIdentity(page, /Elena Instructor/u);
  await selectVisibleCourse(page, courseTitle);
  await page.getByRole("link", { name: "Open Assignments", exact: true }).click();
  const assignmentCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignmentCard.getByRole("link", { name: "Open assignment", exact: true }).click();
  await page.getByRole("link", { name: "Review questions", exact: true }).click();
  const pool = page.getByRole("listitem", { name: "Question pool 2", exact: true });
  await pool.getByLabel("Selection count", { exact: true }).fill("1");
  await page.getByRole("button", { name: "Save questions and order", exact: true }).click();
  const boundary = page.getByRole("alert");
  await expect(boundary).toContainText("Student work already pins this Assignment Revision.");
  await expect(boundary).toContainText(
    "a successor Assignment is required for structural changes.",
  );
}

test.describe.configure({ mode: "serial" });

test("Instructor delivers an immutable ordered Question Pool to a Student", async ({ browser }) => {
  test.setTimeout(scenarioTimeoutMs);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("item_pool_delivery");
  const courseTitle = `Biochemistry: Question Pool ${scenarioInput.namespace}`;
  const assignmentTitle = `Peptide Bond Question Pool ${scenarioInput.namespace}`;
  const fixedTitle = `Fixed peptide question ${scenarioInput.namespace}`;
  const poolTitles = [
    `Peptide pool item one ${scenarioInput.namespace}`,
    `Peptide pool item two ${scenarioInput.namespace}`,
    `Peptide pool item three ${scenarioInput.namespace}`,
  ];
  const origins = {
    instructor: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    student: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
  };
  const contexts: BrowserContext[] = [];

  try {
    const instructorContext = await browser.newContext(contextOptions);
    const studentContext = await browser.newContext(contextOptions);
    contexts.push(instructorContext, studentContext);
    observeContextOrigins(
      instructorContext,
      origins.instructor.pageOrigins,
      origins.instructor.requestOrigins,
    );
    observeContextOrigins(
      studentContext,
      origins.student.pageOrigins,
      origins.student.requestOrigins,
    );
    const instructor = await instructorContext.newPage();
    const student = await studentContext.newPage();
    configureContextAndPage(instructorContext, instructor, actionTimeoutMs);
    configureContextAndPage(studentContext, student, actionTimeoutMs);

    await chooseSeededIdentity(instructor, /Elena Instructor/u);
    for (const title of [fixedTitle, ...poolTitles]) await publishQuestion(instructor, title);
    const references = await createCourseAssignment(
      instructor,
      courseTitle,
      assignmentTitle,
      fixedTitle,
      poolTitles,
    );
    await issueStudentWork(student, references, fixedTitle, poolTitles);
    await provePostIssuePoolImmutability(instructor, courseTitle, assignmentTitle);
    expectObservedOrigin(origins.instructor, new URL(scenarioInput.baseUrl).origin);
    expectObservedOrigin(origins.student, new URL(scenarioInput.baseUrl).origin);
  } finally {
    try {
      await Promise.all(contexts.map(async (context) => await context.close()));
    } finally {
      writeContextOriginReceipt(origins);
    }
  }
});
