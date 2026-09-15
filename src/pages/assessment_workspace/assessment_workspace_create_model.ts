// assessment_workspace_create_model.ts - direct destination after persisted Assessment creation.

import type {
  AssessmentRouteReference,
  CourseInstanceRouteReference,
} from "../../navigation/public_route";
import type { CourseAssessmentSourceChoice } from "../../api/assessment_release";

import { assessmentWorkspacePath } from "./assessment_workspace_paths";

/** A successful title-only create always enters the Questions task for the new persisted draft. */
export function createdAssessmentQuestionsPath(
  courseReference: CourseInstanceRouteReference,
  assessmentReference: AssessmentRouteReference,
): string {
  return assessmentWorkspacePath(courseReference, assessmentReference, "questions");
}

/** Keeps transport details outside the visible draft-creation recovery path. */
export function assessmentWorkspaceCreateErrorMessage(): string {
  return "The Assessment could not be created. Your title is still here. Try again.";
}

/** Resolves only the Instructor's deliberate stable Blueprint Assessment choice. */
export function selectedAssessmentSource(
  choices: ReadonlyArray<CourseAssessmentSourceChoice>,
  blueprintAssessmentReference: string,
): CourseAssessmentSourceChoice | undefined {
  return choices.find(
    (choice) => choice.source.blueprint_assessment_reference === blueprintAssessmentReference,
  );
}
