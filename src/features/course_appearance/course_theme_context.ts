// course_theme_context.ts - route-loaded appearance data shared without transport imports.

import { createContext, useContext } from "solid-js";

import type { CourseAppearance } from "../../../generated/api/CourseAppearance";
import type { CourseRouteData, RunScreenData, RunSummaryResponse } from "../../api/contracts";

export type CourseThemeRouteData =
  | { readonly kind: "course"; readonly course: CourseRouteData }
  | { readonly kind: "runAttempt"; readonly screen: RunScreenData }
  | { readonly kind: "runSummary"; readonly response: RunSummaryResponse };

export const CourseThemeRouteContext = createContext<CourseThemeRouteData>();
export const CourseThemePresentationContext =
  createContext<(appearance: CourseAppearance) => void>();

/** Resolves the one safe course projection already authorized by the route owner. */
export function courseRouteData(data: CourseThemeRouteData): CourseRouteData {
  switch (data.kind) {
    case "course":
      return data.course;
    case "runAttempt":
      return data.screen.course;
    case "runSummary":
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
