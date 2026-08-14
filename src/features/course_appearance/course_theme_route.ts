// course_theme_route.ts - pure route classification for course-owned theme scopes.

import type { CourseRouteReference, RunRouteReference } from "../../navigation/public_route";
import { parsePublicRouteReference } from "../../navigation/public_route";

export type CourseThemeRouteRequest =
  | { readonly kind: "global" }
  | { readonly kind: "course"; readonly courseReference: CourseRouteReference }
  | { readonly kind: "runAttempt"; readonly runReference: RunRouteReference }
  | { readonly kind: "runSummary"; readonly runReference: RunRouteReference };

function courseReference(value: string): CourseRouteReference | null {
  const parsed = parsePublicRouteReference(value);
  return parsed !== null && value.startsWith("C-") ? (parsed as CourseRouteReference) : null;
}

function runReference(value: string): RunRouteReference | null {
  const parsed = parsePublicRouteReference(value);
  return parsed !== null && value.startsWith("R-") ? (parsed as RunRouteReference) : null;
}

/** Classifies only executable course-owned routes; all other pages remain global. */
export function courseThemeRouteRequest(pathname: string): CourseThemeRouteRequest {
  const segments = pathname.split("/").filter((segment) => segment.length > 0);
  if (segments[0] === "runs" && segments[1] !== undefined) {
    const reference = runReference(segments[1]);
    if (reference === null) return { kind: "global" };
    if (segments.length === 2) return { kind: "runAttempt", runReference: reference };
    if (segments.length === 3 && segments[2] === "summary") {
      return { kind: "runSummary", runReference: reference };
    }
  }
  if (segments[0] === "courses" && segments[1] !== undefined) {
    const reference = courseReference(segments[1]);
    if (reference === null) return { kind: "global" };
    if (segments.length === 2) return { kind: "course", courseReference: reference };
    if (segments.length === 4 && segments[2] === "assignments" && segments[3] !== undefined) {
      return { kind: "course", courseReference: reference };
    }
  }
  if (segments[0] === "instructor" && segments[1] === "courses" && segments[2] !== undefined) {
    const reference = courseReference(segments[2]);
    if (reference === null) return { kind: "global" };
    if (segments.length === 5 && segments[3] === "assignments" && segments[4] === "new") {
      return { kind: "course", courseReference: reference };
    }
    if (segments.length === 4 && segments[3] === "gradebook") {
      return { kind: "course", courseReference: reference };
    }
    if (segments.length === 4 && segments[3] === "appearance") {
      return { kind: "course", courseReference: reference };
    }
    if (segments.length === 4 && segments[3] === "students") {
      return { kind: "course", courseReference: reference };
    }
    if (
      segments.length === 6 &&
      segments[3] === "assignments" &&
      segments[4] !== undefined &&
      segments[5] === "edit"
    ) {
      return { kind: "course", courseReference: reference };
    }
  }
  return { kind: "global" };
}
