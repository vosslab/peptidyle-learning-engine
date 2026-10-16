import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
  StudentCourseProgressAssessment,
} from "../api/live_student_course_landing";
import {
  hasSubmittedStudentAttempt,
  isInStudentDueSoonWindow,
} from "./student_coursework_presentation";

export interface StudentCrossCourseCollection<Row> {
  readonly rows: ReadonlyArray<Row>;
  readonly failedCourses: ReadonlyArray<LiveStudentCourseLandingSummary>;
}

/** Keep successful Course results usable while naming every incomplete Course. */
export async function loadStudentCrossCourseCollection<Row>(
  courses: ReadonlyArray<LiveStudentCourseLandingSummary>,
  loadCourse: (course: LiveStudentCourseLandingSummary) => Promise<ReadonlyArray<Row>>,
): Promise<StudentCrossCourseCollection<Row>> {
  const results = await Promise.allSettled(courses.map(loadCourse));
  const rows: Row[] = [];
  const failedCourses: LiveStudentCourseLandingSummary[] = [];
  results.forEach((result, index) => {
    const course = courses[index];
    if (course === undefined) return;
    if (result.status === "fulfilled") rows.push(...result.value);
    else failedCourses.push(course);
  });
  return { rows, failedCourses };
}

/** Apply server-evaluated and submitted-Attempt membership before local discovery. */
export function visibleStudentCoursework(
  assessments: ReadonlyArray<LiveStudentAssessmentLandingSummary>,
  view: "all" | "dueSoon" | "completed",
  progress: ReadonlyArray<StudentCourseProgressAssessment> = [],
): LiveStudentAssessmentLandingSummary[] {
  if (view === "all") return [...assessments];
  if (view === "dueSoon") {
    return assessments.filter((assessment) =>
      isInStudentDueSoonWindow(assessment.decision.dueAt, assessment.decision.evaluatedAt),
    );
  }
  const submittedIds = new Set(
    progress
      .filter((assessment) =>
        hasSubmittedStudentAttempt(assessment.submittedAssessmentAttemptCount),
      )
      .map((assessment) => assessment.id),
  );
  return assessments.filter((assessment) => submittedIds.has(assessment.id));
}

/** Released score presence is the disclosure boundary for the Scores collection. */
export function visibleStudentScores(
  assessments: ReadonlyArray<LiveStudentAssessmentLandingSummary>,
): LiveStudentAssessmentLandingSummary[] {
  return assessments.filter((assessment) => assessment.assessmentScore !== undefined);
}
