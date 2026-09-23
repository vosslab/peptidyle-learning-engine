// scenarios_student.ts - Student course, invitation, delivery, and denial survey.
// Selector contract: shared Course and Assessment actions live in visible_workflows.ts:47; Student
// Course page identity is owned by PageFrame in the Student Course landing and Grades pages;
// other Student headings and controls are owned by src/pages/student_courses_page.tsx:29,
// src/pages/student_course_invitation_page.tsx:42, src/pages/assessment_overview_page.tsx,
// src/pages/assessment_attempt_page.tsx, and src/pages/assessment_attempt_summary_page.tsx.

import type { Locator, Page } from "playwright";

import type { CaptureSession, ScenarioRuntime } from "./runtime";
import {
  directViewportCaptures,
  viewportCoverage,
  type ScenarioDefinition,
} from "./scenario_types";
import {
  ASSESSMENT_TYPE_LABEL,
  ASSIGNMENT_TITLE,
  COURSE_TITLE,
  choosePersona,
  courseCard,
  enterInstructor,
  openStudentCourse,
  openStudentCourseChooser,
  resumeStudentAssignmentAttempt,
  scrollTop,
  type SeededPersona,
} from "./visible_workflows";

const INVITATION_COURSE_SHORT_NAME = "Corpus Invite";
const INVITATION_COURSE_LONG_NAME = "Screenshot Corpus Invitation Course";

function courseworkRow(page: Page): Locator {
  return page.locator(".record-list__row").filter({
    has: page.getByRole("heading", { name: ASSIGNMENT_TITLE, exact: true }),
  });
}

async function openStudentAssignment(page: Page): Promise<void> {
  const coursework = courseworkRow(page);
  await coursework.waitFor();
  await coursework
    .getByRole("link", { name: /^(Open|Resume|Review) Practice Question Assignment$/u })
    .click();
  await page.locator('[data-route-surface="assessmentOverview"]').waitFor();
}

async function captureCheckpoint(
  runtime: ScenarioRuntime,
  checkpoint: string,
  session: CaptureSession,
  top = true,
): Promise<void> {
  if (top) await scrollTop(session.page);
  await runtime.captureCheckpoint(session, checkpoint);
}

async function studentCourseList(runtime: ScenarioRuntime): Promise<void> {
  for (const checkpoint of [
    "course_list_laptop",
    "course_list_tablet",
    "course_list_phone",
    "course_list_square",
  ] as const) {
    const session = await runtime.open(checkpoint);
    try {
      await choosePersona(session.page, "Mary Okafor");
      await openStudentCourseChooser(session.page);
      await courseCard(session.page).waitFor();
      await captureCheckpoint(runtime, checkpoint, session);
    } finally {
      await runtime.close(session);
    }
  }
}

async function studentCourseGrades(runtime: ScenarioRuntime): Promise<void> {
  for (const checkpoint of [
    "course_grades_laptop",
    "course_grades_tablet",
    "course_grades_phone",
    "course_grades_square",
  ] as const) {
    const session = await runtime.open(checkpoint);
    try {
      await choosePersona(session.page, "Mary Okafor");
      await openStudentCourse(session.page);
      await session.page
        .getByRole("navigation", { name: "Ribbon tabs", exact: true })
        .getByRole("link", { name: "Grades", exact: true })
        .click();
      await session.page.locator('[data-route-surface="studentCourseGrades"]').waitFor();
      await session.page
        .getByRole("heading", { level: 1, name: COURSE_TITLE, exact: true })
        .waitFor();
      await session.page
        .getByText("Loading grades...", { exact: true })
        .waitFor({ state: "hidden" });
      await session.page
        .locator("dl")
        .or(session.page.getByText("No Coursework has been graded yet.", { exact: true }))
        .waitFor();
      await captureCheckpoint(runtime, checkpoint, session);
    } finally {
      await runtime.close(session);
    }
  }
}

async function prepareStudentInvitation(
  runtime: ScenarioRuntime,
  checkpoint: string,
): Promise<void> {
  const setup = await runtime.open(checkpoint);
  const page = setup.page;
  try {
    await enterInstructor(page);
    await page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await page.getByRole("heading", { name: "My Active Courses", exact: true }).waitFor();
    // Create Course Instance is a disclosure that auto-expands only for an empty list; the
    // seeded Instructor already teaches one Course, so expand it when still collapsed.
    const createDisclosure = page.getByRole("button", {
      name: "Create Course Instance",
      exact: true,
    });
    await page.getByText("Loading Course Instances...").waitFor({ state: "hidden" });
    // Replays on the same stack reuse the invitation Course and its pending invitation.
    const created = courseCard(page, INVITATION_COURSE_LONG_NAME);
    if ((await created.count()) === 0) {
      if ((await createDisclosure.getAttribute("aria-expanded")) !== "true") {
        await createDisclosure.click();
      }
      const createForm = page.locator("form#create-course-instance");
      await createForm.waitFor();
      await page.getByRole("combobox", { name: "Start with", exact: true }).selectOption("adopted");
      await page
        .getByRole("combobox", { name: "Blueprint Course", exact: true })
        .selectOption({ label: `${COURSE_TITLE} · Revision 1` });
      await page
        .getByRole("combobox", { name: "Discipline (required)", exact: true })
        .selectOption({ label: "Biology" });
      await page.getByLabel("Course short name").fill(INVITATION_COURSE_SHORT_NAME);
      await page.getByLabel("Course long name").fill(INVITATION_COURSE_LONG_NAME);
      await page.getByLabel("Course Term start date").fill("2026-09-01");
      await page.getByLabel("Course Term end date").fill("2026-12-18");
      await createForm.getByRole("button", { name: "Create Course Instance", exact: true }).click();
      await created.waitFor();
    }
    await created.getByRole("link", { name: "Open Course Instance", exact: true }).click();
    await page.getByRole("link", { name: "Open Students", exact: true }).click();
    await page.getByRole("heading", { name: "Students", exact: true }).waitFor();
    await page.getByText("Loading", { exact: false }).first().waitFor({ state: "hidden" });
    if ((await page.getByText("screenshot-invitation", { exact: true }).count()) === 0) {
      await page.getByText("Roster tools", { exact: true }).click();
      await page
        .getByLabel("Email, roster ID, Course roster name")
        .fill(
          "mary.okafor@biology.roosevelt.edu,screenshot-invitation,Synthetic Invitation Student",
        );
      await page.getByRole("button", { name: "Import roster", exact: true }).click();
      await page.getByText("screenshot-invitation", { exact: true }).waitFor();
    }
  } finally {
    await runtime.close(setup);
  }
}

async function studentInvitation(runtime: ScenarioRuntime): Promise<void> {
  await prepareStudentInvitation(runtime, "invitation_index_laptop");
  for (const viewport of ["laptop", "tablet", "phone", "square"] as const) {
    const session = await runtime.open(`invitation_index_${viewport}`);
    const page = session.page;
    try {
      await choosePersona(page, "Mary Okafor");
      await openStudentCourseChooser(page);
      await page.getByRole("link", { name: "Course invitations", exact: true }).click();
      await page.getByRole("heading", { name: "Course invitations", exact: true }).waitFor();
      const invitation = courseCard(page, INVITATION_COURSE_LONG_NAME);
      await invitation.waitFor();
      await captureCheckpoint(runtime, `invitation_index_${viewport}`, session);
      await invitation.getByRole("link", { name: "Review invitation", exact: true }).click();
      await page
        .getByRole("heading", { level: 2, name: INVITATION_COURSE_LONG_NAME, exact: true })
        .waitFor();
      await page.getByRole("button", { name: "Accept invitation", exact: true }).waitFor();
      // The invited Student never joins, so this scenario stays stable across replays.
      await captureCheckpoint(runtime, `invitation_detail_${viewport}`, session);
    } finally {
      await runtime.close(session);
    }
  }
}

type StudentPersona = Extract<SeededPersona, "Mary Okafor" | "Jack Nguyen" | "Avery Thompson">;

async function captureLanding(
  runtime: ScenarioRuntime,
  checkpoint: string,
  persona: StudentPersona,
): Promise<void> {
  const session = await runtime.open(checkpoint);
  try {
    await choosePersona(session.page, persona);
    await openStudentCourse(session.page);
    // The named Coursework row appears only after the landing resource has settled. The separate
    // Assignment-overview workflow covers action navigation.
    await courseworkRow(session.page).waitFor();
    await captureCheckpoint(runtime, checkpoint, session);
  } finally {
    await runtime.close(session);
  }
}

async function studentLandings(runtime: ScenarioRuntime): Promise<void> {
  for (const [checkpointStem, persona] of [
    ["not_started", "Avery Thompson"],
    ["in_progress", "Jack Nguyen"],
    ["completed", "Mary Okafor"],
  ] as const) {
    for (const viewport of ["laptop", "tablet", "phone", "square"] as const) {
      await captureLanding(runtime, `${checkpointStem}_${viewport}`, persona);
    }
  }
}

async function captureAssignmentOverview(
  runtime: ScenarioRuntime,
  checkpoint: string,
  persona: StudentPersona,
): Promise<void> {
  const session = await runtime.open(checkpoint);
  try {
    await choosePersona(session.page, persona);
    await openStudentCourse(session.page);
    await openStudentAssignment(session.page);
    await session.page.getByRole("heading", { level: 1, name: ASSIGNMENT_TITLE }).waitFor();
    await captureCheckpoint(runtime, checkpoint, session);
  } finally {
    await runtime.close(session);
  }
}

async function studentAssignmentOverviews(runtime: ScenarioRuntime): Promise<void> {
  for (const viewport of ["laptop", "tablet", "phone", "square"] as const) {
    await captureAssignmentOverview(runtime, `unanswered_${viewport}`, "Avery Thompson");
  }
}

async function studentAssignmentHistory(runtime: ScenarioRuntime): Promise<void> {
  for (const viewport of ["laptop", "tablet", "phone", "square"] as const) {
    const session = await runtime.open(`overview_history_${viewport}`);
    try {
      await choosePersona(session.page, "Mary Okafor");
      await openStudentCourse(session.page);
      const assignment = courseworkRow(session.page);
      await assignment
        .getByRole("link", { name: `Review ${ASSESSMENT_TYPE_LABEL}`, exact: true })
        .click();
      await session.page.locator('[data-route-surface="assessmentOverview"]').waitFor();
      await session.page.getByRole("heading", { name: "Previous attempts", exact: true }).waitFor();
      const previousAttempt = session.page.getByRole("link", { name: "Attempt 1", exact: true });
      await previousAttempt.waitFor();
      await captureCheckpoint(runtime, `overview_history_${viewport}`, session);
      await previousAttempt.click();
      await session.page.locator('[data-route-surface="assessmentAttemptSummary"]').waitFor();
      await session.page
        .getByRole("heading", { name: "Your recorded work", exact: true })
        .waitFor();
      await captureCheckpoint(runtime, `selected_history_${viewport}`, session);
    } finally {
      await runtime.close(session);
    }
  }
}

function attemptSurface(session: CaptureSession): Locator {
  return session.page.locator('[data-route-surface="assessmentAttempt"]');
}

function attemptQuestion(session: CaptureSession, name: string): Locator {
  return session.page
    .getByRole("navigation", { name: "Assessment questions", exact: true })
    .getByRole("button", { name, exact: true });
}

async function saveCurrentResponse(session: CaptureSession): Promise<void> {
  const responseControl = attemptSurface(session).locator("section.question-response-control");
  const matchingSlots = responseControl.locator(".matching-slot[data-prompt-id]:not(:disabled)");
  const nativeChoices = responseControl.locator('input[type="radio"], input[type="checkbox"]');
  await responseControl
    .locator(
      '.matching-slot[data-prompt-id]:not(:disabled), input[type="radio"], input[type="checkbox"]',
    )
    .first()
    .waitFor();
  if ((await matchingSlots.count()) > 0) {
    const slotCount = await matchingSlots.count();
    for (let index = 0; index < slotCount; index += 1) {
      const choice = responseControl
        .locator(".matching-bank button[data-choice-id]:not(:disabled)")
        .first();
      const slot = matchingSlots.nth(index);
      await choice.click();
      await slot.click();
      await slot
        .getByText("Assign selected choice", { exact: true })
        .waitFor({ state: "detached" });
    }
  } else {
    await nativeChoices.first().check();
  }
  await responseControl.getByRole("button", { name: "Save response", exact: true }).click();
  await session.page.getByText("Response saved.", { exact: true }).waitFor();
}

async function waitForRestoredResponse(session: CaptureSession): Promise<void> {
  const responseControl = attemptSurface(session).locator("section.question-response-control");
  const matchingSlots = responseControl.locator(".matching-slot[data-prompt-id]");
  if ((await matchingSlots.count()) > 0) {
    await matchingSlots
      .first()
      .getByText("Assign selected choice", { exact: true })
      .waitFor({ state: "detached" });
    return;
  }
  await responseControl
    .locator('input[type="radio"]:checked, input[type="checkbox"]:checked')
    .first()
    .waitFor();
}

async function studentAssignmentAttempt(runtime: ScenarioRuntime): Promise<void> {
  const savedSession = await runtime.open("response_selected_laptop");
  try {
    await choosePersona(savedSession.page, "Avery Thompson");
    await openStudentCourse(savedSession.page);
    await openStudentAssignment(savedSession.page);
    await savedSession.page
      .getByRole("button", { name: `Start ${ASSESSMENT_TYPE_LABEL}`, exact: true })
      .click();
    await attemptSurface(savedSession).waitFor();
    await savedSession.page.getByText("Question 1 of 4", { exact: true }).waitFor();
    await saveCurrentResponse(savedSession);
    await attemptQuestion(savedSession, "Question 1: Saved, current").waitFor();
    await captureCheckpoint(runtime, "response_selected_laptop", savedSession);
    await attemptQuestion(savedSession, "Question 2: Not answered").click();
    await attemptQuestion(savedSession, "Question 2: Not answered, current").waitFor();
    await savedSession.page.getByText("Question 2 of 4", { exact: true }).waitFor();
    await captureCheckpoint(runtime, "assessment_navigation_laptop", savedSession);
  } finally {
    await runtime.close(savedSession);
  }

  for (const viewport of ["tablet", "phone", "square"] as const) {
    const session = await runtime.open(`response_selected_${viewport}`);
    try {
      await choosePersona(session.page, "Avery Thompson");
      await openStudentCourse(session.page);
      await resumeStudentAssignmentAttempt(session.page);
      await attemptQuestion(session, "Question 1: Saved").click();
      await session.page.getByText("Question 1 of 4", { exact: true }).waitFor();
      await waitForRestoredResponse(session);
      await captureCheckpoint(runtime, `response_selected_${viewport}`, session);
      await attemptQuestion(session, "Question 2: Not answered").click();
      await attemptQuestion(session, "Question 2: Not answered, current").waitFor();
      await session.page.getByText("Question 2 of 4", { exact: true }).waitFor();
      await captureCheckpoint(runtime, `assessment_navigation_${viewport}`, session);
    } finally {
      await runtime.close(session);
    }
  }

  for (const viewport of ["laptop", "tablet", "phone", "square"] as const) {
    const resumedSession = await runtime.open(`resume_selected_${viewport}`);
    try {
      await choosePersona(resumedSession.page, "Avery Thompson");
      await openStudentCourse(resumedSession.page);
      await resumeStudentAssignmentAttempt(resumedSession.page);
      await resumedSession.page.getByText("Question 2 of 4", { exact: true }).waitFor();
      await attemptSurface(resumedSession).locator("section.question-response-control").waitFor();
      await resumedSession.privacy.settleResponses();
      await resumedSession.page.reload({ waitUntil: "commit" });
      await attemptSurface(resumedSession).waitFor();
      await attemptQuestion(resumedSession, "Question 1: Saved").waitFor();
      await attemptQuestion(resumedSession, "Question 1: Saved").click();
      await resumedSession.page.getByText("Question 1 of 4", { exact: true }).waitFor();
      await waitForRestoredResponse(resumedSession);
      await resumedSession.page.getByText("Response saved.", { exact: true }).waitFor();
      await captureCheckpoint(runtime, `resume_selected_${viewport}`, resumedSession);
    } finally {
      await runtime.close(resumedSession);
    }
  }

  const submittedSession = await runtime.open("submitted_laptop");
  try {
    await choosePersona(submittedSession.page, "Avery Thompson");
    await openStudentCourse(submittedSession.page);
    await resumeStudentAssignmentAttempt(submittedSession.page);
    await attemptQuestion(submittedSession, "Question 2: Not answered, current").waitFor();
    await saveCurrentResponse(submittedSession);
    await attemptQuestion(submittedSession, "Question 2: Saved, current").waitFor();
    await attemptQuestion(submittedSession, "Question 3: Not answered").click();
    await attemptQuestion(submittedSession, "Question 3: Not answered, current").waitFor();
    await saveCurrentResponse(submittedSession);
    await attemptQuestion(submittedSession, "Question 3: Saved, current").waitFor();
    await attemptQuestion(submittedSession, "Question 4: Not answered").click();
    await attemptQuestion(submittedSession, "Question 4: Not answered, current").waitFor();
    await saveCurrentResponse(submittedSession);
    await attemptQuestion(submittedSession, "Question 4: Saved, current").waitFor();
    await submittedSession.page
      .getByRole("button", { name: "Submit Attempt", exact: true })
      .click();
    await submittedSession.page
      .locator('[data-route-surface="assessmentAttemptSummary"]')
      .waitFor();
    await submittedSession.page
      .getByRole("heading", { name: "Your recorded work", exact: true })
      .waitFor();
    await captureCheckpoint(runtime, "submitted_laptop", submittedSession);
  } finally {
    await runtime.close(submittedSession);
  }

  for (const viewport of ["tablet", "phone", "square"] as const) {
    const session = await runtime.open(`submitted_${viewport}`);
    try {
      await choosePersona(session.page, "Avery Thompson");
      await openStudentCourse(session.page);
      const assignment = courseworkRow(session.page);
      await assignment
        .getByRole("link", { name: `Review ${ASSESSMENT_TYPE_LABEL}`, exact: true })
        .click();
      await session.page.locator('[data-route-surface="assessmentOverview"]').waitFor();
      const previousAttempt = session.page.getByRole("link", { name: "Attempt 1", exact: true });
      await previousAttempt.waitFor();
      await previousAttempt.click();
      await session.page.locator('[data-route-surface="assessmentAttemptSummary"]').waitFor();
      await session.page
        .getByRole("heading", { name: "Your recorded work", exact: true })
        .waitFor();
      await captureCheckpoint(runtime, `submitted_${viewport}`, session);
    } finally {
      await runtime.close(session);
    }
  }
}

async function captureDenial(runtime: ScenarioRuntime, checkpoint: string): Promise<void> {
  const session = await runtime.open(checkpoint);
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
    await captureCheckpoint(runtime, checkpoint, session);
  } finally {
    await runtime.close(session);
  }
}

async function studentAuthorization(runtime: ScenarioRuntime): Promise<void> {
  for (const viewport of ["laptop", "tablet", "phone", "square"] as const) {
    await captureDenial(runtime, `denial_${viewport}`);
  }
}

export const STUDENT_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "student_course_list",
    role: "student",
    captures: [
      ...directViewportCaptures({
        checkpoint: "course_list_laptop",
        area: "courses",
        workflow: "course navigation",
        state: "course list",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student courses",
        featured: true,
      }),
    ],
    viewportCoverage: viewportCoverage(["laptop", "tablet", "phone", "square"], {}),
    run: studentCourseList,
  },
  {
    id: "student_course_grades",
    role: "student",
    captures: [
      ...directViewportCaptures({
        checkpoint: "course_grades_laptop",
        area: "courses",
        workflow: "Course grade summary",
        state: "self-only grades",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Course grades",
      }),
    ],
    viewportCoverage: viewportCoverage(["laptop", "tablet", "phone", "square"], {}),
    run: studentCourseGrades,
  },
  {
    id: "student_invitation",
    role: "student",
    captures: [
      ...directViewportCaptures({
        checkpoint: "invitation_index_laptop",
        area: "courses",
        workflow: "course invitation",
        state: "pending index",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Pending Course Invitations",
      }),
      ...directViewportCaptures({
        checkpoint: "invitation_detail_laptop",
        area: "courses",
        workflow: "course invitation",
        state: "review",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Course Invitation review",
      }),
    ],
    viewportCoverage: viewportCoverage(["laptop", "tablet", "phone", "square"], {}),
    run: studentInvitation,
  },
  {
    id: "student_landings",
    role: "student",
    captures: [
      ...directViewportCaptures({
        checkpoint: "not_started_laptop",
        area: "courses",
        workflow: "seeded assignment progress",
        state: "not started",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Not-started Course landing",
      }),
      ...directViewportCaptures({
        checkpoint: "in_progress_laptop",
        area: "courses",
        workflow: "seeded assignment progress",
        state: "in progress",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "In-progress Course landing",
      }),
      ...directViewportCaptures({
        checkpoint: "completed_laptop",
        area: "courses",
        workflow: "seeded assignment progress",
        state: "completed",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Completed Course landing",
        featured: true,
      }),
    ],
    viewportCoverage: viewportCoverage(["laptop", "tablet", "phone", "square"], {}),
    run: studentLandings,
  },
  {
    id: "student_assignment_overviews",
    role: "student",
    captures: [
      ...directViewportCaptures({
        checkpoint: "unanswered_laptop",
        area: "assignments",
        workflow: "assignment entry",
        state: "unanswered",
        viewport: "laptop",
        privacyProfile: "student_unanswered",
        caption: "Unanswered Assignment overview",
        featured: true,
      }),
    ],
    viewportCoverage: viewportCoverage(["laptop", "tablet", "phone", "square"], {}),
    run: studentAssignmentOverviews,
  },
  {
    id: "student_assignment_history",
    role: "student",
    captures: [
      ...directViewportCaptures({
        checkpoint: "overview_history_laptop",
        area: "assignments",
        workflow: "assignment history",
        state: "previous attempts",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Assignment overview with previous attempts",
      }),
      ...directViewportCaptures({
        checkpoint: "selected_history_laptop",
        area: "assignments",
        workflow: "assignment history",
        state: "selected previous attempt",
        viewport: "laptop",
        privacyProfile: "student_feedback_released",
        caption: "Selected Assignment history",
      }),
    ],
    viewportCoverage: viewportCoverage(["laptop", "tablet", "phone", "square"], {}),
    run: studentAssignmentHistory,
  },
  {
    id: "student_assignment_attempt",
    role: "student",
    captures: [
      ...directViewportCaptures({
        checkpoint: "response_selected_laptop",
        area: "assignments",
        workflow: "assignment attempt",
        state: "saved response",
        viewport: "laptop",
        privacyProfile: "student_selected_response",
        caption: "Saved Student Assignment response",
        featured: true,
      }),
      ...directViewportCaptures({
        checkpoint: "assessment_navigation_laptop",
        area: "assessments",
        workflow: "assessment navigation and progress",
        state: "Question 2 current with one saved response",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Question navigation, saved progress, and countdown timer",
      }),
      ...directViewportCaptures({
        checkpoint: "resume_selected_laptop",
        area: "assignments",
        workflow: "assignment attempt",
        state: "reloaded saved response",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Reloaded Student Assignment response",
      }),
      ...directViewportCaptures({
        checkpoint: "submitted_laptop",
        area: "assignments",
        workflow: "assignment attempt",
        state: "submitted",
        viewport: "laptop",
        privacyProfile: "student_feedback_released",
        caption: "Submitted Student Assignment",
      }),
    ],
    viewportCoverage: viewportCoverage(["laptop", "tablet", "phone", "square"], {}),
    run: studentAssignmentAttempt,
  },
  {
    id: "student_authorization",
    role: "student",
    captures: [
      ...directViewportCaptures({
        checkpoint: "denial_laptop",
        area: "authorization",
        workflow: "role denial",
        state: "denied",
        viewport: "laptop",
        privacyProfile: "authorization_denial",
        caption: "Student authorization denial",
      }),
    ],
    viewportCoverage: viewportCoverage(["laptop", "tablet", "phone", "square"], {}),
    run: studentAuthorization,
  },
];
