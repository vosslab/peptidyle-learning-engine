// assessment_workspace_create_model.ts - direct destination after persisted Assessment creation.

import type { AssessmentRouteId, CourseInstanceRouteId } from "../../navigation/public_route";
import { isAssessmentType } from "../../assessment_type_presentation";
import type { AssessmentType } from "../../../generated/api/AssessmentType";

import { assessmentWorkspacePath } from "./assessment_workspace_paths";

/** A successful Assessment create always enters the Questions task for the new persisted draft. */
export function createdAssessmentQuestionsPath(
  courseInstanceId: CourseInstanceRouteId,
  assessmentId: AssessmentRouteId,
): string {
  return assessmentWorkspacePath(courseInstanceId, assessmentId, "questions");
}

/** Keeps transport details outside the visible draft-creation recovery path. */
export function assessmentWorkspaceCreateErrorMessage(): string {
  return "The Assessment could not be created. Your title is still here. Try again.";
}

/** Keeps Template-copy recovery as explicit as direct-create recovery. */
export function assessmentWorkspaceTemplateCreateErrorMessage(): string {
  return "The Assessment could not be created from this Template. Your title is still here. Try again.";
}

/** Accepts only one deliberate selection from PLE's fixed Assessment Type registry. */
export function selectedAssessmentType(value: string): AssessmentType | undefined {
  return isAssessmentType(value) ? value : undefined;
}
