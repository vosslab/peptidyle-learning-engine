// course_theme_route.ts - pure route classification for course-owned theme scopes.

import type { CourseRouteReference, RunRouteReference } from "../../navigation/public_route";
import { parseCourseReference, parseRunReference } from "../../navigation/public_route";
import { routeContractForPathname } from "../../route_contract";

export type CourseThemeRouteRequest =
  | { readonly kind: "global" }
  | { readonly kind: "course"; readonly courseReference: CourseRouteReference }
  | { readonly kind: "runAttempt"; readonly runReference: RunRouteReference }
  | { readonly kind: "runSummary"; readonly runReference: RunRouteReference };

/** Classifies only executable course-owned routes; all other pages remain global. */
export function courseThemeRouteRequest(pathname: string): CourseThemeRouteRequest {
  const route = routeContractForPathname(pathname);
  if (route === undefined) return { kind: "global" };
  const segments = pathname.split("/").filter((segment) => segment.length > 0);
  if ((route.id === "runAttempt" || route.id === "runSummary") && segments[1] !== undefined) {
    const reference = parseRunReference(segments[1]);
    if (reference === null) return { kind: "global" };
    return route.id === "runAttempt"
      ? { kind: "runAttempt", runReference: reference }
      : { kind: "runSummary", runReference: reference };
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
