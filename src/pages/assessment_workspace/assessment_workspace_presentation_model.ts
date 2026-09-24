// assessment_workspace_presentation_model.ts - Student-accessible state language for policies.

import type { InstructorAssessmentAvailabilityView } from "../../../generated/api/InstructorAssessmentAvailabilityView";
import type { AssessmentStatus } from "../../../generated/api/AssessmentStatus";

function displayCourseLocalTime(value: string): string {
  return `${value.slice(0, 10)} ${value.slice(11, 16)}`;
}

/** Explains the current student-access state without exposing an implementation detail. */
export function assessmentAvailabilityCopy(
  status: AssessmentStatus,
  current: InstructorAssessmentAvailabilityView,
): string {
  if (current.state === "unreleased") return "Unreleased. Students cannot access this assessment.";
  if (current.state === "archived") return "Archived. Students cannot access this assessment.";
  if (current.state === "scheduled") {
    return `Released, scheduled to open at ${displayCourseLocalTime(current.available_at)}.`;
  }
  if (current.state === "available") return "Released, available now.";
  if (status === "released" && current.closed_at !== null) {
    return `Released, closed since ${displayCourseLocalTime(current.closed_at)}.`;
  }
  return "Closed by instructor. Students cannot start new work.";
}
