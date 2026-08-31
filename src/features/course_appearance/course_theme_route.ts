// course_theme_route.ts - pure route classification for course-owned theme scopes.

import type {
  AssignmentAttemptRouteReference,
  CourseRouteReference,
} from "../../navigation/public_route";
import {
  parseAssignmentAttemptReference,
  parseCourseReference,
} from "../../navigation/public_route";
import { routeContractForPathname } from "../../route_contract";

export type CourseThemeRouteRequest =
  | { readonly kind: "global" }
  | { readonly kind: "course"; readonly courseReference: CourseRouteReference }
  | {
      readonly kind: "assignmentAttempt";
      readonly assignmentAttemptReference: AssignmentAttemptRouteReference;
    }
  | {
      readonly kind: "assignmentAttemptSummary";
      readonly assignmentAttemptReference: AssignmentAttemptRouteReference;
    };

/** Classifies only executable course-owned routes; all other pages remain global. */
export function courseThemeRouteRequest(pathname: string): CourseThemeRouteRequest {
  const route = routeContractForPathname(pathname);
  if (route === undefined) return { kind: "global" };
  const segments = pathname.split("/").filter((segment) => segment.length > 0);
  if (
    (route.id === "assignmentAttempt" || route.id === "assignmentAttemptSummary") &&
    segments[1] !== undefined
  ) {
    const reference = parseAssignmentAttemptReference(segments[1]);
    if (reference === null) return { kind: "global" };
    return route.id === "assignmentAttempt"
      ? { kind: "assignmentAttempt", assignmentAttemptReference: reference }
      : { kind: "assignmentAttemptSummary", assignmentAttemptReference: reference };
  }
  if (route.path.startsWith("/courses/:courseRef") && segments[1] !== undefined) {
    const reference = parseCourseReference(segments[1]);
    return reference === null ? { kind: "global" } : { kind: "course", courseReference: reference };
  }
  if (route.path.startsWith("/instructor/courses/:courseRef") && segments[2] !== undefined) {
    const reference = parseCourseReference(segments[2]);
    return reference === null ? { kind: "global" } : { kind: "course", courseReference: reference };
  }
  return { kind: "global" };
}
