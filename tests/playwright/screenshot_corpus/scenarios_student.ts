// scenarios_student.ts - Student course, invitation, delivery, and denial survey.
// Selector contract: shared Course and Assignment actions live in visible_workflows.ts:47; Student
// headings and controls are owned by src/pages/student_courses_page.tsx:29,
// src/pages/student_course_invitation_page.tsx:42, src/pages/assignment_overview_page.tsx:236,
// and src/pages/assignment_attempt_page.tsx:730.

import { expect } from "@playwright/test";
import type { Page } from "playwright";

import type { CaptureRecord } from "./manifest";
import type { CaptureSession, ScenarioRuntime } from "./runtime";
import type { ScenarioDefinition } from "./scenario_types";
import {
  COURSE_TITLE,
  assignmentCard,
  choosePersona,
  courseCard,
  enterInstructor,
  enterStudentAssignment,
  openStudentAssignment,
  openStudentCourse,
  scrollTop,
  startStudentAssignment,
  type SeededPersona,
} from "./visible_workflows";

const INVITATION_COURSE_TITLE = "Screenshot Corpus Invitation Course";

async function captureCheckpoint(
  runtime: ScenarioRuntime,
  scenario: string,
  checkpoint: string,
  session: CaptureSession,
  top = true,
): Promise<void> {
  if (top) await scrollTop(session.page);
  await runtime.capture(session, runtime.record(scenario, checkpoint));
}

async function studentCourseList(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "student_course_list";
  for (const checkpoint of ["course_list_laptop", "course_list_phone"] as const) {
    const record = runtime.record(scenario, checkpoint);
    const session = await runtime.open(record);
    try {
      await choosePersona(session.page, "Mary Okafor");
      await session.page.getByRole("heading", { name: "Your courses", exact: true }).waitFor();
      await courseCard(session.page).waitFor();
      await captureCheckpoint(runtime, scenario, checkpoint, session);
    } finally {
      await runtime.close(session);
    }
  }
}

async function prepareStudentInvitation(
  runtime: ScenarioRuntime,
  record: CaptureRecord,
): Promise<void> {
  const setup = await runtime.open(record);
  const page = setup.page;
  try {
    await enterInstructor(page);
    await page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await page.getByRole("heading", { name: "Course Instances you teach", exact: true }).waitFor();
    await page
      .getByLabel("Blueprint Course Revision")
      .selectOption({ label: `${COURSE_TITLE} · Revision 1` });
    await page.getByLabel("Course Instance title").fill(INVITATION_COURSE_TITLE);
    await page.getByLabel("Course Term start date").fill("2026-09-01");
    await page.getByLabel("Course Term end date").fill("2026-12-18");
    await page.getByLabel("Course Time Zone (IANA)").fill("America/Chicago");
    await page.getByRole("button", { name: "Create Course Instance", exact: true }).click();
    const created = courseCard(page, INVITATION_COURSE_TITLE);
    await created.waitFor();
    await created.getByRole("link", { name: "Open Course Instance", exact: true }).click();
    await page.getByRole("link", { name: "Open Students", exact: true }).click();
    await page.getByRole("heading", { name: "Students", exact: true }).waitFor();
    await page
      .getByLabel("Email, roster ID")
      .fill("mary.okafor@live-demo.invalid,screenshot-invitation");
    await page.getByRole("button", { name: "Import roster", exact: true }).click();
    await page.getByText("screenshot-invitation", { exact: true }).waitFor();
  } finally {
    await runtime.close(setup);
  }
}

async function studentInvitation(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "student_invitation";
  const indexRecord = runtime.record(scenario, "invitation_index");
  await prepareStudentInvitation(runtime, indexRecord);
  const session = await runtime.open(indexRecord);
  const page = session.page;
  try {
    await choosePersona(page, "Mary Okafor");
    await page.getByRole("link", { name: "Course invitations", exact: true }).click();
    await page.getByRole("heading", { name: "Course invitations", exact: true }).waitFor();
    const invitation = courseCard(page, INVITATION_COURSE_TITLE);
    await invitation.waitFor();
    await captureCheckpoint(runtime, scenario, "invitation_index", session);
    await invitation.getByRole("link", { name: "Review Course Invitation", exact: true }).click();
    await page.getByRole("heading", { name: "Join this Course Instance", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "invitation_detail", session);
    await page.getByRole("button", { name: "Accept Course Invitation", exact: true }).click();
    await page.getByText("Course Invitation accepted.", { exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "invitation_accepted", session);
    await page.getByRole("link", { name: "Open assigned work", exact: true }).click();
    await page.getByRole("heading", { name: INVITATION_COURSE_TITLE, exact: true }).waitFor();
  } finally {
    await runtime.close(session);
  }
}

type StudentPersona = Extract<SeededPersona, "Mary Okafor" | "Jack Nguyen" | "Avery Thompson">;

async function captureLanding(
  runtime: ScenarioRuntime,
  checkpoint: string,
  persona: StudentPersona,
  expectedState: string,
): Promise<void> {
  const scenario = "student_landings";
  const record = runtime.record(scenario, checkpoint);
  const session = await runtime.open(record);
  try {
    await choosePersona(session.page, persona);
    await openStudentCourse(session.page);
    await assignmentCard(session.page).getByText(expectedState, { exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, checkpoint, session);
  } finally {
    await runtime.close(session);
  }
}

async function studentLandings(runtime: ScenarioRuntime): Promise<void> {
  await captureLanding(runtime, "not_started_laptop", "Avery Thompson", "Not started");
  await captureLanding(runtime, "in_progress_laptop", "Jack Nguyen", "In progress");
  await captureLanding(runtime, "completed_laptop", "Mary Okafor", "Completed and scored");
  await captureLanding(runtime, "not_started_phone", "Avery Thompson", "Not started");
}

async function captureAssignmentOverview(
  runtime: ScenarioRuntime,
  checkpoint: string,
  persona: StudentPersona,
  expectedCourseState: "Not started",
): Promise<void> {
  const scenario = "student_assignment_overviews";
  const record = runtime.record(scenario, checkpoint);
  const session = await runtime.open(record);
  try {
    await choosePersona(session.page, persona);
    await openStudentCourse(session.page);
    await assignmentCard(session.page).getByText(expectedCourseState, { exact: true }).waitFor();
    await openStudentAssignment(session.page);
    await session.page
      .getByText("This Assignment is available to start.", { exact: true })
      .waitFor();
    await captureCheckpoint(runtime, scenario, checkpoint, session);
  } finally {
    await runtime.close(session);
  }
}

async function captureResumedAssignmentOverview(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "student_assignment_overviews";
  const record = runtime.record(scenario, "resumed_laptop");
  const session = await runtime.open(record);
  try {
    await choosePersona(session.page, "Jack Nguyen");
    await openStudentCourse(session.page);
    await assignmentCard(session.page).getByText("In progress", { exact: true }).waitFor();
    await openStudentAssignment(session.page);
    await startStudentAssignment(session.page);
    await session.page
      .getByText("Your current Assignment Attempt has been reopened.", { exact: true })
      .waitFor();
    await captureCheckpoint(runtime, scenario, "resumed_laptop", session);
  } finally {
    await runtime.close(session);
  }
}

async function studentAssignmentOverviews(runtime: ScenarioRuntime): Promise<void> {
  await captureAssignmentOverview(runtime, "unanswered_laptop", "Avery Thompson", "Not started");
  await captureAssignmentOverview(runtime, "unanswered_tablet", "Avery Thompson", "Not started");
  await captureResumedAssignmentOverview(runtime);
}

async function openUnansweredQuestion(
  runtime: ScenarioRuntime,
  checkpoint: string,
): Promise<CaptureSession> {
  const scenario = "student_question_delivery";
  const session = await runtime.open(runtime.record(scenario, checkpoint));
  await enterStudentAssignment(session.page, "Avery Thompson");
  await startStudentAssignment(session.page);
  return session;
}

async function captureResponsiveUnanswered(runtime: ScenarioRuntime): Promise<CaptureSession> {
  const scenario = "student_question_delivery";
  const laptop = await openUnansweredQuestion(runtime, "question_unanswered_laptop");
  await captureCheckpoint(runtime, scenario, "question_unanswered_laptop", laptop);
  for (const checkpoint of ["question_unanswered_phone", "question_unanswered_square"] as const) {
    const session = await openUnansweredQuestion(runtime, checkpoint);
    try {
      await captureCheckpoint(runtime, scenario, checkpoint, session);
    } finally {
      await runtime.close(session);
    }
  }
  return laptop;
}

async function waitForGraded(page: Page): Promise<void> {
  const graded = page.getByRole("heading", { name: "Graded", exact: true });
  await expect
    .poll(
      async () => {
        if (await graded.isVisible()) return true;
        const check = page.getByRole("button", { name: "Check grading status", exact: true });
        if (await check.isVisible()) await check.click();
        return await graded.isVisible();
      },
      { timeout: 120_000, intervals: [2_000] },
    )
    .toBe(true);
}

async function captureResponseProgression(
  runtime: ScenarioRuntime,
  session: CaptureSession,
): Promise<void> {
  const scenario = "student_question_delivery";
  const page = session.page;
  await page.getByRole("link", { name: "Answer this question", exact: true }).first().click();
  await page.getByRole("button", { name: "Start Assignment", exact: true }).click();
  await page.getByRole("radio").first().check();
  await page.getByRole("button", { name: "Submit answer", exact: true }).waitFor();
  await captureCheckpoint(runtime, scenario, "response_selected", session);
  await page.getByRole("button", { name: "Submit answer", exact: true }).click();
  await page.getByRole("heading", { name: "Response received", exact: true }).waitFor();
  await captureCheckpoint(runtime, scenario, "response_received", session);
  await waitForGraded(page);
  await captureCheckpoint(runtime, scenario, "graded", session);
  await page.reload({ waitUntil: "commit" });
  await page.getByRole("heading", { name: "Graded", exact: true }).waitFor();
  await captureCheckpoint(runtime, scenario, "graded_after_reload", session);
}

async function captureResponseFormats(
  runtime: ScenarioRuntime,
  session: CaptureSession,
): Promise<void> {
  const scenario = "student_question_delivery";
  const page = session.page;
  await page.goto(
    new URL(
      `/courses/${runtime.references.course}/assignments/${runtime.references.assignment}`,
      runtime.entryUrl,
    ).href,
    { waitUntil: "commit" },
  );
  await page.getByRole("button", { name: "Start Assignment", exact: true }).click();
  await page.getByRole("heading", { name: /^Question 3:/u }).waitFor();
  const numeric = page.getByLabel("Numeric response", { exact: true });
  await numeric.scrollIntoViewIfNeeded();
  await runtime.capture(session, runtime.record(scenario, "numerical_response"));
  const hotspot = page.getByRole("group", { name: /Choose the labeled image region/u });
  await hotspot.scrollIntoViewIfNeeded();
  const hotspotImage = hotspot
    .locator("xpath=ancestor::article[contains(@class, 'question-presentation')]")
    .locator("img");
  await hotspotImage.waitFor();
  const hotspotImageHandle = await hotspotImage.elementHandle();
  if (hotspotImageHandle === null) throw new Error("hotspot response image is not attached");
  await page.waitForFunction(
    (image) => image instanceof HTMLImageElement && image.complete && image.naturalWidth > 0,
    hotspotImageHandle,
  );
  await runtime.capture(session, runtime.record(scenario, "hotspot_response"));
}

async function studentQuestionDelivery(runtime: ScenarioRuntime): Promise<void> {
  const laptop = await captureResponsiveUnanswered(runtime);
  try {
    await captureResponseProgression(runtime, laptop);
    await captureResponseFormats(runtime, laptop);
  } finally {
    await runtime.close(laptop);
  }
}

async function captureDenial(runtime: ScenarioRuntime, checkpoint: string): Promise<void> {
  const scenario = "student_authorization";
  const record = runtime.record(scenario, checkpoint);
  const session = await runtime.open(record);
  try {
    const studentCoursesLoaded = session.page.waitForEvent("requestfinished", {
      predicate: (request) => new URL(request.url()).pathname === "/api/student/course-instances",
    });
    await choosePersona(session.page, "Mary Okafor");
    // Drain the completed Student landing responses before navigation can discard their bodies.
    await studentCoursesLoaded;
    await session.privacy.settleResponses();
    await session.page.goto(new URL("/library", runtime.entryUrl).href, { waitUntil: "commit" });
    await session.page
      .getByRole("heading", { name: "This page is not available to this account", exact: true })
      .waitFor();
    await captureCheckpoint(runtime, scenario, checkpoint, session);
  } finally {
    await runtime.close(session);
  }
}

async function studentAuthorization(runtime: ScenarioRuntime): Promise<void> {
  await captureDenial(runtime, "denial_laptop");
  await captureDenial(runtime, "denial_phone");
}

export const STUDENT_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "student_course_list",
    checkpoints: ["course_list_laptop", "course_list_phone"],
    run: studentCourseList,
  },
  {
    id: "student_invitation",
    checkpoints: ["invitation_index", "invitation_detail", "invitation_accepted"],
    run: studentInvitation,
  },
  {
    id: "student_landings",
    checkpoints: [
      "not_started_laptop",
      "in_progress_laptop",
      "completed_laptop",
      "not_started_phone",
    ],
    run: studentLandings,
  },
  {
    id: "student_assignment_overviews",
    checkpoints: ["unanswered_laptop", "unanswered_tablet", "resumed_laptop"],
    run: studentAssignmentOverviews,
  },
  {
    id: "student_question_delivery",
    checkpoints: [
      "question_unanswered_laptop",
      "question_unanswered_phone",
      "question_unanswered_square",
      "response_selected",
      "response_received",
      "graded",
      "graded_after_reload",
      "numerical_response",
      "hotspot_response",
    ],
    run: studentQuestionDelivery,
  },
  {
    id: "student_authorization",
    checkpoints: ["denial_laptop", "denial_phone"],
    run: studentAuthorization,
  },
];
