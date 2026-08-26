// assignment_workspace_create_model.ts - canonical destination after persisted draft creation.

import type { AssignmentRouteReference, CourseRouteReference } from "../../navigation/public_route";

import { assignmentWorkspacePath } from "./assignment_workspace_paths";

/** A successful title-only create always enters the Questions task for the new persisted draft. */
export function createdAssignmentQuestionsPath(
  courseReference: CourseRouteReference,
  assignmentReference: AssignmentRouteReference,
): string {
  return assignmentWorkspacePath(courseReference, assignmentReference, "questions");
}
