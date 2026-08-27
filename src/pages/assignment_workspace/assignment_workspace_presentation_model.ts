// assignment_workspace_presentation_model.ts - learner-accessible state language for policies.

import type { InstructorAssignmentCurrentState } from "../../../generated/api/InstructorAssignmentCurrentState";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../../generated/api/InstructorAssignmentTeachingSettingsLocal";

function displayCourseLocalTime(value: string): string {
  return `${value.slice(0, 10)} ${value.slice(11, 16)}`;
}

/** Explains the current learner-access state without exposing an implementation detail. */
export function assignmentCurrentStateCopy(
  lifecycle: InstructorAssignmentTeachingSettingsLocal["lifecycle"],
  current: InstructorAssignmentCurrentState,
  timeZone: string,
): string {
  if (current.state === "draft") return "Draft. Students cannot access this assignment.";
  if (current.state === "archived") return "Archived. Students cannot access this assignment.";
  if (current.state === "scheduled") {
    return `Published, scheduled to open at ${displayCourseLocalTime(current.availableAt)} ${timeZone}.`;
  }
  if (current.state === "open") return "Published, open now.";
  if (lifecycle === "published" && current.closedAt !== null) {
    return `Published, closed since ${displayCourseLocalTime(current.closedAt)} ${timeZone}.`;
  }
  return "Closed by instructor. Students cannot start new work.";
}
