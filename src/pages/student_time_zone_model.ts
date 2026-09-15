import type { StudentAssessmentDecisionSummary } from "../../generated/api/StudentAssessmentDecisionSummary";
import type { LiveStudentAssessmentLandingSummary } from "../api/live_student_course_landing";

/** Applies a server-confirmed display preference without changing any decision or instant. */
export function applyStudentDisplayTimeZone(
  assessments: ReadonlyArray<LiveStudentAssessmentLandingSummary>,
  timeZone: string,
): ReadonlyArray<LiveStudentAssessmentLandingSummary> {
  return assessments.map((assessment) => ({
    ...assessment,
    decision: {
      ...assessment.decision,
      displayTimeZone: timeZone,
    } satisfies StudentAssessmentDecisionSummary,
  }));
}
