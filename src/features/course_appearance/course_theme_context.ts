// course_theme_context.ts - route-loaded appearance data shared without transport imports.

import { createContext, useContext } from "solid-js";

import type { CourseId } from "../../../generated/api/CourseId";
import type { CourseAppearanceView } from "../../../generated/api/CourseAppearanceView";
import type { CourseRouteView } from "../../api/contracts";
import type { StudentAssessmentAttemptContext } from "../../api/assessment_attempt_navigation";
import type { StudentAssessmentAttemptHistory } from "../../api/assessment_attempt_history";

export type CourseThemeRouteData =
  | { readonly kind: "course"; readonly course: CourseRouteView }
  | { readonly kind: "assessmentAttempt"; readonly context: StudentAssessmentAttemptContext }
  | {
      readonly kind: "assessmentAttemptHistory";
      readonly history: StudentAssessmentAttemptHistory;
    };

/** A temporary rendered appearance; `undefined` releases the local preview. */
export type CourseThemePresentation = (appearance: CourseAppearanceView | undefined) => void;

export const CourseThemePresentationContext = createContext<CourseThemePresentation>();

/** Resolves the authorized Course Route View already owned by the route. */
export function courseRouteView(data: CourseThemeRouteData): CourseRouteView {
  switch (data.kind) {
    case "course":
      return data.course;
    case "assessmentAttempt":
      throw new Error("Assessment Attempt context has no UUID-backed Course Route View");
    case "assessmentAttemptHistory":
      throw new Error("Assessment Attempt history has no UUID-backed Course Route View");
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
