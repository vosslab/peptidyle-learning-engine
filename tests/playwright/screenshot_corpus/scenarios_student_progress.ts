// Student Course Progress, Coursework views, Response Stats, and Attempt History captures.

import type { Locator, Page } from "playwright";

import type { CaptureSession, ScenarioRuntime } from "./runtime";
import { viewportCoverage, type ScenarioDefinition } from "./scenario_types";
import {
  ASSESSMENT_ENTRY_BUTTON,
  ASSIGNMENT_TITLE,
  choosePersona,
  openAllStudentCoursework,
  openStudentAssignment,
  openStudentCourse,
  scrollTop,
} from "./visible_workflows";

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
  await page.locator('[data-route-surface="studentCourseAttemptHistory"]').waitFor();
  await page.getByText("Loading Attempt History...", { exact: true }).waitFor({ state: "hidden" });
}

function attemptHistoryForPilotAssessment(page: Page): Locator {
  return page
    .locator(".student-course-attempt-history__row")
    .filter({ has: page.getByRole("heading", { name: ASSIGNMENT_TITLE, exact: true }) });
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
  const session = await runtime.open("course_progress_laptop");
  try {
    const page = session.page;
    await choosePersona(page, "Mary Okafor");
    await openStudentCourse(page);
    await page.locator('[data-route-surface="studentCourseProgress"]').waitFor();
    await page
      .getByText("Loading Assessment progress...", { exact: true })
      .waitFor({ state: "hidden" });
    await createUnansweredAttempt(page, true);
    await page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await openStudentCourse(page);
    await page.locator('[data-route-surface="studentCourseProgress"]').waitFor();
    await page
      .getByText("Loading Assessment progress...", { exact: true })
      .waitFor({ state: "hidden" });
    await captureCheckpoint(runtime, "course_progress_laptop", session);

    const tabs = page.getByRole("navigation", { name: "Ribbon tabs", exact: true });
    await tabs.getByRole("link", { name: "Coursework", exact: true }).click();
    await page.getByRole("link", { name: "Due Soon", exact: true }).click();
    await page.locator('[data-route-surface="studentCourseDueSoon"]').waitFor();
    await page.getByText("Loading Coursework...", { exact: true }).waitFor({ state: "hidden" });
    await captureCheckpoint(runtime, "course_due_soon_laptop", session);

    await page.getByRole("link", { name: "Completed", exact: true }).click();
    await page.locator('[data-route-surface="studentCourseCompleted"]').waitFor();
    await page.getByText("Loading Coursework...", { exact: true }).waitFor({ state: "hidden" });
    await page.locator(".record-list__row").first().waitFor();
    await captureCheckpoint(runtime, "course_completed_laptop", session);

    await tabs.getByRole("link", { name: "Courses", exact: true }).click();
    await openStudentCourse(page);
    await page.locator('[data-route-surface="studentCourseProgress"]').waitFor();

    await page.getByRole("link", { name: "Response Stats", exact: true }).click();
    await page.locator('[data-route-surface="studentCourseResponseStats"]').waitFor();
    await page.getByText("Loading Response Stats...", { exact: true }).waitFor({ state: "hidden" });
    await page.locator(".student-course-response-stats__row").first().waitFor();
    await captureCheckpoint(runtime, "response_stats_laptop", session);
  } finally {
    await runtime.close(session);
  }
}

async function studentCourseAttemptHistory(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("course_attempt_history_laptop");
  try {
    const page = session.page;
    await choosePersona(page, "Mary Okafor");
    await openStudentCourse(page);
    await openCourseAttemptHistory(page);
    let submittedAttemptCount = await submittedAttemptHistoryForPilotAssessment(page).count();
    for (
      let createdAttemptCount = 0;
      submittedAttemptCount < 40 && createdAttemptCount < 40;
      createdAttemptCount += 1
    ) {
      const previousAttemptCount = submittedAttemptCount;
      await createUnansweredAttempt(page, false);
      const historyResponsePromise = page.waitForResponse((response) => {
        return (
          new URL(response.url()).pathname.endsWith("/assessment-attempts") &&
          response.request().method() === "GET"
        );
      });
      await openCourseAttemptHistory(page);
      const historyResponse = await historyResponsePromise;
      const historyPayload = (await historyResponse.json()) as {
        readonly items?: ReadonlyArray<{ readonly assessmentAttemptNumber?: number }>;
      };
      submittedAttemptCount = await submittedAttemptHistoryForPilotAssessment(page).count();
      if (submittedAttemptCount <= previousAttemptCount) {
        throw new Error(
          `Course Attempt History stayed at ${submittedAttemptCount} submitted Attempts after a new submission; ` +
            `API returned ${historyPayload.items?.length ?? "no items field"} items ` +
            `(${historyResponse.status()}) with Attempt numbers ${historyPayload.items?.map((item) => item.assessmentAttemptNumber).join(", ") ?? "unavailable"}. ` +
            `History text: ${await page.locator(".student-course-attempt-history").innerText()}`,
        );
      }
    }
    if (submittedAttemptCount < 40) {
      throw new Error(
        `Expected at least 40 submitted Course Attempts, found ${submittedAttemptCount}.`,
      );
    }
    await captureCheckpoint(runtime, "course_attempt_history_laptop", session);
    const latestSubmittedAttempt = submittedAttemptHistoryForPilotAssessment(page).first();
    await latestSubmittedAttempt.getByRole("link", { name: "Review Attempt", exact: true }).click();
    await page.locator('[data-route-surface="assessmentAttemptSummary"]').waitFor();
    await page.getByRole("heading", { name: "Your recorded work", exact: true }).waitFor();
    await captureCheckpoint(runtime, "course_attempt_history_selected_laptop", session);

    await page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await openStudentCourse(page);
    await page.getByRole("link", { name: "Response Stats", exact: true }).click();
    await page.locator('[data-route-surface="studentCourseResponseStats"]').waitFor();
    await page.getByText("Loading Response Stats...", { exact: true }).waitFor({ state: "hidden" });
    const historyResponsePromise = page.waitForResponse((response) => {
      return (
        new URL(response.url()).pathname.endsWith("/assessment-attempts") &&
        response.request().method() === "GET"
      );
    });
    await openCourseAttemptHistory(page);
    const historyResponse = await historyResponsePromise;
    const historyPayload = (await historyResponse.json()) as {
      readonly items?: ReadonlyArray<{ readonly assessmentAttemptNumber?: number }>;
    };
    submittedAttemptCount = await submittedAttemptHistoryForPilotAssessment(page).count();
    if (submittedAttemptCount < 40) {
      throw new Error(
        `Course History lost submitted Attempts after navigating through Response Stats; ` +
          `found ${submittedAttemptCount}. API returned ${historyPayload.items?.length ?? "no items field"} items ` +
          `(${historyResponse.status()}) with Attempt numbers ${historyPayload.items?.map((item) => item.assessmentAttemptNumber).join(", ") ?? "unavailable"}. ` +
          `URL: ${page.url()}; history text: ${await page.locator(".student-course-attempt-history").innerText()}`,
      );
    }
  } finally {
    await runtime.close(session);
  }
}

async function studentProgressWithoutReleasedScore(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("course_progress_unreleased_laptop");
  try {
    await choosePersona(session.page, "Jack Nguyen");
    await openStudentCourse(session.page);
    await session.page.locator('[data-route-surface="studentCourseProgress"]').waitFor();
    await session.page
      .getByText("Loading Assessment progress...", { exact: true })
      .waitFor({ state: "hidden" });
    await session.page.getByText("Score not released", { exact: true }).waitFor();
    await captureCheckpoint(runtime, "course_progress_unreleased_laptop", session);
  } finally {
    await runtime.close(session);
  }
}

export const STUDENT_PROGRESS_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "student_progress_stats_attempt_history",
    role: "student",
    captures: [
      {
        checkpoint: "course_progress_laptop",
        area: "courses",
        workflow: "Course Progress",
        state: "completed Attempts and score status",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Course Progress",
        featured: true,
      },
      {
        checkpoint: "course_due_soon_laptop",
        area: "coursework",
        workflow: "Student Due Soon Coursework",
        state: "server-bounded next seven days",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Coursework Due Soon",
      },
      {
        checkpoint: "course_completed_laptop",
        area: "coursework",
        workflow: "Student Completed Coursework",
        state: "submitted Attempts",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Completed Coursework",
      },
      {
        checkpoint: "response_stats_laptop",
        area: "courses",
        workflow: "Response Stats",
        state: "released Question outcomes and measured duration",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Response Stats",
      },
      {
        checkpoint: "course_attempt_history_laptop",
        area: "grades",
        workflow: "Course Attempt History",
        state: "at least 40 submitted Attempts",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Course Attempt History",
      },
      {
        checkpoint: "course_attempt_history_selected_laptop",
        area: "grades",
        workflow: "Course Attempt History",
        state: "latest submitted Attempt review",
        viewport: "laptop",
        privacyProfile: "student_feedback_released",
        caption: "Latest submitted Attempt review opened from Course History",
      },
      {
        checkpoint: "course_progress_unreleased_laptop",
        area: "courses",
        workflow: "Course Progress",
        state: "Attempts with no released score",
        viewport: "laptop",
        privacyProfile: "student_self",
        caption: "Student Progress when an Attempt has no released score",
      },
    ],
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: {
        target: "course_progress_laptop",
        reason: "The laptop capture is the representative.",
      },
      phone: {
        target: "course_progress_laptop",
        reason: "The laptop capture is the representative.",
      },
      square: {
        target: "course_progress_laptop",
        reason: "The laptop capture is the representative.",
      },
    }),
    run: async (runtime): Promise<void> => {
      await studentProgressAndStats(runtime);
      await studentCourseAttemptHistory(runtime);
      await studentProgressWithoutReleasedScore(runtime);
    },
  },
];
