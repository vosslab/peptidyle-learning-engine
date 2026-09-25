// student_ribbon_navigation.ts - Student data used to resolve fixed Tier 2 controls.

import type { LiveStudentCourseLandingSummary } from "../api/live_student_course_landing";
import type { OrdinaryBrowserApiClient } from "../api/client";

export interface StudentRibbonNavigation {
  readonly studentCourses: ReadonlyArray<Pick<LiveStudentCourseLandingSummary, "id" | "shortName">>;
  readonly activeAttemptId?: string;
  readonly latestFeedbackAttemptId?: string;
}

interface ActiveAttemptCandidate {
  readonly id: string;
  readonly startedAt: number;
  readonly latestActivityAt: number;
}

/** Reads and resolves only the data needed by the fixed Student Tier 2 controls. */
export async function loadStudentRibbonNavigation(
  client: OrdinaryBrowserApiClient,
): Promise<StudentRibbonNavigation> {
  const courses = await client.listLiveStudentCourses();
  const [activeAttempts, latestFeedbackResult] = await Promise.all([
    Promise.allSettled(
      courses.map((course) => client.getStudentCourseActiveAttempt(course.id)),
    ).then((results) =>
      results.flatMap((result) => (result.status === "fulfilled" ? [result.value] : [])),
    ),
    client.getStudentLatestFeedback().catch(() => undefined),
  ]);
  const candidates = activeAttempts.flatMap((attempt): Array<ActiveAttemptCandidate> => {
    if (
      attempt.assessmentAttemptId === null ||
      attempt.startedAt === null ||
      attempt.latestActivityAt === null
    ) {
      return [];
    }
    return [
      {
        id: attempt.assessmentAttemptId,
        startedAt: attempt.startedAt,
        latestActivityAt: attempt.latestActivityAt,
      },
    ];
  });
  candidates.sort(
    (left, right) =>
      right.latestActivityAt - left.latestActivityAt ||
      right.startedAt - left.startedAt ||
      right.id.localeCompare(left.id),
  );
  return {
    studentCourses: courses.map(({ id, shortName }) => ({ id, shortName })),
    ...(candidates[0] === undefined ? {} : { activeAttemptId: candidates[0].id }),
    ...(latestFeedbackResult === undefined || latestFeedbackResult.assessmentAttemptId === null
      ? {}
      : { latestFeedbackAttemptId: latestFeedbackResult.assessmentAttemptId }),
  };
}
