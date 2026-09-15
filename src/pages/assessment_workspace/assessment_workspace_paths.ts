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
  courseReference: CourseInstanceRouteReference,
): string {
  return `/instructor/courses/${courseReference}/assessments/new`;
}

export function assessmentWorkspacePath(
  courseReference: CourseInstanceRouteReference,
  assessmentReference: AssessmentRouteReference,
  section?: AssessmentWorkspaceSection,
): string {
  const base = `/instructor/courses/${courseReference}/assessments/${assessmentReference}`;
  const segment = ASSIGNMENT_WORKSPACE_SECTION_SEGMENTS[section ?? "overview"];
  return segment.length === 0 ? base : `${base}/${segment}`;
}
