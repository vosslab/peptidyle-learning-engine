// course_theme_scope.tsx - one pre-render loader and CSS-variable owner per course route.

import { createAsync } from "@solidjs/router";
import { createMemo, createSignal, Show, type JSX } from "solid-js";

import { useApplicationApi } from "../../api/application_api";
import type { CourseRouteView } from "../../api/contracts";
import {
  CourseManagementFrame,
  courseManagementSectionForRoute,
} from "../../components/course_management_frame";
import {
  CourseThemeRouteContext,
  CourseThemePresentationContext,
  courseRouteView,
  type CourseThemeRouteData,
} from "./course_theme_context";
import { courseThemeRouteRequest, type CourseThemeRouteRequest } from "./course_theme_route";
import { COURSE_THEME_SCOPE_STYLES } from "./course_theme_scope_styles";
import {
  courseThemeStyle,
  courseThemeTokens,
  type CourseThemeTokens,
} from "./course_theme_registry";
import { resolveAssignmentAttemptRoute, resolveCourseRoute } from "../../navigation/resolved_route";
import { courseInstanceRouteReference } from "../../navigation/public_route";
import type { CourseId } from "../../../generated/api/CourseId";
import type { AssignmentAttemptId } from "../../../generated/api/AssignmentAttemptId";
import { routeContractForPathname, type RouteContract } from "../../route_contract";

export interface CourseThemeScopeProps {
  readonly pathname: string;
  readonly children: JSX.Element;
}

type ResolvedThemeReference =
  | { readonly kind: "course"; readonly courseId: CourseId }
  | { readonly kind: "assignmentAttempt"; readonly assignmentAttemptId: AssignmentAttemptId }
  | {
      readonly kind: "assignmentAttemptSummary";
      readonly assignmentAttemptId: AssignmentAttemptId;
    };

type ScopedThemeRequest = Exclude<CourseThemeRouteRequest, { readonly kind: "global" }>;

interface ResolvedCourseThemeScopeProps {
  readonly request: ScopedThemeRequest;
  readonly pathname: string;
  readonly children: JSX.Element;
}

function ResolvedCourseThemeScope(props: ResolvedCourseThemeScopeProps): JSX.Element {
  const runtime = useApplicationApi();

  const resolvedReference = createAsync<ResolvedThemeReference>(async () => {
    switch (props.request.kind) {
      case "course":
        return {
          kind: "course",
          courseId: await resolveCourseRoute(runtime.client, props.request.courseReference),
        };
      case "assignmentAttempt":
        return {
          kind: "assignmentAttempt",
          assignmentAttemptId: await resolveAssignmentAttemptRoute(
            runtime.client,
            props.request.assignmentAttemptReference,
          ),
        };
      case "assignmentAttemptSummary":
        return {
          kind: "assignmentAttemptSummary",
          assignmentAttemptId: await resolveAssignmentAttemptRoute(
            runtime.client,
            props.request.assignmentAttemptReference,
          ),
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
      case "assignmentAttempt":
        return {
          kind: "assignmentAttempt",
          screen: await runtime.queries.assignmentAttemptScreen(resolved.assignmentAttemptId),
        };
      case "assignmentAttemptSummary":
        return {
          kind: "assignmentAttemptSummary",
          response: await runtime.queries.assignmentAttemptSummary(resolved.assignmentAttemptId),
        };
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
        const course = (): CourseRouteView => courseRouteView(loaded());
        const managementRoute = (): RouteContract | undefined => {
          if (course().summary.role !== "instructor") return undefined;
          const route = routeContractForPathname(props.pathname);
          if (route === undefined || courseManagementSectionForRoute(route.id) === undefined) {
            return undefined;
          }
          return route;
        };
        const [savedAppearance, setSavedAppearance] = createSignal(course().appearance);
        const tokens = (): CourseThemeTokens => courseThemeTokens(savedAppearance().theme);
        return (
          <CourseThemeRouteContext.Provider value={loaded()}>
            <CourseThemePresentationContext.Provider value={setSavedAppearance}>
              <style>{COURSE_THEME_SCOPE_STYLES}</style>
              <div
                class="course-theme-scope"
                data-course-theme={savedAppearance().theme}
                data-course-reference={courseInstanceRouteReference(course().summary.reference)}
                style={courseThemeStyle(tokens())}
              >
                <Show when={managementRoute()} keyed fallback={props.children}>
                  {(route) => (
                    <CourseManagementFrame course={course().summary} routeId={route.id}>
                      {props.children}
                    </CourseManagementFrame>
                  )}
                </Show>
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
        <ResolvedCourseThemeScope request={request} pathname={props.pathname}>
          {props.children}
        </ResolvedCourseThemeScope>
      )}
    </Show>
  );
}
