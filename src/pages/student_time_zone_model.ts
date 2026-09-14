import type { StudentAssignmentDecisionSummary } from "../../generated/api/StudentAssignmentDecisionSummary";
import type { LiveStudentAssignmentLandingSummary } from "../api/live_student_course_landing";

/** Applies a server-confirmed display preference without changing any decision or instant. */
export function applyStudentDisplayTimeZone(
  assignments: ReadonlyArray<LiveStudentAssignmentLandingSummary>,
  timeZone: string,
): ReadonlyArray<LiveStudentAssignmentLandingSummary> {
  return assignments.map((assignment) => ({
    ...assignment,
    decision: {
      ...assignment.decision,
      displayTimeZone: timeZone,
    } satisfies StudentAssignmentDecisionSummary,
  }));
}
