// assignment_workspace_paths.ts - pure public paths for Instructor assignment-local tasks.

import type {
  AssignmentRouteReference,
  CourseInstanceRouteReference,
} from "../../navigation/public_route";

export type AssignmentWorkspaceSection =
  "overview" | "questions" | "policies" | "studentView" | "gradingOperations";

const ASSIGNMENT_WORKSPACE_SECTION_SEGMENTS = {
  overview: "",
  questions: "questions",
  policies: "policies",
  studentView: "student-view",
  gradingOperations: "grading-operations",
} as const satisfies Readonly<Record<AssignmentWorkspaceSection, string>>;

/** The one route owner for starting a persisted Assignment and its Assignment Working Copy. */
export function assignmentWorkspaceCreatePath(
  courseReference: CourseInstanceRouteReference,
): string {
  return `/instructor/courses/${courseReference}/assignments/new`;
}

export function assignmentWorkspacePath(
  courseReference: CourseInstanceRouteReference,
  assignmentReference: AssignmentRouteReference,
  section?: AssignmentWorkspaceSection,
): string {
  const base = `/instructor/courses/${courseReference}/assignments/${assignmentReference}`;
  const segment = ASSIGNMENT_WORKSPACE_SECTION_SEGMENTS[section ?? "overview"];
  return segment.length === 0 ? base : `${base}/${segment}`;
}
