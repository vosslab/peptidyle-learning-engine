// course_theme_variables.tsx - shell-stable course-theme variable presentation.

import { createEffect, createMemo, createSignal, type JSX } from "solid-js";

import type { CourseAppearanceView } from "../../../generated/api/CourseAppearanceView";
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

.course-theme-scope > .shell,
.course-theme-scope > .ple-ribbon-shell-grid > .shell {
  min-height: var(--ple-course-scope-min-block-size, calc(100vh - 5rem));
  margin: var(--ple-course-scope-edge-offset, -0.25rem);
  padding: var(--ple-course-scope-padding, 1rem);
  border-radius: var(--ple-radius-surface, 0.9rem);
  background-color: var(--ple-theme-canvas);
  background-image:
    radial-gradient(
      circle at 8% 0%,
      color-mix(
        in srgb,
        var(--ple-theme-secondary) var(--ple-course-theme-secondary-wash, 18%),
        transparent
      ),
      transparent 26rem
    ),
    radial-gradient(
      circle at 92% 0%,
      color-mix(
        in srgb,
        var(--ple-theme-accent) var(--ple-course-theme-accent-wash, 16%),
        transparent
      ),
      transparent 24rem
    ),
    linear-gradient(90deg, var(--ple-theme-secondary), var(--ple-theme-accent)),
    linear-gradient(
      180deg,
      transparent var(--ple-course-theme-fade-start, 18rem),
      var(--ple-surface) 100%
    );
  background-position: center, center, top, center;
  background-size: auto, auto, 100% var(--ple-course-theme-rail-size, 0.32rem), 100% 100%;
  background-repeat: no-repeat;
  color: var(--ple-ink);
}

@media (forced-colors: active) {
  .course-theme-scope > .shell,
  .course-theme-scope > .ple-ribbon-shell-grid > .shell {
    border: 2px solid CanvasText;
    background: Canvas;
    color: CanvasText;
  }
}
`;

function appearanceFor(data: CourseThemeRouteData | undefined): CourseAppearanceView | undefined {
  return data === undefined ? undefined : courseRouteView(data).appearance;
}

/**
 * Presents route-scope theme variables above route content without becoming a
 * resolver, loading boundary, authorization boundary, or navigation owner.
 * Its wrapper remains mounted while cached data changes so the Ribbon and its
 * children retain their DOM identity during route transitions.
 */
export function CourseThemeVariables(props: CourseThemeVariablesProps): JSX.Element {
  const routeData = useRouteScopeData();
  const [presentedAppearance, setPresentedAppearance] = createSignal<CourseAppearanceView>();
  const appearance = createMemo(() => presentedAppearance() ?? appearanceFor(routeData()));

  createEffect(() => {
    setPresentedAppearance(appearanceFor(routeData()));
  });

  const themeStyle = createMemo(() => {
    const current = appearance();
    return current === undefined ? undefined : courseThemeStyle(courseThemeTokens(current.theme));
  });
  const courseReference = createMemo(() => {
    const data = routeData();
    return data === undefined
      ? undefined
      : courseInstanceRouteReference(courseRouteView(data).summary.reference);
  });

  return (
    <CourseThemePresentationContext.Provider value={setPresentedAppearance}>
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
