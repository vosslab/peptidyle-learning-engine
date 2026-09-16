// course_theme_variables.tsx - shell-stable course-theme variable presentation.

import { createEffect, createMemo, createSignal, type JSX } from "solid-js";

import type { CourseAppearanceView } from "../../../generated/api/CourseAppearanceView";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import { useRouteScopeData } from "../../ribbon/route_scope_context";
import {
  CourseThemePresentationContext,
  courseRouteView,
  type CourseThemeRouteData,
} from "./course_theme_context";
import { COURSE_THEME_SCOPE_STYLES } from "./course_theme_scope_styles";
import { courseThemeStyle, courseThemeTokens } from "./course_theme_registry";
import { courseInstanceRouteReference } from "../../navigation/public_route";

export interface CourseThemeVariablesProps {
  readonly children: JSX.Element;
}

interface CourseThemePresentationOverride {
  readonly courseReference: CourseInstanceReference;
  readonly appearance: CourseAppearanceView;
}

/* The legacy scoped presentation was written when its wrapper enclosed only
 * page content. Theme variables now sit above the persistent Ribbon, so this
 * composition rule keeps the course surface on content alone. */
const COURSE_THEME_VARIABLE_SHELL_STYLES = `
.course-theme-scope {
  min-height: 0;
  margin: 0;
  padding: 0;
  border-radius: 0;
  background: none;
}

.course-theme-scope > .ple-shell-frame > .shell {
  inline-size: 100%;
  margin: 0;
  padding: var(--ple-course-scope-padding, 1rem);
  border-radius: 0;
  background-color: var(--ple-surface);
  background-image: linear-gradient(90deg, var(--ple-theme-secondary), var(--ple-theme-accent));
  background-position: top;
  background-size: 100% var(--ple-course-theme-rail-size, 0.32rem);
  background-repeat: no-repeat;
  color: var(--ple-ink);
}

@media (forced-colors: active) {
  .course-theme-scope > .ple-shell-frame > .shell {
    border: 2px solid CanvasText;
    background: Canvas;
    color: CanvasText;
  }
}
`;

function appearanceFor(data: CourseThemeRouteData | undefined): CourseAppearanceView | undefined {
  if (data === undefined) return undefined;
  if (data.kind === "assessmentAttempt") return { theme: data.context.course.theme, banner: null };
  if (data.kind === "assessmentAttemptHistory")
    return { theme: data.history.course.theme, banner: null };
  return courseRouteView(data).appearance;
}

/**
 * Presents route-scope theme variables above route content without becoming a
 * resolver, loading boundary, authorization boundary, or navigation owner.
 * Its wrapper remains mounted while cached data changes so the Ribbon and its
 * children retain their DOM identity during route transitions.
 */
export function CourseThemeVariables(props: CourseThemeVariablesProps): JSX.Element {
  const routeData = useRouteScopeData();
  const currentCourseReference = createMemo(() => {
    const data = routeData();
    return data === undefined ||
      data.kind === "assessmentAttempt" ||
      data.kind === "assessmentAttemptHistory"
      ? undefined
      : courseRouteView(data).summary.reference;
  });
  const [presentationOverride, setPresentationOverride] =
    createSignal<CourseThemePresentationOverride>();
  const presentAppearance = (appearance: CourseAppearanceView | undefined): void => {
    if (appearance === undefined) {
      setPresentationOverride(undefined);
      return;
    }
    const courseReference = currentCourseReference();
    if (courseReference === undefined) return;
    setPresentationOverride({ courseReference, appearance });
  };
  const appearance = createMemo(() => {
    const override = presentationOverride();
    if (override !== undefined && override.courseReference === currentCourseReference())
      return override.appearance;
    return appearanceFor(routeData());
  });

  createEffect(() => {
    if (presentationOverride()?.courseReference !== currentCourseReference())
      setPresentationOverride(undefined);
  });

  const themeStyle = createMemo(() => {
    const current = appearance();
    return current === undefined ? undefined : courseThemeStyle(courseThemeTokens(current.theme));
  });
  const courseReference = createMemo(() => {
    const data = routeData();
    if (data === undefined) return undefined;
    if (data.kind === "assessmentAttempt") {
      return courseInstanceRouteReference(data.context.course.reference);
    }
    if (data.kind === "assessmentAttemptHistory") {
      return courseInstanceRouteReference(data.history.course.reference);
    }
    return courseInstanceRouteReference(courseRouteView(data).summary.reference);
  });

  return (
    <CourseThemePresentationContext.Provider value={presentAppearance}>
      <style>{COURSE_THEME_SCOPE_STYLES}</style>
      <style>{COURSE_THEME_VARIABLE_SHELL_STYLES}</style>
      <div
        class="course-theme-scope"
        data-course-theme={appearance()?.theme}
        data-course-reference={courseReference()}
        style={themeStyle()}
      >
        {props.children}
      </div>
    </CourseThemePresentationContext.Provider>
  );
}
