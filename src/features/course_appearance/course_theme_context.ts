// course_theme_context.ts - route-loaded appearance data shared without transport imports.

import { createContext, useContext } from "solid-js";

import type { CourseId } from "../../../generated/api/CourseId";
import type { CourseAppearanceView } from "../../../generated/api/CourseAppearanceView";
import type { AssignmentAttemptSummaryResponse, CourseRouteView } from "../../api/contracts";
import type { StudentAssignmentAttemptContext } from "../../api/assignment_attempt_navigation";

export type CourseThemeRouteData =
  | { readonly kind: "course"; readonly course: CourseRouteView }
  | { readonly kind: "assignmentAttempt"; readonly context: StudentAssignmentAttemptContext }
  | {
      readonly kind: "assignmentAttemptSummary";
      readonly response: AssignmentAttemptSummaryResponse;
    };

/** A temporary rendered appearance; `undefined` releases the local preview. */
export type CourseThemePresentation = (appearance: CourseAppearanceView | undefined) => void;

export const CourseThemePresentationContext = createContext<CourseThemePresentation>();

/** Resolves the authorized Course Route View already owned by the route. */
export function courseRouteView(data: CourseThemeRouteData): CourseRouteView {
  switch (data.kind) {
    case "course":
      return data.course;
    case "assignmentAttempt":
      throw new Error("Assignment Attempt context has no UUID-backed Course Route View");
    case "assignmentAttemptSummary":
      return data.response.course;
  }
}

/** Lets a successful appearance mutation update its route-local palette immediately. */
export function useCourseThemePresentation(): CourseThemePresentation | undefined {
  return useContext(CourseThemePresentationContext);
}

/** Replaces a saved Course Appearance in the presentation cache for its Course. */
export type ReplaceCourseAppearance = (
  courseId: CourseId,
  appearance: CourseAppearanceView,
) => void;
