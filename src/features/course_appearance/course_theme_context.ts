// course_theme_context.ts - route-loaded appearance data shared without transport imports.

import { createContext, useContext } from "solid-js";

import type { CourseAppearanceView } from "../../../generated/api/CourseAppearanceView";
import type {
  AssignmentAttemptScreenData,
  AssignmentAttemptSummaryResponse,
  CourseRouteView,
} from "../../api/contracts";

export type CourseThemeRouteData =
  | { readonly kind: "course"; readonly course: CourseRouteView }
  | { readonly kind: "assignmentAttempt"; readonly screen: AssignmentAttemptScreenData }
  | {
      readonly kind: "assignmentAttemptSummary";
      readonly response: AssignmentAttemptSummaryResponse;
    };

export const CourseThemePresentationContext =
  createContext<(appearance: CourseAppearanceView) => void>();

/** Resolves the authorized Course Route View already owned by the route. */
export function courseRouteView(data: CourseThemeRouteData): CourseRouteView {
  switch (data.kind) {
    case "course":
      return data.course;
    case "assignmentAttempt":
      return data.screen.course;
    case "assignmentAttemptSummary":
      return data.response.course;
  }
}

/** Lets a successful appearance mutation update its route-local palette immediately. */
export function useCourseThemePresentation():
  ((appearance: CourseAppearanceView) => void) | undefined {
  return useContext(CourseThemePresentationContext);
}
