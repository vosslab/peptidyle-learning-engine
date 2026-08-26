// assignment_workspace_paths.ts - pure public paths for Instructor assignment-local tasks.

import type { AssignmentRouteReference, CourseRouteReference } from "../../navigation/public_route";

export type AssignmentWorkspaceSection = "overview" | "questions" | "policies" | "studentView";

export function assignmentWorkspacePath(
  courseReference: CourseRouteReference,
  assignmentReference: AssignmentRouteReference,
  section?: AssignmentWorkspaceSection,
): string {
  const base = `/instructor/courses/${courseReference}/assignments/${assignmentReference}`;
  if (section === undefined || section === "overview") return base;
  return `${base}/${section === "studentView" ? "student-view" : section}`;
}
