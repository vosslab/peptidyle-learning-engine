// course_theme_scope.tsx - one pre-render loader and CSS-variable owner per course route.

import { createAsync } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import { useApiRuntime } from "../../api/runtime";
import type { CourseRouteData } from "../../api/contracts";
import { useSessionBootstrap } from "../../auth/session_context";
import {
  CourseThemeRouteContext,
  courseRouteData,
  type CourseThemeRouteData,
} from "./course_theme_context";
import { courseThemeRouteRequest } from "./course_theme_route";
import { COURSE_THEME_SCOPE_STYLES } from "./course_theme_scope_styles";
import { courseThemeStyle, courseThemeTokens, type CourseThemeTokens } from "./theme_catalog";

export interface CourseThemeScopeProps {
  readonly pathname: string;
  readonly children: JSX.Element;
}

/** Owns all course variables below the persistent global shell and nowhere else. */
export function CourseThemeScope(props: CourseThemeScopeProps): JSX.Element {
  const runtime = useApiRuntime();
  const request = courseThemeRouteRequest(props.pathname);
  if (request.kind === "global") return <>{props.children}</>;
  const session = useSessionBootstrap().state();
  if (
    props.pathname.startsWith("/instructor/") &&
    (session.kind !== "authenticated" ||
      !session.session.user.roles.some((role) =>
        ["instructor", "publisher", "administrator"].includes(role),
      ))
  ) {
    return <>{props.children}</>;
  }

  const routeData = createAsync(async (): Promise<CourseThemeRouteData> => {
    switch (request.kind) {
      case "course":
        return { kind: "course", course: await runtime.queries.courseScope(request.courseId) };
      case "runAttempt":
        return { kind: "runAttempt", screen: await runtime.queries.runScreen(request.runId) };
      case "runSummary":
        return { kind: "runSummary", response: await runtime.queries.runSummary(request.runId) };
    }
  });

  return (
    <Show
      when={routeData()}
      fallback={
        <section class="page loading-state" data-course-theme-state="loading" aria-live="polite">
          Loading this course's appearance...
        </section>
      }
    >
      {(loaded) => {
        const course = (): CourseRouteData => courseRouteData(loaded());
        const tokens = (): CourseThemeTokens => courseThemeTokens(course().appearance.theme);
        return (
          <CourseThemeRouteContext.Provider value={loaded()}>
            <style>{COURSE_THEME_SCOPE_STYLES}</style>
            <div
              class="course-theme-scope"
              data-course-theme={course().appearance.theme}
              data-course-id={course().summary.id}
              style={courseThemeStyle(tokens())}
            >
              {props.children}
            </div>
          </CourseThemeRouteContext.Provider>
        );
      }}
    </Show>
  );
}
