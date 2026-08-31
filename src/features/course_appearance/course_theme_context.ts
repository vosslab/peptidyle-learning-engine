// course_theme_context.ts - route-loaded appearance data shared without transport imports.

import { createContext, useContext } from "solid-js";

import type { CourseAppearance } from "../../../generated/api/CourseAppearance";
import type {
  AssignmentAttemptScreenData,
  AssignmentAttemptSummaryResponse,
  CourseRouteData,
} from "../../api/contracts";

export type CourseThemeRouteData =
  | { readonly kind: "course"; readonly course: CourseRouteData }
  | { readonly kind: "assignmentAttempt"; readonly screen: AssignmentAttemptScreenData }
  | {
      readonly kind: "assignmentAttemptSummary";
      readonly response: AssignmentAttemptSummaryResponse;
    };

export const CourseThemeRouteContext = createContext<CourseThemeRouteData>();
export const CourseThemePresentationContext =
  createContext<(appearance: CourseAppearance) => void>();

/** Resolves the one safe course projection already authorized by the route owner. */
export function courseRouteData(data: CourseThemeRouteData): CourseRouteData {
  switch (data.kind) {
    case "course":
      return data.course;
    case "assignmentAttempt":
      return data.screen.course;
    case "assignmentAttemptSummary":
      return data.response.course;
  }
}

/** Optional access lets focused component fixtures mount outside the app shell. */
export function useCourseThemeRouteData(): CourseThemeRouteData | undefined {
  return useContext(CourseThemeRouteContext);
}

/** Lets a successful appearance mutation update its route-local palette immediately. */
export function useCourseThemePresentation(): ((appearance: CourseAppearance) => void) | undefined {
  return useContext(CourseThemePresentationContext);
}
