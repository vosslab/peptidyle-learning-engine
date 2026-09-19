// assessment_workspace_paths.ts - pure public paths for Instructor assessment-local tasks.

import type {
  AssessmentRouteReference,
  CourseInstanceRouteReference,
} from "../../navigation/public_route";

export type AssessmentWorkspaceSection = "overview" | "questions" | "policies" | "studentView";

const ASSIGNMENT_WORKSPACE_SECTION_SEGMENTS = {
  overview: "",
  questions: "questions",
  policies: "properties",
  studentView: "student-view",
} as const satisfies Readonly<Record<AssessmentWorkspaceSection, string>>;

/** The one route owner for starting a persisted Assessment and its Assessment. */
export function assessmentWorkspaceCreatePath(
  courseInstanceId: CourseInstanceRouteReference,
): string {
  return `/instructor/courses/${courseInstanceId}/assessments/new`;
}

export function assessmentWorkspacePath(
  courseInstanceId: CourseInstanceRouteReference,
  assessmentId: AssessmentRouteReference,
  section?: AssessmentWorkspaceSection,
): string {
  const base = `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}`;
  const segment = ASSIGNMENT_WORKSPACE_SECTION_SEGMENTS[section ?? "overview"];
  return segment.length === 0 ? base : `${base}/${segment}`;
}
