// assignment_workspace_create_model.ts - canonical destination after persisted draft creation.

import type {
  AssignmentRouteReference,
  CourseInstanceRouteReference,
} from "../../navigation/public_route";

import { assignmentWorkspacePath } from "./assignment_workspace_paths";

/** A successful title-only create always enters the Questions task for the new persisted draft. */
export function createdAssignmentQuestionsPath(
  courseReference: CourseInstanceRouteReference,
  assignmentReference: AssignmentRouteReference,
): string {
  return assignmentWorkspacePath(courseReference, assignmentReference, "questions");
}

/** Keeps transport details outside the visible draft-creation recovery path. */
export function assignmentWorkspaceCreateErrorMessage(): string {
  return "The Assignment could not be created. Your title is still here. Try again.";
}
