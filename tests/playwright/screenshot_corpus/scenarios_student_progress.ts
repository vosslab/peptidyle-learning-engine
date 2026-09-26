// Student Progress, Coursework views, Response Stats, Attempt History, and Latest Feedback captures.

import type { Locator, Page } from "playwright";

import type { CaptureSession, ScenarioRuntime } from "./runtime";
import { catalogScreenshotFilename } from "./filenames";
import {
  directViewportCaptures,
  viewportCoverage,
  type ScenarioDefinition,
} from "./scenario_types";
import {
  ASSESSMENT_ENTRY_BUTTON,
  ASSIGNMENT_TITLE,
  choosePersona,
  openAllStudentCoursework,
  openStudentAssignment,
  openStudentCourse,
  scrollTop,
} from "./visible_workflows";

const PILOT_COURSE_SHORT_NAME = "BCHM 301";

async function captureCheckpoint(
  runtime: ScenarioRuntime,
  checkpoint: string,
  session: CaptureSession,
  top = true,
): Promise<void> {
  if (top) await scrollTop(session.page);
  await runtime.captureCheckpoint(session, checkpoint);
}

async function openCourseAttemptHistory(page: Page): Promise<void> {
  const tabs = page.getByRole("navigation", { name: "Ribbon tabs", exact: true });
  await tabs.getByRole("link", { name: "Grades", exact: true }).click();
  await page.getByRole("link", { name: "Attempt History", exact: true }).click();
  await page.locator('[data-route-surface="studentAttemptHistory"]').waitFor();
  await waitForCourseSections(
    page,
    ".student-course-attempt-history",
    ".record-collection__state--loading",
  );
}

async function openCourseProgress(page: Page): Promise<void> {
  await openStudentCourse(page);
  await page.getByRole("link", { name: "Course Progress", exact: true }).click();
  await page.locator('[data-route-surface="studentCourseProgress"]').waitFor();
}

async function waitForCourseProgress(page: Page): Promise<void> {
  await page.locator(".student-course-progress__completion").waitFor();
}

async function waitForCourseworkSections(page: Page): Promise<void> {
  await waitForCourseSections(
    page,
    "section.student-coursework",
    ".record-collection__state--loading",
  );
}

async function waitForCourseSections(
  page: Page,
  sectionSelector: string,
  loadingSelector: string,
): Promise<void> {
  const sections = page.locator(sectionSelector);
  await sections.first().waitFor();
  await Promise.all(
    Array.from({ length: await sections.count() }, (_, index) =>
      sections.nth(index).locator(loadingSelector).waitFor({ state: "hidden" }),
    ),
  );
}

function attemptHistoryForPilotAssessment(page: Page): Locator {
  return pageCourseAttemptHistorySection(page)
    .getByRole("list", { name: `${PILOT_COURSE_SHORT_NAME} Attempts`, exact: true })
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: ASSIGNMENT_TITLE, exact: true }) });
}

function pageCourseAttemptHistorySection(page: Page): Locator {
  return page.getByRole("region", {
    name: `${PILOT_COURSE_SHORT_NAME} Attempt History`,
    exact: true,
  });
}

function submittedAttemptHistoryForPilotAssessment(page: Page): Locator {
  return attemptHistoryForPilotAssessment(page).filter({ hasText: /· Submitted/u });
}

async function nextDurationCheckpoint(
  page: Page,
  position: number,
): Promise<import("playwright").Response> {
  return page.waitForResponse((response) => {
    const request = response.request();
    return (
      request.method() === "PUT" &&
      new URL(response.url()).pathname.endsWith(`/questions/${position}/display-duration`)
    );
  });
}

async function verifyQuestionDurationLifecycle(page: Page): Promise<void> {
  await page.waitForTimeout(1_250);

  const hiddenCheckpoint = nextDurationCheckpoint(page, 1);
  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", { configurable: true, value: "hidden" });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  const hiddenReceipt = (await (await hiddenCheckpoint).json()) as {
    cumulativeDisplayDurationMs: number;
  };
  if (
    !Number.isSafeInteger(hiddenReceipt.cumulativeDisplayDurationMs) ||
    hiddenReceipt.cumulativeDisplayDurationMs <= 0
  ) {
    throw new Error("A hidden Question checkpoint did not return its accumulated display time.");
  }

  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", { configurable: true, value: "visible" });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await page.waitForTimeout(250);
  const transitionCheckpoint = nextDurationCheckpoint(page, 1);
  await page
    .getByRole("navigation", { name: "Assessment questions", exact: true })
    .getByRole("button", { name: "Question 2: Not answered", exact: true })
    .click();
  const transitionReceipt = (await (await transitionCheckpoint).json()) as {
    cumulativeDisplayDurationMs: number;
  };
  await page.getByText("Question 2 of 4", { exact: true }).waitFor();

  const progressResponsePromise = page.waitForResponse(
    (response) =>
      response.request().method() === "GET" &&
      new URL(response.url()).pathname.endsWith("/student-progress"),
  );
  await page.reload({ waitUntil: "commit" });
  const progressResponse = await progressResponsePromise;
  const progress = (await progressResponse.json()) as {
    positions: ReadonlyArray<{ position: number; displayDurationMs: number | null }>;
  };
  const resumedMilliseconds = progress.positions.find(
    (item) => item.position === 1,
  )?.displayDurationMs;
  if (
    resumedMilliseconds === null ||
    resumedMilliseconds === undefined ||
    resumedMilliseconds < transitionReceipt.cumulativeDisplayDurationMs
  ) {
    throw new Error("Reload did not resume from the Question's saved cumulative display time.");
  }

  await page.locator('[data-route-surface="assessmentAttempt"]').waitFor();
  await page
    .getByRole("navigation", { name: "Assessment questions", exact: true })
    .getByRole("button", { name: /^Question 1: Not answered/u })
    .click();
  await page.getByText("Question 1 of 4", { exact: true }).waitFor();
  await page.waitForTimeout(250);
  const submissionCheckpoint = nextDurationCheckpoint(page, 1);
  await page.getByRole("button", { name: "Submit Attempt", exact: true }).click();
  const submissionReceipt = (await (await submissionCheckpoint).json()) as {
    cumulativeDisplayDurationMs: number;
  };
  if (submissionReceipt.cumulativeDisplayDurationMs < resumedMilliseconds) {
    throw new Error("Submission reduced the saved Question display duration.");
  }
  await page.locator('[data-route-surface="assessmentAttemptSummary"]').waitFor();
}

async function createUnansweredAttempt(page: Page, captureDurationSample: boolean): Promise<void> {
  const tabs = page.getByRole("navigation", { name: "Ribbon tabs", exact: true });
  await tabs.getByRole("link", { name: "Courses", exact: true }).click();
  await openStudentCourse(page);
  await openAllStudentCoursework(page);
  await openStudentAssignment(page);
  await page.getByRole("button", { name: ASSESSMENT_ENTRY_BUTTON }).click();
  await page.locator('[data-route-surface="assessmentAttempt"]').waitFor();
  await page.getByText("Question 1 of 4", { exact: true }).waitFor();
  if (captureDurationSample) {
    await verifyQuestionDurationLifecycle(page);
  } else {
    await page.getByRole("button", { name: "Submit Attempt", exact: true }).click();
    await page.locator('[data-route-surface="assessmentAttemptSummary"]').waitFor();
  }
}

async function studentProgressAndStats(runtime: ScenarioRuntime): Promise<void> {
  const setup = await runtime.open("course_progress_laptop");
  try {
    const page = setup.page;
    await choosePersona(page, "Mary Okafor");
    await openCourseProgress(page);
    await waitForCourseProgress(page);
    await createUnansweredAttempt(page, true);
  } finally {
    await runtime.close(setup);
  }

  for (const viewport of ["laptop", "tablet", "phone", "square"] as const) {
    const session = await runtime.open(`course_progress_${viewport}`);
    try {
      const page = session.page;
      await choosePersona(page, "Mary Okafor");
      await openCourseProgress(page);
      await waitForCourseProgress(page);
      await page
        .getByRole("list", { name: "Coursework progress", exact: true })
        .getByRole("listitem")
        .first()
        .getByText(/^Latest activity:/u)
        .waitFor();
      await captureCheckpoint(runtime, `course_progress_${viewport}`, session);

      const tabs = page.getByRole("navigation", { name: "Ribbon tabs", exact: true });
      await tabs.getByRole("link", { name: "Coursework", exact: true }).click();
      await page.getByRole("link", { name: "Due Soon", exact: true }).click();
      await page.locator('[data-route-surface="studentDueSoon"]').waitFor();
      await waitForCourseworkSections(page);
      await captureCheckpoint(runtime, `course_due_soon_${viewport}`, session);

      await page.getByRole("link", { name: "Completed", exact: true }).click();
      await page.locator('[data-route-surface="studentCompleted"]').waitFor();
      await waitForCourseworkSections(page);
      await page.locator(".record-list__row").first().waitFor();
      await captureCheckpoint(runtime, `course_completed_${viewport}`, session);

      await tabs.getByRole("link", { name: "Courses", exact: true }).click();
      await openCourseProgress(page);

      await tabs.getByRole("link", { name: "Grades", exact: true }).click();
      await page.getByRole("link", { name: "Response Stats", exact: true }).click();
      await page.locator('[data-route-surface="studentResponseStats"]').waitFor();
      await waitForCourseSections(
        page,
        ".student-course-response-stats",
        ".record-collection__state--loading",
      );
      await page
        .getByRole("list", { name: /Question outcomes$/u })
        .getByRole("listitem")
        .first()
        .waitFor();
      await captureCheckpoint(runtime, `response_stats_${viewport}`, session);
    } finally {
      await runtime.close(session);
    }
  }
}

async function captureAttemptHistoryAndFeedback(
  runtime: ScenarioRuntime,
  viewport: "laptop" | "tablet" | "phone" | "square",
  seedHistory: boolean,
): Promise<void> {
  const session = await runtime.open(`course_attempt_history_${viewport}`);
  try {
    const page = session.page;
    await choosePersona(page, "Mary Okafor");
    await openStudentCourse(page);
    await openCourseAttemptHistory(page);
    let submittedAttemptCount = await submittedAttemptHistoryForPilotAssessment(page).count();
    for (
      let createdAttemptCount = 0;
      seedHistory && submittedAttemptCount < 40 && createdAttemptCount < 40;
      createdAttemptCount += 1
    ) {
      const previousAttemptCount = submittedAttemptCount;
      await createUnansweredAttempt(page, false);
      await openCourseAttemptHistory(page);
      submittedAttemptCount = await submittedAttemptHistoryForPilotAssessment(page).count();
      if (submittedAttemptCount <= previousAttemptCount) {
        throw new Error(
          `Course Attempt History stayed at ${submittedAttemptCount} submitted Attempts after a new submission. ` +
            `History text: ${await pageCourseAttemptHistorySection(page).innerText()}`,
        );
      }
    }
    if (submittedAttemptCount < 40) {
      throw new Error(
        `Expected at least 40 submitted Course Attempts, found ${submittedAttemptCount}.`,
      );
    }
    await attemptHistoryForPilotAssessment(page)
      .first()
      .getByText(/^Started/u)
      .waitFor();
    await page.locator('[data-ribbon-control-id="studentLatestFeedback"][href]').waitFor();
    await captureCheckpoint(runtime, `course_attempt_history_${viewport}`, session);

    await page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await openStudentCourse(page);
    await page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Grades", exact: true })
      .click();
    await page.getByRole("link", { name: "Response Stats", exact: true }).click();
    await page.locator('[data-route-surface="studentResponseStats"]').waitFor();
    await waitForCourseSections(page, ".student-course-response-stats", ".loading-state");
    const latestFeedbackResponsePromise = page.waitForResponse((response) => {
      return new URL(response.url()).pathname === "/api/student/latest-feedback";
    });
    await openCourseAttemptHistory(page);
    const latestFeedbackResponse = await latestFeedbackResponsePromise;
    const latestFeedback = (await latestFeedbackResponse.json()) as {
      readonly assessmentAttemptId?: string | null;
    };
    if (
      latestFeedbackResponse.status() !== 200 ||
      latestFeedback.assessmentAttemptId === null ||
      latestFeedback.assessmentAttemptId === undefined
    ) {
      throw new Error(
        `Expected a released latest-feedback Attempt after Course History showed 40 released scores; ` +
          `API returned ${latestFeedbackResponse.status()} with Attempt ` +
          `${latestFeedback.assessmentAttemptId ?? "none"}.`,
      );
    }
    submittedAttemptCount = await submittedAttemptHistoryForPilotAssessment(page).count();
    if (submittedAttemptCount < 40) {
      throw new Error(
        `Course History lost submitted Attempts after navigating through Response Stats; ` +
          `found ${submittedAttemptCount}. URL: ${page.url()}; ` +
          `history text: ${await pageCourseAttemptHistorySection(page).innerText()}`,
      );
    }
    const latestFeedbackLink = page.locator(
      '[data-ribbon-control-id="studentLatestFeedback"][href]',
    );
    await latestFeedbackLink.waitFor();
    const latestFeedbackHref = await latestFeedbackLink.getAttribute("href");
    if (latestFeedbackHref === null) throw new Error("Latest Feedback did not provide a target.");
    const expectedLatestFeedbackPath = new URL(latestFeedbackHref, page.url()).pathname;
    if (!expectedLatestFeedbackPath.includes(latestFeedback.assessmentAttemptId)) {
      throw new Error("Latest Feedback target did not match the authorized Attempt from the API.");
    }
    await latestFeedbackLink.click();
    await page.locator('[data-route-surface="assessmentAttemptSummary"]').waitFor();
    await page.getByRole("heading", { name: "Your recorded work", exact: true }).waitFor();
    if (new URL(page.url()).pathname !== expectedLatestFeedbackPath) {
      throw new Error("Latest Feedback did not open the Attempt selected by its shortcut.");
    }
    await captureCheckpoint(runtime, `latest_feedback_${viewport}`, session);
  } finally {
    await runtime.close(session);
  }
}

async function studentAttemptHistory(runtime: ScenarioRuntime): Promise<void> {
  const viewports = ["laptop", "tablet", "phone", "square"] as const;
  for (const [index, viewport] of viewports.entries()) {
    await captureAttemptHistoryAndFeedback(runtime, viewport, index === 0);
  }
}

async function studentProgressWithoutReleasedScore(runtime: ScenarioRuntime): Promise<void> {
  for (const viewport of ["laptop", "tablet", "phone", "square"] as const) {
    const session = await runtime.open(`course_progress_unreleased_${viewport}`);
    try {
      await choosePersona(session.page, "Jack Nguyen");
      await openCourseProgress(session.page);
      await session.page.getByText("Score not released", { exact: true }).waitFor();
      await captureCheckpoint(runtime, `course_progress_unreleased_${viewport}`, session);
    } finally {
      await runtime.close(session);
    }
  }
}

export const STUDENT_PROGRESS_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "student_progress_response_stats_history",
    role: "student",
    captures: [
      ...directViewportCaptures({
        checkpoint: "course_progress_laptop",
        area: "courses",
        workflow: "Course Progress",
        state: "completed Attempts and score status",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Course Progress",
        featured: true,
      }),
      ...directViewportCaptures({
        checkpoint: "course_due_soon_laptop",
        filenameStem: catalogScreenshotFilename("coursework", "dueSoon", "course_due_soon"),
        area: "coursework",
        workflow: "Student Due Soon Coursework",
        state: "server-bounded next seven days",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Coursework Due Soon",
      }),
      ...directViewportCaptures({
        checkpoint: "course_completed_laptop",
        filenameStem: catalogScreenshotFilename(
          "coursework",
          "completedCoursework",
          "course_completed",
        ),
        area: "coursework",
        workflow: "Student Completed Coursework",
        state: "submitted Attempts",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Completed Coursework",
      }),
      ...directViewportCaptures({
        checkpoint: "response_stats_laptop",
        filenameStem: catalogScreenshotFilename("grades", "studentResponseStats", "response_stats"),
        area: "grades",
        workflow: "Response Stats",
        state: "released Question outcomes and measured duration",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Response Stats",
      }),
      ...directViewportCaptures({
        checkpoint: "course_attempt_history_laptop",
        filenameStem: catalogScreenshotFilename(
          "grades",
          "studentAttemptHistory",
          "course_attempt_history",
        ),
        area: "grades",
        workflow: "Course Attempt History and Latest Feedback",
        state: "at least 40 submitted Attempts and released feedback shortcut enabled",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Attempt History with Latest Feedback available",
      }),
      ...directViewportCaptures({
        checkpoint: "latest_feedback_laptop",
        filenameStem: catalogScreenshotFilename(
          "grades",
          "studentLatestFeedback",
          "latest_feedback",
        ),
        area: "grades",
        workflow: "Latest Feedback shortcut",
        state: "released feedback review opened from Grades",
        viewport: "laptop",
        privacyProfile: "student_feedback_released",
        caption: "Attempt review opened from Latest Feedback",
      }),
      ...directViewportCaptures({
        checkpoint: "course_progress_unreleased_laptop",
        area: "courses",
        workflow: "Course Progress",
        state: "Attempts with no released score",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Progress when an Attempt has no released score",
      }),
    ],
    viewportCoverage: viewportCoverage(["laptop", "tablet", "phone", "square"], {}),
    run: async (runtime): Promise<void> => {
      await studentProgressAndStats(runtime);
      await studentAttemptHistory(runtime);
      await studentProgressWithoutReleasedScore(runtime);
    },
  },
];
