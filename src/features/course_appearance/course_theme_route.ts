// course_theme_route.ts - pure route classification for course-owned theme scopes.

import type { CourseId } from "../../../generated/api/CourseId";
import type { RunId } from "../../../generated/api/RunId";

export type CourseThemeRouteRequest =
  | { readonly kind: "global" }
  | { readonly kind: "course"; readonly courseId: CourseId }
  | { readonly kind: "runAttempt"; readonly runId: RunId }
  | { readonly kind: "runSummary"; readonly runId: RunId };

/** Classifies only executable course-owned routes; all other pages remain global. */
export function courseThemeRouteRequest(pathname: string): CourseThemeRouteRequest {
  const segments = pathname.split("/").filter((segment) => segment.length > 0);
  if (segments[0] === "runs" && segments[1] !== undefined) {
    if (segments.length === 2) return { kind: "runAttempt", runId: segments[1] };
    if (segments.length === 3 && segments[2] === "summary") {
      return { kind: "runSummary", runId: segments[1] };
    }
  }
  if (segments[0] === "courses" && segments[1] !== undefined) {
    if (segments.length === 2) return { kind: "course", courseId: segments[1] };
    if (segments.length === 4 && segments[2] === "assignments" && segments[3] !== undefined) {
      return { kind: "course", courseId: segments[1] };
    }
  }
  if (segments[0] === "instructor" && segments[1] === "courses" && segments[2] !== undefined) {
    if (segments.length === 5 && segments[3] === "assignments" && segments[4] === "new") {
      return { kind: "course", courseId: segments[2] };
    }
    if (segments.length === 4 && segments[3] === "gradebook") {
      return { kind: "course", courseId: segments[2] };
    }
    if (segments.length === 4 && segments[3] === "appearance") {
      return { kind: "course", courseId: segments[2] };
    }
    if (
      segments.length === 6 &&
      segments[3] === "assignments" &&
      segments[4] !== undefined &&
      segments[5] === "edit"
    ) {
      return { kind: "course", courseId: segments[2] };
    }
  }
  return { kind: "global" };
}
