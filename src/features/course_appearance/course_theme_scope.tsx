// course_theme_scope.tsx - one pre-render loader and CSS-variable owner per course route.

import { createAsync } from "@solidjs/router";
import { createMemo, createSignal, Show, type JSX } from "solid-js";

import { useApiRuntime } from "../../api/runtime";
import type { CourseRouteData } from "../../api/contracts";
import {
  CourseThemeRouteContext,
  CourseThemePresentationContext,
  courseRouteData,
  type CourseThemeRouteData,
} from "./course_theme_context";
import { courseThemeRouteRequest, type CourseThemeRouteRequest } from "./course_theme_route";
import { COURSE_THEME_SCOPE_STYLES } from "./course_theme_scope_styles";
import { courseThemeStyle, courseThemeTokens, type CourseThemeTokens } from "./theme_catalog";
import { resolveCourseRoute, resolveRunRoute } from "../../navigation/resolved_route";
import { courseRouteReference } from "../../navigation/public_route";
import type { CourseId } from "../../../generated/api/CourseId";
import type { RunId } from "../../../generated/api/RunId";

export interface CourseThemeScopeProps {
  readonly pathname: string;
  readonly children: JSX.Element;
}

type ResolvedThemeReference =
  | { readonly kind: "course"; readonly courseId: CourseId }
  | { readonly kind: "runAttempt"; readonly runId: RunId }
  | { readonly kind: "runSummary"; readonly runId: RunId };

type ScopedThemeRequest = Exclude<CourseThemeRouteRequest, { readonly kind: "global" }>;

interface ResolvedCourseThemeScopeProps {
  readonly request: ScopedThemeRequest;
  readonly children: JSX.Element;
}

function ResolvedCourseThemeScope(props: ResolvedCourseThemeScopeProps): JSX.Element {
  const runtime = useApiRuntime();

  const resolvedReference = createAsync<ResolvedThemeReference>(async () => {
    switch (props.request.kind) {
      case "course":
        return {
          kind: "course",
          courseId: await resolveCourseRoute(runtime.client, props.request.courseReference),
        };
      case "runAttempt":
        return {
          kind: "runAttempt",
          runId: await resolveRunRoute(runtime.client, props.request.runReference),
        };
      case "runSummary":
        return {
          kind: "runSummary",
          runId: await resolveRunRoute(runtime.client, props.request.runReference),
        };
    }
  });

  /* Keep the router query call synchronous with this reactive owner. Awaiting
   * route resolution inside the same callback would hide the query dependency,
   * so an appearance revalidation could leave the surrounding palette stale. */
  const routeData = createAsync<CourseThemeRouteData | undefined>(async () => {
    const resolved = resolvedReference();
    if (resolved === undefined) return undefined;
    switch (resolved.kind) {
      case "course":
        return { kind: "course", course: await runtime.queries.courseScope(resolved.courseId) };
      case "runAttempt":
        return { kind: "runAttempt", screen: await runtime.queries.runScreen(resolved.runId) };
      case "runSummary":
        return { kind: "runSummary", response: await runtime.queries.runSummary(resolved.runId) };
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
        const [savedAppearance, setSavedAppearance] = createSignal(course().appearance);
        const tokens = (): CourseThemeTokens => courseThemeTokens(savedAppearance().theme);
        return (
          <CourseThemeRouteContext.Provider value={loaded()}>
            <CourseThemePresentationContext.Provider value={setSavedAppearance}>
              <style>{COURSE_THEME_SCOPE_STYLES}</style>
              <div
                class="course-theme-scope"
                data-course-theme={savedAppearance().theme}
                data-course-reference={courseRouteReference(course().summary.reference)}
                style={courseThemeStyle(tokens())}
              >
                {props.children}
              </div>
            </CourseThemePresentationContext.Provider>
          </CourseThemeRouteContext.Provider>
        );
      }}
    </Show>
  );
}

/** Owns course variables only after the central route boundary grants access. */
export function CourseThemeScope(props: CourseThemeScopeProps): JSX.Element {
  const scopedRequest = createMemo((): ScopedThemeRequest | undefined => {
    const request = courseThemeRouteRequest(props.pathname);
    return request.kind === "global" ? undefined : request;
  });

  return (
    <Show when={scopedRequest()} keyed fallback={<>{props.children}</>}>
      {(request) => (
        <ResolvedCourseThemeScope request={request}>{props.children}</ResolvedCourseThemeScope>
      )}
    </Show>
  );
}
