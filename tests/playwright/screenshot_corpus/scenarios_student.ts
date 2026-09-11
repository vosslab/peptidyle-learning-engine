// scenarios_student.ts - Student course, invitation, delivery, and denial survey.
// Selector contract: shared Course and Assignment actions live in visible_workflows.ts:47; Student
// headings and controls are owned by src/pages/student_courses_page.tsx:29,
// src/pages/student_course_invitation_page.tsx:42, src/pages/assignment_overview_page.tsx:95,
// src/pages/assignment_attempt_page.tsx:322, and src/pages/assignment_attempt_summary_page.tsx:55.

import type { Locator } from "playwright";

import type { CaptureRecord } from "./manifest";
import type { CaptureSession, ScenarioRuntime } from "./runtime";
import type { ScenarioDefinition } from "./scenario_types";
import {
  COURSE_TITLE,
  assignmentCard,
  choosePersona,
  courseCard,
  enterInstructor,
  openStudentAssignment,
  openStudentCourse,
  openStudentCourseChooser,
  resumeStudentAssignmentAttempt,
  scrollTop,
  type SeededPersona,
} from "./visible_workflows";

const INVITATION_COURSE_SHORT_NAME = "Corpus Invite";
const INVITATION_COURSE_LONG_NAME = "Screenshot Corpus Invitation Course";

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
      await openStudentCourseChooser(session.page);
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
    await page.getByLabel("Course short name").fill(INVITATION_COURSE_SHORT_NAME);
    await page.getByLabel("Course long name").fill(INVITATION_COURSE_LONG_NAME);
    await page.getByLabel("Course Term start date").fill("2026-09-01");
    await page.getByLabel("Course Term end date").fill("2026-12-18");
    await page.getByRole("button", { name: "Create Course Instance", exact: true }).click();
    const created = courseCard(page, INVITATION_COURSE_LONG_NAME);
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
    await openStudentCourseChooser(page);
    await page.getByRole("link", { name: "Course invitations", exact: true }).click();
    await page.getByRole("heading", { name: "Course invitations", exact: true }).waitFor();
    const invitation = courseCard(page, INVITATION_COURSE_LONG_NAME);
    await invitation.waitFor();
    await captureCheckpoint(runtime, scenario, "invitation_index", session);
    await invitation.getByRole("link", { name: "Review invitation", exact: true }).click();
    await page.getByRole("heading", { name: "Join this course", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "invitation_detail", session);
    await page.getByRole("button", { name: "Accept invitation", exact: true }).click();
    await page.getByText("Invitation accepted.", { exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "invitation_accepted", session);
    await page.getByRole("link", { name: "Open assigned work", exact: true }).click();
    await page.getByRole("heading", { name: INVITATION_COURSE_LONG_NAME, exact: true }).waitFor();
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
      .getByRole("heading", { level: 1, name: "Peptide Structure Practice" })
      .waitFor();
    await session.page.getByRole("heading", { name: "Before you start", exact: true }).waitFor();
    await session.page.getByRole("button", { name: "Start Assignment", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, checkpoint, session);
  } finally {
    await runtime.close(session);
  }
}

async function studentAssignmentOverviews(runtime: ScenarioRuntime): Promise<void> {
  await captureAssignmentOverview(runtime, "unanswered_laptop", "Avery Thompson", "Not started");
  await captureAssignmentOverview(runtime, "unanswered_tablet", "Avery Thompson", "Not started");
}

async function studentAssignmentHistory(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "student_assignment_history";
  const overviewRecord = runtime.record(scenario, "overview_history");
  const session = await runtime.open(overviewRecord);
  try {
    await choosePersona(session.page, "Mary Okafor");
    await openStudentCourse(session.page);
    const assignment = assignmentCard(session.page);
    await assignment.getByRole("link", { name: "Open Assignment", exact: true }).click();
    await session.page.locator('[data-route-surface="assignmentOverview"]').waitFor();
    await session.page.getByRole("heading", { name: "Previous attempts", exact: true }).waitFor();
    const previousAttempt = session.page.getByRole("link", { name: "Attempt 1", exact: true });
    await previousAttempt.waitFor();
    await captureCheckpoint(runtime, scenario, "overview_history", session);
    await previousAttempt.click();
    await session.page.locator('[data-route-surface="assignmentAttemptSummary"]').waitFor();
    await session.page.getByRole("heading", { name: "Your recorded work", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "selected_history", session);
  } finally {
    await runtime.close(session);
  }
}

function attemptSurface(session: CaptureSession): Locator {
  return session.page.locator('[data-route-surface="assignmentAttempt"]');
}

function attemptQuestion(session: CaptureSession, name: string): Locator {
  return session.page
    .getByRole("navigation", { name: "Assignment questions", exact: true })
    .getByRole("button", { name, exact: true });
}

async function saveCurrentRadioResponse(session: CaptureSession): Promise<void> {
  const responseControl = attemptSurface(session).locator("section.question-response-control");
  await responseControl.getByRole("radio").first().check();
  await responseControl.getByRole("button", { name: "Save response", exact: true }).click();
  await session.page.getByText("Response saved.", { exact: true }).waitFor();
}

async function saveCurrentNumericResponse(session: CaptureSession): Promise<void> {
  const responseControl = attemptSurface(session).locator("section.question-response-control");
  await responseControl.locator("input[type='number']").fill("1");
  await responseControl.getByRole("button", { name: "Save response", exact: true }).click();
  await session.page.getByText("Response saved.", { exact: true }).waitFor();
}

async function studentAssignmentAttempt(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "student_assignment_attempt";
  const savedRecord = runtime.record(scenario, "response_selected");
  const savedSession = await runtime.open(savedRecord);
  try {
    await choosePersona(savedSession.page, "Avery Thompson");
    await openStudentCourse(savedSession.page);
    await openStudentAssignment(savedSession.page);
    await savedSession.page.getByRole("button", { name: "Start Assignment", exact: true }).click();
    await attemptSurface(savedSession).waitFor();
    await savedSession.page.getByText("Question 1 of 4", { exact: true }).waitFor();
    await saveCurrentRadioResponse(savedSession);
    await attemptQuestion(savedSession, "Question 1: Saved, current").waitFor();
    await captureCheckpoint(runtime, scenario, "response_selected", savedSession);
  } finally {
    await runtime.close(savedSession);
  }

  const resumedRecord = runtime.record(scenario, "resume_selected");
  const resumedSession = await runtime.open(resumedRecord);
  try {
    await choosePersona(resumedSession.page, "Avery Thompson");
    await openStudentCourse(resumedSession.page);
    const attemptContextLoaded = resumedSession.page.waitForEvent("requestfinished", {
      predicate: (request) => {
        const url = new URL(request.url());
        return (
          url.origin === runtime.entryUrl.origin &&
          /^\/api\/assignment-attempts\/R-[1-9][0-9]{0,9}\/context$/u.test(url.pathname)
        );
      },
    });
    await resumeStudentAssignmentAttempt(resumedSession.page);
    await attemptContextLoaded;
    await resumedSession.page.getByText("Question 2 of 4", { exact: true }).waitFor();
    await attemptSurface(resumedSession)
      .locator("section.question-response-control")
      .getByRole("radio")
      .first()
      .waitFor();
    await resumedSession.privacy.settleResponses();
    await resumedSession.page.reload({ waitUntil: "commit" });
    await attemptSurface(resumedSession).waitFor();
    await attemptQuestion(resumedSession, "Question 1: Saved").waitFor();
    await attemptQuestion(resumedSession, "Question 1: Saved").click();
    await resumedSession.page.getByText("Question 1 of 4", { exact: true }).waitFor();
    await attemptSurface(resumedSession).locator("input[type='radio']:checked").waitFor();
    await resumedSession.page.getByText("Response saved.", { exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "resume_selected", resumedSession);
  } finally {
    await runtime.close(resumedSession);
  }

  const submittedRecord = runtime.record(scenario, "submitted");
  const submittedSession = await runtime.open(submittedRecord);
  try {
    await choosePersona(submittedSession.page, "Avery Thompson");
    await openStudentCourse(submittedSession.page);
    await resumeStudentAssignmentAttempt(submittedSession.page);
    await attemptQuestion(submittedSession, "Question 2: Not answered, current").waitFor();
    await saveCurrentRadioResponse(submittedSession);
    await attemptQuestion(submittedSession, "Question 2: Saved, current").waitFor();
    await attemptQuestion(submittedSession, "Question 3: Not answered").click();
    await attemptQuestion(submittedSession, "Question 3: Not answered, current").waitFor();
    await saveCurrentNumericResponse(submittedSession);
    await attemptQuestion(submittedSession, "Question 3: Saved, current").waitFor();
    await attemptQuestion(submittedSession, "Question 4: Not answered").click();
    await attemptQuestion(submittedSession, "Question 4: Not answered, current").waitFor();
    const responseControl = attemptSurface(submittedSession).locator(
      "section.question-response-control",
    );
    await responseControl.locator("input[type='checkbox']:visible").first().check();
    await responseControl.getByRole("button", { name: "Save response", exact: true }).click();
    await submittedSession.page.getByText("Response saved.", { exact: true }).waitFor();
    await attemptQuestion(submittedSession, "Question 4: Saved, current").waitFor();
    await submittedSession.page
      .getByRole("button", { name: "Submit Assignment", exact: true })
      .click();
    await submittedSession.page
      .getByRole("heading", { name: "Assignment submitted", exact: true })
      .waitFor();
    await captureCheckpoint(runtime, scenario, "submitted", submittedSession);
  } finally {
    await runtime.close(submittedSession);
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
    checkpoints: ["unanswered_laptop", "unanswered_tablet"],
    run: studentAssignmentOverviews,
  },
  {
    id: "student_assignment_history",
    checkpoints: ["overview_history", "selected_history"],
    run: studentAssignmentHistory,
  },
  {
    id: "student_assignment_attempt",
    checkpoints: ["response_selected", "resume_selected", "submitted"],
    run: studentAssignmentAttempt,
  },
  {
    id: "student_authorization",
    checkpoints: ["denial_laptop", "denial_phone"],
    run: studentAuthorization,
  },
];
