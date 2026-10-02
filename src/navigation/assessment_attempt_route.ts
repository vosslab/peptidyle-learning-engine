// Course-scoped Attempt paths keep the private Attempt UUID in browser history state.

import { parseAssessmentAttemptId, type AssessmentAttemptRouteId } from "./public_route";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";

export type AssessmentAttemptRouteState = Readonly<{
  assessmentAttemptId: AssessmentAttemptRouteId;
}>;

export function assessmentAttemptRouteState(
  assessmentAttemptId: AssessmentAttemptRouteId,
): AssessmentAttemptRouteState {
  return Object.freeze({ assessmentAttemptId });
}

export function assessmentAttemptRouteStateFromHistory(
  value: unknown,
): AssessmentAttemptRouteState | undefined {
  if (value === null || typeof value !== "object" || !("assessmentAttemptId" in value)) {
    return undefined;
  }
  const assessmentAttemptId: unknown = value.assessmentAttemptId;
  if (typeof assessmentAttemptId !== "string") return undefined;
  const parsed = parseAssessmentAttemptId(assessmentAttemptId);
  return parsed === null ? undefined : assessmentAttemptRouteState(parsed);
}

export function assessmentAttemptPath(
  view: "attempt" | "review",
  courseInstanceId: CourseInstanceId,
): string {
  return `/courses/${courseInstanceId}/${view}`;
}
